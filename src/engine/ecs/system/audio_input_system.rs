use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Consumer, Producer};

use crate::engine::ecs::component::{
    AmplitudeSample, AmplitudeStatus, AudioInputComponent, AudioInputDeviceSelector,
};
use crate::engine::ecs::system::amplitude_system::{AmplitudeSnapshot, InputAmplitudeConsumer};
use crate::engine::ecs::{ComponentId, World};

use super::AmplitudeSystem;

const SNAPSHOT_QUEUE_CAPACITY: usize = 512;
const RETRY_DELAY: Duration = Duration::from_secs(2);
const NO_DATA_TIMEOUT: Duration = Duration::from_secs(3);
const DIAGNOSTIC_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq)]
struct CaptureSignature {
    device: AudioInputDeviceSelector,
    consumers: Vec<InputAmplitudeConsumer>,
}

struct CaptureRuntime {
    signature: CaptureSignature,
    _stream: cpal::Stream,
    snapshots: Consumer<AmplitudeSnapshot>,
    failed: Arc<AtomicBool>,
    dropped: Arc<AtomicU64>,
    reported_dropped: u64,
    last_snapshot: Instant,
}

pub struct AudioInputSystem {
    runtimes: HashMap<ComponentId, CaptureRuntime>,
    retry_after: HashMap<ComponentId, Instant>,
    last_diagnostic: HashMap<ComponentId, (Instant, AmplitudeStatus)>,
}

impl std::fmt::Debug for AudioInputSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioInputSystem")
            .field("active_sources", &self.runtimes.len())
            .field("failed_sources", &self.retry_after.len())
            .finish()
    }
}

impl Default for AudioInputSystem {
    fn default() -> Self {
        Self {
            runtimes: HashMap::new(),
            retry_after: HashMap::new(),
            last_diagnostic: HashMap::new(),
        }
    }
}

impl AudioInputSystem {
    pub fn tick(&mut self, world: &mut World, amplitude: &mut AmplitudeSystem) {
        amplitude.refresh_consumers(world);
        let mut desired: HashMap<ComponentId, Vec<InputAmplitudeConsumer>> = HashMap::new();
        for consumer in amplitude.input_consumers(world) {
            desired.entry(consumer.source).or_default().push(consumer);
        }

        self.runtimes.retain(|source, _| desired.contains_key(source));
        self.retry_after.retain(|source, _| desired.contains_key(source));
        self.last_diagnostic.retain(|observer, _| {
            desired.values().flatten().any(|consumer| consumer.observer == *observer)
        });

        for (&source, consumers) in &desired {
            let Some(input) = world.get_component_by_id_as::<AudioInputComponent>(source) else {
                continue;
            };
            let signature = CaptureSignature {
                device: input.device.clone(),
                consumers: consumers.clone(),
            };
            let needs_rebuild = self.runtimes.get(&source)
                .is_none_or(|runtime| runtime.signature != signature);
            if !needs_rebuild {
                continue;
            }
            self.runtimes.remove(&source);
            if self.retry_after.get(&source).is_some_and(|until| *until > Instant::now()) {
                continue;
            }
            match start_capture(source, signature) {
                Ok(runtime) => {
                    self.retry_after.remove(&source);
                    self.runtimes.insert(source, runtime);
                }
                Err(error) => {
                    eprintln!("[AudioInput] source={source:?} capture start failed: {error}");
                    self.retry_after.insert(source, Instant::now() + RETRY_DELAY);
                    amplitude.invalidate_source(world, source);
                }
            }
        }

        let now = Instant::now();
        let failed: Vec<_> = self.runtimes.iter()
            .filter_map(|(&source, runtime)| {
                (runtime.failed.load(Ordering::Acquire)
                    || (now.duration_since(runtime.last_snapshot) >= NO_DATA_TIMEOUT
                        && runtime.snapshots.is_empty()))
                    .then_some(source)
            })
            .collect();
        for source in failed {
            eprintln!("[AudioInput] source={source:?} capture stream failed; retrying");
            self.runtimes.remove(&source);
            self.retry_after.insert(source, Instant::now() + RETRY_DELAY);
            amplitude.invalidate_source(world, source);
        }

        for runtime in self.runtimes.values_mut() {
            while let Ok(snapshot) = runtime.snapshots.pop() {
                runtime.last_snapshot = Instant::now();
                let diagnostic = self.last_diagnostic.entry(snapshot.observer)
                    .or_insert((Instant::now() - DIAGNOSTIC_INTERVAL, AmplitudeStatus::Pending));
                if diagnostic.1 != snapshot.sample.status
                    || diagnostic.0.elapsed() >= DIAGNOSTIC_INTERVAL
                {
                    eprintln!(
                        "[Amplitude] observer={:?} source={:?} status={:?} rms={:.6} peak={:.6} frames={} dropped={}",
                        snapshot.observer,
                        snapshot.source,
                        snapshot.sample.status,
                        snapshot.sample.rms,
                        snapshot.sample.peak,
                        snapshot.sample.valid_frames,
                        runtime.dropped.load(Ordering::Relaxed),
                    );
                    *diagnostic = (Instant::now(), snapshot.sample.status);
                }
                amplitude.submit_snapshot(snapshot);
            }
            let dropped = runtime.dropped.load(Ordering::Relaxed);
            let new_drops = dropped.wrapping_sub(runtime.reported_dropped);
            if new_drops != 0 {
                amplitude.record_dropped_snapshots(new_drops);
                runtime.reported_dropped = dropped;
            }
        }
        amplitude.drain_pending(world);
    }
}

fn selected_device(
    selector: &AudioInputDeviceSelector,
) -> Result<(cpal::Device, String), String> {
    let host = cpal::default_host();
    let device = match selector {
        AudioInputDeviceSelector::Default => host.default_input_device()
            .ok_or_else(|| "no default input device is available".to_string())?,
        AudioInputDeviceSelector::DeviceNumber(index) => host.input_devices()
            .map_err(|error| format!("cannot enumerate input devices: {error}"))?
            .nth(*index)
            .ok_or_else(|| format!("input device number {index} is not available"))?,
    };
    let name = device.name().unwrap_or_else(|_| "<unnamed input>".into());
    Ok((device, name))
}

fn start_capture(source: ComponentId, signature: CaptureSignature) -> Result<CaptureRuntime, String> {
    let (device, device_name) = selected_device(&signature.device)?;
    let supported = device.default_input_config()
        .map_err(|error| format!("cannot query default input format: {error}"))?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let accumulators = signature.consumers.iter().map(|consumer| {
        RollingRms::new(*consumer, sample_rate)
    }).collect::<Vec<_>>();
    let windows = signature.consumers.iter()
        .map(|consumer| format!("{:.3}s", consumer.window_sec))
        .collect::<Vec<_>>().join(",");
    let (producer, snapshots) = rtrb::RingBuffer::new(SNAPSHOT_QUEUE_CAPACITY);
    let failed = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicU64::new(0));
    let error_flag = failed.clone();
    let error_callback = move |_error| {
        error_flag.store(true, Ordering::Release);
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let mut callback = CaptureCallback::new(source, channels, sample_rate, accumulators, producer, dropped.clone());
            device.build_input_stream(&config, move |data: &[f32], _| callback.process(data, |v| v), error_callback, None)
        }
        cpal::SampleFormat::I16 => {
            let mut callback = CaptureCallback::new(source, channels, sample_rate, accumulators, producer, dropped.clone());
            device.build_input_stream(&config, move |data: &[i16], _| callback.process(data, |v| v as f32 / i16::MAX as f32), error_callback, None)
        }
        cpal::SampleFormat::U16 => {
            let mut callback = CaptureCallback::new(source, channels, sample_rate, accumulators, producer, dropped.clone());
            device.build_input_stream(&config, move |data: &[u16], _| callback.process(data, |v| v as f32 / u16::MAX as f32 * 2.0 - 1.0), error_callback, None)
        }
        other => return Err(format!("unsupported input sample format {other:?}")),
    }.map_err(|error| format!("cannot build input stream: {error}"))?;
    stream.play().map_err(|error| format!("cannot start input stream: {error}"))?;
    eprintln!(
        "[AudioInput] source={source:?} device={device_name:?} format={sample_format:?} sample_rate={sample_rate} channels={channels} windows=[{windows}]"
    );
    Ok(CaptureRuntime {
        signature,
        _stream: stream,
        snapshots,
        failed,
        dropped,
        reported_dropped: 0,
        last_snapshot: Instant::now(),
    })
}

struct CaptureCallback {
    source: ComponentId,
    channels: usize,
    sample_rate: u32,
    frame_count: u64,
    accumulators: Vec<RollingRms>,
    snapshots: Producer<AmplitudeSnapshot>,
    dropped: Arc<AtomicU64>,
}

impl CaptureCallback {
    fn new(
        source: ComponentId,
        channels: usize,
        sample_rate: u32,
        accumulators: Vec<RollingRms>,
        snapshots: Producer<AmplitudeSnapshot>,
        dropped: Arc<AtomicU64>,
    ) -> Self {
        Self { source, channels: channels.max(1), sample_rate, frame_count: 0, accumulators, snapshots, dropped }
    }

    fn process<T: Copy>(&mut self, data: &[T], convert: impl Fn(T) -> f32) {
        let before = self.frame_count;
        for frame in data.chunks_exact(self.channels) {
            let mut sum_squares = 0.0;
            let mut peak = 0.0_f32;
            for &sample in frame {
                let value = convert(sample);
                let value = if value.is_finite() { value.clamp(-1.0, 1.0) } else { 0.0 };
                sum_squares += value * value;
                peak = peak.max(value.abs());
            }
            let mean_square = sum_squares / self.channels as f32;
            for accumulator in &mut self.accumulators {
                accumulator.push(mean_square, peak);
            }
            self.frame_count = self.frame_count.wrapping_add(1);
        }
        let valid_frames = self.frame_count.wrapping_sub(before) as u32;
        if valid_frames == 0 {
            return;
        }
        let timestamp_sec = self.frame_count as f64 / self.sample_rate.max(1) as f64;
        for accumulator in &mut self.accumulators {
            let snapshot = accumulator.snapshot(self.source, timestamp_sec, valid_frames);
            if self.snapshots.push(snapshot).is_err() {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

struct RollingRms {
    consumer: InputAmplitudeConsumer,
    squares: Vec<f32>,
    peaks: Vec<f32>,
    cursor: usize,
    filled: usize,
    sum_squares: f64,
    sequence: u64,
}

impl RollingRms {
    fn new(consumer: InputAmplitudeConsumer, sample_rate: u32) -> Self {
        let frames = (consumer.window_sec as f64 * sample_rate as f64)
            .round().clamp(1.0, usize::MAX as f64) as usize;
        Self {
            consumer,
            squares: vec![0.0; frames],
            peaks: vec![0.0; frames],
            cursor: 0,
            filled: 0,
            sum_squares: 0.0,
            sequence: 0,
        }
    }

    fn push(&mut self, square: f32, peak: f32) {
        if self.filled == self.squares.len() {
            self.sum_squares -= self.squares[self.cursor] as f64;
        } else {
            self.filled += 1;
        }
        self.squares[self.cursor] = square;
        self.peaks[self.cursor] = peak;
        self.sum_squares += square as f64;
        self.cursor = (self.cursor + 1) % self.squares.len();
    }

    fn snapshot(&mut self, source: ComponentId, timestamp_sec: f64, valid_frames: u32) -> AmplitudeSnapshot {
        self.sequence = self.sequence.wrapping_add(1);
        let rms = (self.sum_squares.max(0.0) / self.filled.max(1) as f64).sqrt() as f32;
        let peak = self.peaks[..self.filled].iter().copied().fold(0.0_f32, f32::max);
        let status = if rms <= f32::EPSILON { AmplitudeStatus::Neutral } else { AmplitudeStatus::Live };
        AmplitudeSnapshot {
            observer: self.consumer.observer,
            source,
            sample: AmplitudeSample {
                generation: self.consumer.generation,
                sequence: self.sequence,
                timestamp_sec,
                valid_frames,
                rms,
                peak,
                status,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consumer(window_sec: f32) -> InputAmplitudeConsumer {
        InputAmplitudeConsumer {
            observer: ComponentId::default(),
            source: ComponentId::default(),
            generation: 3,
            window_sec,
        }
    }

    #[test]
    fn rolling_rms_expires_old_frames_and_reports_peak() {
        let mut rms = RollingRms::new(consumer(0.5), 4);
        rms.push(0.25, 0.5);
        rms.push(1.0, 1.0);
        let first = rms.snapshot(ComponentId::default(), 0.5, 2).sample;
        assert!((first.rms - (0.625_f32).sqrt()).abs() < 1e-6);
        assert_eq!(first.peak, 1.0);
        rms.push(0.0, 0.0);
        let rolled = rms.snapshot(ComponentId::default(), 0.75, 1).sample;
        assert!((rolled.rms - (0.5_f32).sqrt()).abs() < 1e-6);
        assert_eq!(rolled.peak, 1.0);
    }

    #[test]
    fn exact_silence_is_neutral() {
        let mut rms = RollingRms::new(consumer(0.25), 4);
        rms.push(0.0, 0.0);
        assert_eq!(
            rms.snapshot(ComponentId::default(), 0.25, 1).sample.status,
            AmplitudeStatus::Neutral
        );
    }

    #[test]
    fn callback_converts_stereo_frames_and_queue_overflow_never_blocks() {
        let source = ComponentId::default();
        let accumulator = RollingRms::new(consumer(1.0), 2);
        let (producer, mut snapshots) = rtrb::RingBuffer::new(1);
        let dropped = Arc::new(AtomicU64::new(0));
        let mut callback = CaptureCallback::new(
            source,
            2,
            2,
            vec![accumulator],
            producer,
            dropped.clone(),
        );
        callback.process(&[1.0_f32, -1.0, 0.5, 0.5], |value| value);
        let sample = snapshots.pop().unwrap().sample;
        assert!((sample.rms - 0.625_f32.sqrt()).abs() < 1e-6);
        assert_eq!(sample.peak, 1.0);

        callback.process(&[0.25_f32, 0.25], |value| value);
        callback.process(&[0.25_f32, 0.25], |value| value);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
    }
}

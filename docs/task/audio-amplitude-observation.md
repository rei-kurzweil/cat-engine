# Audio amplitude observation: first slice

Date: 2026-09-01
Status: ready for implementation planning

## Goal

Expose a retained, main-thread-readable amplitude measurement for any live
`AudioSource`, starting with `AudioInput` and `AudioClip`. This is the first
observable capture path and remains useful when speech/phoneme detection is
unavailable, misconfigured, or intentionally not used.

`Amplitude` is an observer, not a gain/effect: it does not alter samples or
the compiled audio graph's audible result.

## Proposed MMS surface

```mms
let microphone = AudioInput {}
let amplitude = Amplitude.rolling_window(0.250).from(microphone)
```

`Amplitude.from(source)` returns a live host-owned amplitude component handle,
not an immediate `f32`. Its later live read surface is intentionally deferred:

```mms
// v2 spelling, after live component reads are available
let average: f32 = amplitude.value()
```

An immediate `let amplitude: f32 = ...` cannot track a changing microphone
without a reactive value model. The first slice instead uses the retained
component state for diagnostics and Rust systems. In particular,
`AvatarControlSystem` can inspect an enabled direct-child `AmplitudeComponent`
and read its latest retained RMS without MMS exposing the float yet.

## Semantics

- `.rolling_window(seconds)` chooses the window duration in seconds; default
  and allowed bounds must be selected and documented during implementation.
- `value` is RMS: `sqrt(mean(sample * sample))` over valid samples in the
  current window, as linear PCM amplitude in the range normally 0–1.
- `peak` may be retained alongside RMS for diagnostics, but RMS is the public
  first-slice meaning of `Amplitude`.
- `timestamp`, `sequence`, window sample count, and discontinuity status are
  retained with the measurement so stale silence cannot masquerade as a live
  value.
- A disabled, removed, failed, or underrun source publishes neutral/invalid
  state; it must not preserve an old positive amplitude.

## Source and graph ownership

`Amplitude.from(source)` accepts an `AudioSource` (`AudioInput`, `AudioClip`,
or `AudioOscillator`) and rejects other components. It observes the source's
raw emitted PCM before parent graph effects and output mixing.

That source-level contract is intentional for this first slice. A later
explicit graph-node/post-effect meter can reuse the retained metric type but
must not silently change `Amplitude.from(source)` semantics.

```text
AudioInput capture PCM ──┬── Amplitude rolling RMS
                         ├── Visemes analysis (later)
                         └── graph monitoring/output (optional)
```

## Component lifecycle and activation

`AmplitudeComponent` stores an explicit `AudioSource` component reference,
its window configuration, enabled state, generation, and main-thread-retained
measurement/status. It never owns a CPAL stream, callback buffer, or mutable
audio-thread state.

Creating/initializing an enabled live component such as:

```mms
let microphone = AudioInput {}
let amplitude = Amplitude.rolling_window(0.250).from(microphone) {}
```

registers one amplitude consumer for `microphone` even if `amplitude` is not
parented under an `AVC` or an `AudioOutput`. This mirrors `Visemes`: reference
selects the source; topology selects a later consumer.

The audio-input runtime maintains a consumer record keyed by `(amplitude id,
generation)`. It starts/keeps capture active while at least one enabled graph,
amplitude, or viseme consumer exists, and removes the unit on disable, source
reconfiguration, source removal, or component removal. A re-enable/recreate
uses a new generation, so old queued snapshots cannot update it.

Directly placing `amplitude` beneath `AVC` makes its retained measurement
eligible for `AvatarControlSystem` to consume. That system reads the retained
main-thread value only; it never queries a worker or callback. The exact
amplitude-to-mouth policy is deliberately a separate adapter decision—plain
`Amplitude` observes and does not itself alter morph targets.

## Runtime design

- `AmplitudeComponent` owns only authored source reference/window intent and
  retained main-thread result/status.
- `AudioInputSystem` / source runtime preallocates one rolling RMS unit per
  active `(AmplitudeComponent, generation)` consumer and publishes bounded
  snapshots. A unit exists only while its enabled live component needs it.
- The capture/RT callback performs fixed arithmetic and bounded enqueue only:
  no allocation, ECS access, logging, locks, or blocking send.
- Main thread drains snapshots once per tick, validates source and generation,
  and retains the newest measurement.
- Console diagnostics are main-thread-only: on start log device/format and
  calibrated RMS noise floor; later log only meaningful threshold crossings or
  state changes, not every audio block.

## Test path

1. Initialize `Amplitude.rolling_window(0.250).from(microphone)` against a
   detached `AudioInput`; capture becomes active but remains inaudible.
2. Inject deterministic PCM through the fake endpoint and prove RMS/peak,
   rolling-window expiry, queue-full discontinuity, enable/disable, and
   generation-safe teardown.
3. Run the VTuber mirror scene and confirm startup noise-floor and threshold
   logs before enabling any viseme backend.
4. Place the instance directly beneath `AVC` and prove `AvatarControlSystem`
   sees its newest retained RMS without any MMS scalar read.
5. Reuse this path as a visible mouth-open fallback only if explicitly authored
   by a future `AmplitudeToMorph` adapter; `Amplitude` itself never drives
   avatar morphs.

## Non-goals

- LUFS/integrated loudness or normalization;
- per-frame MMS scalar reads before live receiver reads exist;
- automatic mouth animation;
- changing an audio signal's gain or effects;
- post-effect or output-bus metering in the first slice.

## Exit

`Amplitude` can observe every first-slice source with bounded lifecycle-safe
runtime behavior and trusted diagnostic output. Only then use it to validate
real microphone capture before evaluating viseme recognition.

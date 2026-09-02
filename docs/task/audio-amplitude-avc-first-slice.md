# Audio amplitude to AVC: first slice

Date: 2026-09-01
Status: in progress

## Implementation progress — 2026-09-01

The authoring and retained-state seam is implemented:

- `AmplitudeComponent` owns the authored source reference, rolling-window
  request, enabled state, generation, retained main-thread sample, and status.
- MMS accepts `Amplitude.rolling_window(seconds).from(audio_source)`, preserves
  that authored shape through scene round-trip, and rejects a resolved
  non-audio source.
- The MMS parser now accepts the reserved import word `from` as a method name
  after a component dot-chain.

Still pending: consumer lifecycle/control messages, bounded RMS production,
main-thread snapshot draining, AVC direct-child pickup, and live diagnostics.

## Goal

Ship the smallest end-to-end observable microphone path:

1. MMS creates `Amplitude.rolling_window(seconds).from(audio_source)`.
2. The live `AmplitudeComponent` explicitly references one `AudioSource`.
3. An enabled instance provisions a preallocated rolling-RMS unit only while
   it has a live source consumer relationship.
4. The worker/capture path returns bounded timestamped RMS snapshots to the
   main thread.
5. An enabled direct `Amplitude` child of `AVC` is visible to
   `AvatarControlSystem` through retained main-thread state.
6. Startup/noise-floor/threshold diagnostics prove that real microphone data is
   flowing, while silence, disable, failure, and removal clear stale state.

This slice does **not** animate mouth morphs. It proves the semantic input
path an explicit later `AmplitudeToMorph` adapter or viseme backend will use.

## Authoring target

```mms
let microphone = AudioInput {}
let amplitude = Amplitude.rolling_window(0.250).from(microphone) {}

AVC {
    amplitude

    T {
        GLTF.new("assets/models/bisket.glb") {
            MorphTargetMap.new()
                .slot("viseme_aa", "Fcl_MTH_A")
                .slot("viseme_ih", "Fcl_MTH_I")
                .slot("viseme_ou", "Fcl_MTH_U")
                .slot("viseme_e", "Fcl_MTH_E")
                .slot("viseme_oh", "Fcl_MTH_O")
        }
    }
}
```

`microphone` remains detached from `AudioOutput`: measurement activates
capture but does not monitor it. `amplitude` is a live component handle, not
an immediate scalar. MMS receiver reads such as `amplitude.value()` are
deferred; Rust systems can read the retained snapshot in this slice.

## Locked decisions

- `Amplitude` is an observer, never gain/effect processing.
- `Amplitude.from(source)` accepts only an `AudioSource`; first support covers
  `AudioInput`, `AudioClip`, and `AudioOscillator` as their runtime paths land.
- `rolling_window(seconds)` measures source-output RMS in linear PCM amplitude,
  `sqrt(mean(sample²))`, over the requested window.
- `AmplitudeComponent` owns source reference, requested window, enabled state,
  generation, retained sample, and observable status—never a CPAL stream,
  queue, or worker-owned accumulator.
- An enabled initialized `Amplitude` instance is an audio-source consumer even
  when it has no parent. Direct `AVC` placement controls avatar eligibility,
  not capture activation.
- `AvatarControlSystem` consumes only enabled direct-child amplitude instances
  and only their validated retained main-thread snapshots.
- Callback/RT code never allocates, logs, locks, waits, accesses ECS, or sends
  through a blocking channel.
- Raw source measurement is the first-slice contract. Post-effect/output-bus
  metering is deferred to [audio node metering](audio-node-metering.md).
- This slice does not infer phonemes/visemes or apply morph drivers.

## Runtime path

```text
AmplitudeComponent { source, window, generation }
                  │ register / update / remove
                  ▼
AudioInputSystem source runtime
                  │ preallocated rolling RMS unit
                  ▼
capture / source PCM callback ── bounded snapshot queue ──► main AmplitudeSystem
                                                           │ retain newest valid RMS
                                                           ▼
                                                AvatarControlSystem (direct AVC child)
```

## Ordered implementation checklist

### 1. Component and MMS catalog

- [ ] Add `AmplitudeComponent`, source selector/reference, requested window,
  enabled flag, generation, retained `AmplitudeSample`, and status.
- [ ] Register `Amplitude.rolling_window(seconds).from(audio_source)` in the
  MMS component registry and runtime catalog.
- [ ] Reject invalid/non-audio sources and invalid/non-finite/non-positive
  windows with typed authoring errors.
- [ ] Serialize/round-trip source choice and window; omit retained runtime data.

Gate: an MMS amplitude expression materializes one live component bound to its
declared `AudioSource`.

### 2. Consumer lifecycle and preallocation

- [ ] Add `AmplitudeSystem` control messages: register, update, reset, remove,
  and shutdown.
- [ ] Add a per-source consumer registry keyed by amplitude identity and
  generation.
- [ ] Activate source capture/render production only while at least one graph,
  amplitude, or later viseme consumer is enabled.
- [ ] Preallocate one rolling accumulator/window state per supported amplitude
  consumer before capture delivery begins.
- [ ] Teardown on disable, reconfigure, source removal, component removal, and
  final-consumer disappearance without waiting in callback context.

Gate: repeat enable/disable/reconfigure/removal cycles neither leak a consumer
nor allow an old generation to update a new instance.

### 3. RMS protocol

- [ ] Define fixed-size measurement snapshots carrying observer identity,
  generation, sequence, source timestamp, valid-frame count, RMS, peak, and
  discontinuity/status.
- [ ] Calculate rolling RMS with bounded preallocated state; use safe finite
  sample handling and reset temporal state after a sequence gap.
- [ ] Bound the callback-to-main queue and drop rather than block when full.
- [ ] Retain only the newest valid snapshot once per main-thread tick.
- [ ] Clear retained state to neutral/invalid on silence policy, overrun reset,
  source failure, disable, and removal.

Gate: deterministic fake PCM proves RMS/peak math, partial blocks, window
expiration, queue-full recovery, and stale event rejection.

### 4. AVC semantic pickup

- [ ] Extend `AvatarControlSystem` topology walk to find enabled direct
  `Amplitude` children.
- [ ] Expose a private AVC-side resolved amplitude snapshot/result for later
  adapters and diagnostics.
- [ ] Choose newest valid sample deterministically if multiple eligible direct
  components exist; diagnose conflict once.
- [ ] Do not write morph drivers in this slice.

Gate: a synthetic retained sample is visible to AVC only for a direct enabled
child; detached, stale, disabled, and indirect instances are ignored.

### 5. Diagnostics and focused live proof

- [ ] At capture start, log selected input device, negotiated format, requested
  RMS window, and calibrated noise-floor RMS from the main thread.
- [ ] Log only status transitions and configurable/significant crossings above
  the calibrated floor; never log every callback or render tick.
- [ ] Add `Amplitude` to
  `examples/vtuber-eye-tracking-mirror-eye-stabilize.mms` as a direct AVC child
  with monitoring absent.
- [ ] Validate speaking changes retained RMS, silence neutralizes it, and
  disable/re-enable/device loss recovers without a held stale value.

Gate: console output provides enough signal to verify real microphone capture
and noise floor before a viseme backend exists.

## Required automated coverage

- MMS catalog/materialization/round-trip and invalid source/window cases;
- consumer reference-counting and generation-safe teardown;
- RMS and peak fixture values across partial/rolling blocks;
- queue full, sequence gap, source failure, disable, and removal reset;
- no source activation without a graph/amplitude/viseme consumer;
- direct-child AVC eligibility and newest valid retained sample selection;
- no morph-driver writes from amplitude observation alone.

## Live acceptance record

Record hardware, OS/audio host, input device, negotiated format, requested
window, calibrated noise floor, speech RMS range, update rate, queue overrun
exercise, and disable/re-enable/device-loss exercise.

## Explicitly deferred

- MMS `amplitude.value()` live reads;
- amplitude-to-mouth/morph mapping. The approved follow-up direction is an
  explicit AVC builder such as `mouth_open_from_amplitude(amplitude)`, rather
  than behavior inferred merely because AVC has no `Visemes` child;
- phoneme detection and `Visemes` worker;
- post-effect/output-bus meters;
- loudness normalization, LUFS, recording, and audio UI widgets.

## Exit

Stop after the mirror scene proves stable source-bound amplitude observation and
AVC semantic pickup. The next approved slice may add an explicit AVC
amplitude-to-mouth binding or proceed to the selected viseme backend.

## V2 authoring exploration (not part of this slice)

The proposed richer AVC surface—multiple named amplitude/viseme slots or AVC
builders that accept component expressions—remains a V2 design task. The
first approved binding is a single explicit component reference, for example:

```mms
AVC {
    mouth_open_from_amplitude(amplitude)
}
```

That builder belongs to AVC because it owns avatar-specific policy: morph
target choice, scaling, smoothing, thresholding, and precedence with
`Visemes`. `Amplitude` only publishes a retained measurement. V2 must define
multiple-slot semantics, serialization, replacement/removal lifecycle, and
additional target policies.

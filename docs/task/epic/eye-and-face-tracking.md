# Epic: eye and face tracking

## Outcome

Mittens can consume supported eye and face tracking transports and drive an
avatar predictably.  A tracker component owns transport configuration and
calibration; avatar-specific mappings state which rig targets receive the
semantic channels.  The generic VRChat OSC path is the interoperable baseline,
and the HTC transport adds per-eye face data where the sender provides it.

## Current checkpoint

- `XREyeTracking` accepts ALVR/VRChat Eye OSC on `127.0.0.1:9000` by default.
  It retains normalized gaze and a live combined closure sample.
- As a direct child of `AVC`, it drives mapped eye bones from retained gaze.
- The in-progress morph-target path maps generic OSC closure to Bisket's
  `Fcl_EYE_Close_L` and `Fcl_EYE_Close_R` targets in
  `examples/vtuber-eye-tracking-mirror.mms`.  It compiles, but still needs an
  actual headset/ALVR blink verification.
- `XREyeTrackingHTC` receives the proprietary HTC packet on port `9002`,
  including per-eye direction, position, openness, wide, squeeze, and pupil
  diameter.  Apart from gaze, these fields are event-only today.

## Workstreams

1. [Pupil direction, pupil size, and eye openness](../pupil-direction-size-and-openness.md)
   establishes the generic OSC baseline and its calibration surface.
2. Per-eye face channels via the HTC transport, following the generic baseline:
   individual pupil position/direction, openness, wide, squeeze, and pupil
   diameter.
3. Avatar mapping and calibration UX: reusable semantic maps, diagnostics,
   safe defaults, and per-rig presets.

## Boundaries

- Trackers are direct `AVC` children; AVC may consume their retained samples.
- Generic OSC must remain useful without an HTC-specific sender.
- Never silently invent a rig basis or calibration. Defaults must preserve the
  source values, and an absent tracker must release any driven values.
- Transport decoding, calibration, semantic routing, and morph-target/bone
  application remain separately testable.

## Validation

- Run the focused scene with:

  ```text
  cargo run -- load examples/vtuber-eye-tracking-mirror.mms
  ```

- Confirm gaze movement, a fully closed blink, partial closure, reopening, and
  that stopping OSC restores the target's base morph values.
- Exercise a rig without mapped eye/blink targets to ensure it is unchanged.

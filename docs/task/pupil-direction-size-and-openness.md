# Task: pupil direction, pupil size, and eye openness

Parent: [Eye and face tracking epic](epic/eye-and-face-tracking.md)

## Goal

Drive the eye data that avatars commonly expose while making incoming pupil
direction tunable per `XREyeTracking` instance.  Begin with the combined
values compatible with standard VRChat Eye OSC, then add individual-eye data
from the HTC transport.

## Current state

`XREyeTracking` already decodes generic OSC gaze and
`EyesClosedAmount`; the latter is converted to closure (`0 = open`, `1 =
closed`).  The VTuber mirror now has an explicit Bisket blink map and AVC
applies that live closure to both mapped morph targets.  This is an
implementation checkpoint rather than a completed feature: it needs an
on-headset test to verify packet receipt, morph palette upload, and visible
blinking.

Gaze currently consumes normalized direction vectors directly.  The observed
movement is too sensitive, and there is no component-level tuning surface yet.

## Phase 1 — standard VRChat OSC

Deliver combined pupil direction and blink amount through `XREyeTracking`.

- Verify the mirror example receives `/avatar/parameters/EyesClosedAmount`
  (and the supported `/tracking/eye/...` alias) from ALVR, maps it to both
  Bisket blink targets, and releases the override immediately when packets
  stop.
- Add focused decoding and AVC/morph-driver tests, including open, closed,
  partial, invalid, and stale/no-packet cases.
- Add optional per-component pupil-direction calibration.  Start with separate
  pitch/yaw (or X/Y) scale floats, each defaulting to `1.0`; add offsets only
  when needed by a real calibration.  Define the sign and units explicitly
  and apply calibration before direction reconstruction/normalization.
- Expose the settings in MMS and preserve them through serialization.  The
  `on()` defaults must be neutral, so existing scenes do not change.
- Keep gaze eye-bone driving and pupil/blink morph routing independently
  configurable; a rig may support one but not the other.

Acceptance: with a standard VRChat OSC sender, gaze is comfortably scaled,
both eyes blink together from the combined closure value, and removal/loss of
the tracker leaves base bone/morph values intact.

## Phase 2 — HTC individual-eye transport

Use `XREyeTrackingHTC` to support individual-eye values from the proprietary
HTC packet already decoded by the engine.

- Retain and route each eye's pupil direction/position, pupil diameter, and
  openness.  Decide semantic channel names and mapping API before exposing
  them to MMS.
- Add left/right mapping so rigs can use asymmetric blink, pupil, wide, and
  squeeze targets without relying on label-name guesses.
- Apply the same explicit calibration model per eye where required; document
  whether calibration is shared or side-specific.
- Define arbitration when generic and HTC trackers coexist: newest valid value
  per semantic channel wins, with a visible diagnostic/source indicator.
- Add packet fixtures and a hardware validation matrix covering absent field
  flags, unilateral tracking, packet loss, and reconnects.

Acceptance: HTC input visibly and independently drives left and right pupil
direction, pupil size, and openness on a mapped avatar; generic OSC behavior
continues to work unchanged.

## Open decisions

- Is pupil direction expressed as pitch/yaw or a 2D pupil-position vector at
  the public component boundary? Normalize internally only after calibration.
- Should the first offset be angular/normalized units, or should it wait for a
  calibration UI and recorded device data?
- Which neutral/default target values and value ranges should morph mappings
  support beyond the current `[0, 1]` blink closure?

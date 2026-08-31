# Eye-tracking head-motion probe and sample alignment

Status: probe specified / implementation and headset recordings pending

Parent: [Eye and face tracking epic](epic/eye-and-face-tracking.md)

Related symptom: [Eye tracking appears to counteract head rotation](../bugs/eye-tracking-head-rotation-counteracts-gaze.md)

## Outcome

Build a reproducible capture-and-analysis harness that can explain why Vive Focus 3 gaze changes
strongly during HMD motion. Do not choose another compensation formula until recordings distinguish
source-coordinate leakage, sample-time skew, physical tracker disturbance, ordinary human
vestibulo-ocular response, and Mittens' own transform/application path.

Any eventual mitigation is opt-in and source-specific. The default engine path must continue to use
the unmodified tracker signal.

## Current checkpoint

- The custom ALVR client reads `XR_HTC_eye_tracker` and carries left/right gaze, geometric eye
  values, and pupil values to the server.
- ALVR's `Mittens (binary UDP)` sink emits protocol v1 on `127.0.0.1:9002`.
- `XREyeTrackingHTC` decodes that packet and AVC retains and applies left/right gaze independently.
- Live testing on 2026-08-30 tentatively showed packet activity, but the visible pupil response was
  dominated by motion correlated with the HMD. Independent intentional gaze was not yet obvious.
- `rotation_limits(...)` bounds the visible error but does not identify or remove its cause.
- `AVC.head_motion_gaze_policy("freeze")` is an opt-in interim experiment. It suppresses all new
  gaze during rapid head motion, including intentional gaze, so it is not a model of the defect.
- The first `head_rotation_compensation("cancel")` experiment used the inverse absolute head basis.
  That is invalid for a signal that is steady-state head-relative: it can turn neutral `-Z` into
  `+Z` and leave the pupils facing away after the head settles.
- HTC blinking is a separate missing consumer path. Per-eye `openness` reaches
  `XrEyeTrackingHtcUpdated`, but it is not retained as live closure state or mapped by AVC. Failure
  to blink therefore does not prove that HTC UDP gaze is absent.

## Questions the recordings must answer

1. Does the unwanted motion already exist in ALVR's raw HTC gaze quaternion, before UDP encoding
   and before Mittens?
2. Is the gaze head-local, stage/world-relative, or a mixture that changes while the HMD moves?
3. Is gaze delayed or filtered relative to the head pose sampled in the same ALVR tracking poll?
4. Is the disturbance better predicted by head angle, angular velocity, angular acceleration, or
   high-frequency vibration?
5. Is the response common to both eyes, or does the left/right difference contain useful independent
   gaze?
6. Does fit, eye-to-sensor distance, illuminator clearance, or headset tightness materially change
   gain, lag, validity, or noise?
7. How much apparent counter-rotation is real vestibulo-ocular reflex from fixating a world-stable
   target, rather than tracker error?
8. If ALVR's raw signal is stable, at what exact Mittens stage does the unwanted component appear?

## Ranked hypotheses and discriminating evidence

### H1 — source sample-time skew

The eye quaternion is head-relative at its own sample time but is compared or applied with a newer
head basis. Expect a strong gaze/head angular-velocity cross-correlation at a consistent non-zero
lag. Time-shifting or interpolating head pose should reduce neutral-gaze residuals without changing
held-pose output.

### H2 — physical common-mode disturbance

Headset slip, sensor/eye geometry, illumination loss, or vibration perturbs both eyes together.
Expect the binocular common-mode signal to correlate more with angular acceleration or
high-frequency HMD motion than with absolute orientation, and expect the fitted relationship or
validity rate to change with headset fit. A purely software basis correction will not be stable
across fit conditions.

### H3 — coordinate-space leakage

The source quaternion includes some absolute HMD orientation although Mittens treats it as
head-local. Expect a near-instantaneous, approximately linear rotation gain with a repeatable axis
mapping. A true absolute-space leak should also leave a predictable offset at a held rotated pose;
the currently reported recovery after motion makes this less likely than H1 or H2.

### H4 — physiological eye motion

While fixating a world-stable point, normal vestibulo-ocular response makes the eyes counter-rotate
as the head turns. This is expected signal. It should be present for a world-fixed target but much
smaller for a target rigidly attached to the HMD view. The probe must not train a correction on
world-fixation trials and then label that response hardware error.

### H5 — Mittens basis, scheduling, or rig application

ALVR raw gaze is stable, but the decoded gaze, post-limit gaze, effective eye-parent basis, or final
eye-bone rotation gains an HMD-correlated component. Expect the ALVR source trace to remain clean
while one adjacent pair of Mittens trace stages diverges.

## Probe architecture

Use append-only JSON Lines for capture. It is easier to inspect than a custom binary format, keeps
optional fields explicit, and can be converted to CSV/Parquet by the analysis script. Recording is
strictly opt-in and must not share the normal per-frame logging path.

### Probe A — ALVR source trace (first implementation slice)

Record from the server tracking loop before `FaceTrackingSink` serialization, where one
`TrackingData` already contains `poll_timestamp`, the HMD `DeviceMotion`, and `FaceData` from the
same client poll.

Each row must contain:

- capture sequence and trial label;
- ALVR client `poll_timestamp_ns` and server monotonic arrival time;
- HMD orientation quaternion, position, angular velocity, and linear velocity;
- left/right raw HTC gaze quaternion plus validity;
- left/right openness, wide, squeeze, pupil diameter, pupil position, and their validity;
- active sink/protocol version and ALVR build/protocol identifier.

Use a dedicated opt-in path such as `ALVR_EYE_PROBE_PATH`. Opening failure must produce one clear
diagnostic and leave streaming behavior unchanged. Buffer writes and flush at bounded intervals;
do not emit an `info!` line per tracking poll.

This trace is sufficient to test H1–H4 and avoids changing the UDP contract before the source
behavior is understood.

### Probe B — Mittens pipeline trace (second slice)

Add only if Probe A shows a stable source or if the ALVR-to-Mittens boundary remains ambiguous.
Record:

- local monotonic receive and engine-frame timestamps;
- packet version, validity flags, and decoded left/right head-local direction;
- current effective HMD/avatar-head and each eye-parent world quaternion;
- configured rotation limits and head-motion policy state;
- raw, post-limit, optionally aligned, and finally applied gaze direction;
- final left/right local eye-bone quaternion and mapped bone identifier;
- packet age, source sequence/timestamp when available, and stale/invalid reason.

Use a separate opt-in path such as `MITTENS_EYE_PROBE_PATH`. If source timestamps are required,
introduce an explicitly versioned UDP v2 packet; do not silently reinterpret v1 or use UDP arrival
time as if it were the headset sample time.

### Analysis tool

Add a deterministic command-line script that accepts one or two trace files and produces:

- a cleaned tabular dataset with quaternion sign continuity and normalized vectors;
- time plots for head yaw/pitch/roll, angular velocity/acceleration, left/right gaze angles,
  binocular common mode, left-right difference, openness, and validity;
- windowed RMS/variance for stationary and moving segments;
- normalized cross-correlation over at least `-250..+250 ms` for each head/gaze axis;
- a lag report naming the peak coefficient and lag per axis;
- robust or ridge regressions of gaze velocity against head angular velocity and acceleration;
- residual plots after applying candidate lag/gain models offline;
- a machine-readable summary so two fit conditions or algorithm candidates can be compared.

For small angular motion, analyze yaw/pitch channels directly. For larger motion, use quaternion
relative rotations/log maps rather than subtracting Euler angles across wrap boundaries. Resample
only for analysis and preserve the original timestamps and validity mask.

## Controlled recording protocol

Use separate 15–20 second files. Begin and end each with two seconds held still. Repeat yaw, pitch,
and roll independently before combining axes.

1. **Head still, HMD-fixed target:** look at a reticle rigidly attached to the headset view.
2. **Head moving, HMD-fixed target:** slow sinusoidal turns, then faster turns, while keeping gaze on
   the reticle. This is the primary hardware-error trial.
3. **Head moving, world-fixed target:** fixate a real stationary point. This captures legitimate
   vestibulo-ocular response and must remain labeled separately.
4. **Eyes moving, head still:** left/right/up/down fixation and several deliberate saccades. This
   establishes intended gaze gain and per-eye independence.
5. **Micro-motion:** head still by intent, then gentle taps or short high-frequency movements to
   expose vibration sensitivity.
6. **Fit A/B:** repeat trials 1–5 with normal and deliberately snug fit. If safe and useful, record a
   looser fit as a third condition; never obscure ventilation or over-tighten the HMD.

Record calibration state, wearer, fit condition, target type/distance, approximate movement tempo,
and whether `rotation_limits`/freeze were disabled. Source diagnosis runs must use raw gaze with all
Mittens mitigations off; those policies may be replayed offline afterward.

Run every important condition at least three times. Do not infer a correction from a single noisy
trial.

## Derived signals and model selection

Convert each gaze quaternion to forward direction, then to angular coordinates in the declared
head basis. Define:

```text
g_common = 0.5 * (g_left + g_right)
g_diff   = 0.5 * (g_left - g_right)
```

`g_common` exposes motion shared by both eyes; `g_diff` exposes vergence, asymmetric tracking, and
independent-eye information. Fit each independently.

Candidate explanatory model, evaluated offline only:

```text
g_error(t) = A * head_angle(t - lag)
           + B * head_angular_velocity(t - lag)
           + C * head_angular_acceleration(t - lag)
           + residual(t)
```

Select the smallest model that generalizes to held-out repetitions. Report baseline stationary
noise, moving-trial error, explained variance, residual RMS, peak lag, and the change between fit
conditions. A model that only improves its training capture is rejected.

## Decision gates

- **Timestamp alignment:** pursue when a stable non-zero lag explains the HMD-fixed common-mode
  response across axes and repetitions, and offline alignment reduces error without creating a held
  pose offset.
- **Static basis correction:** pursue only when a repeatable axis/gain mapping exists at zero lag
  and remains present at held rotated poses.
- **Physical-disturbance filter:** pursue only when acceleration/vibration and fit condition explain
  the response better than angle or fixed lag. The production option must be clearly labeled as a
  hardware workaround and bounded so it cannot erase ordinary gaze.
- **Mittens pipeline fix:** pursue when Probe A is clean and Probe B identifies the first divergent
  processing stage.
- **No correction:** choose this when the apparent signal is predominantly legitimate world-target
  fixation or no model generalizes across recordings.

Potential production mitigations include timestamped head-basis interpolation, a calibrated and
bounded common-mode subtraction model, motion-conditioned filtering, or the existing freeze
policy. None becomes default behavior, and none is implemented in the probe phase.

## Separate HTC blinking workstream

The current HTC packet provides per-eye openness, but `XREyeTrackingHTCComponent` retains only gaze.
Implement blinking independently of the motion probe:

- retain fresh left/right openness with independent validity and sequence/timestamp;
- define closure as `clamp(1 - openness, 0, 1)`;
- route left/right closure through explicit avatar morph mappings;
- release each live morph override immediately when its field becomes invalid or stale;
- test unilateral validity, open/partial/closed values, packet loss, and source switching;
- keep gaze filtering and blink routing independently configurable.

## Implementation checklist

- [ ] Add opt-in ALVR source recorder and schema/version metadata.
- [ ] Add synthetic tests proving one row preserves timestamps, head pose, both gaze quaternions,
      validity, and all per-eye detail.
- [ ] Add the analysis CLI with generated fixtures of known gain, lag, noise, and missing samples.
- [ ] Capture the controlled trial matrix on the Focus 3.
- [ ] Publish plots and machine-readable summaries under an ignored capture/artifact directory.
- [ ] Decide H1–H5 using the gates above; record rejected hypotheses as well as the winner.
- [ ] Add the Mittens pipeline recorder only if the source trace cannot localize the defect.
- [ ] Replay candidate mitigations offline and validate on held-out recordings.
- [ ] Implement one opt-in mitigation only after a model generalizes.
- [ ] Implement and headset-test the separate HTC per-eye blink path.

## Acceptance

The investigation is complete when another developer can repeat the capture protocol and obtain the
same diagnosis, the chosen model predicts held-out motion better than the raw baseline, and the
evidence identifies the ownership layer for the fix. A mitigation is complete only when neutral
HMD-fixed gaze remains stable during motion, intentional gaze and world-fixation response remain
usable, held poses gain no persistent offset, and `off` preserves existing behavior.

## Related files

- `../ALVR/ALVR/alvr/client_openxr/src/interaction.rs` — raw HTC extension sampling.
- `../ALVR/ALVR/alvr/packets/src/lib.rs` — `TrackingData`, `FaceData`, and `EyeDataHtc`.
- `../ALVR/ALVR/alvr/server_core/src/tracking/mod.rs` — synchronized server tracking boundary.
- `../ALVR/ALVR/alvr/server_core/src/tracking/face.rs` — Mittens UDP serialization.
- `src/engine/ecs/system/xr_eye_tracking_system.rs` — HTC receive/decode boundary.
- `src/engine/ecs/system/avatar_control_system.rs` — limits, motion policy, and eye-bone application.
- `examples/vtuber-eye-tracking-mirror.mms` — normal headset reproduction scene.
- `examples/vtuber-eye-tracking-mirror-eye-stabilize.mms` — freeze experiment scene.

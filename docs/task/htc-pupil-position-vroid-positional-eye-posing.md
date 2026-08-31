# Task: HTC pupil-position posing for VRoid eyes

Status: planned / raw transport exists, positional AVC posing not implemented

Parent: [Eye and face tracking epic](epic/eye-and-face-tracking.md)

Related:

- [Pupil direction, pupil size, and eye openness](pupil-direction-size-and-openness.md)
- [Unified two-eye tracking normalization and AVC routing](unified-two-eye-tracking-normalization-and-avc-routing.md)
- [Head-motion probe and sample alignment](eye-tracking-head-rotation-compensation-sample-alignment.md)

## Outcome

Determine whether the Vive Focus 3's native per-eye pupil positions produce better-looking and more
stable eye motion on VRoid-style cartoon avatars than its per-eye gaze quaternions. Expose the
choice and its calibration through `XREyeTrackingHTC` builder methods, while retaining both raw
signals so the two methods can be compared from the same capture.

This is an empirical posing option, not a claim that pupil position is a gaze direction. The default
must remain quaternion-driven rotation until headset testing demonstrates a better policy.

## Source data and current boundary

HTC supplies three relevant but distinct values per eye:

- `gaze_pose.orientation`: a quaternion describing the gaze ray orientation;
- `gaze_pose.position`: the 3D gaze-ray origin in the requested OpenXR reference space;
- `pupil_position`: a 2D observation in the eye-tracker sensor area, normalized to `[0, 1]`, with
  `+X` right and `+Y` up.

The ALVR Mittens v1 packet already preserves the gaze orientation and 2D pupil position separately,
with independent validity flags. It does not carry the 3D gaze origin. Do not redefine the existing
position field or convert it in ALVR: Mittens should retain the raw `[0, 1]` values and own
calibration and avatar application.

Today `XREyeTrackingHTC` decodes pupil position into the source-native update event, but does not
retain it as normalized runtime state or apply it through AVC. AVC only converts the gaze quaternion
to a direction and rotates the mapped `left_eye` and `right_eye` targets.

References:

- [VIVE OpenXR eye-tracker data guide](https://developer.vive.com/resources/openxr/unity/tutorials/face-data/getting-the-data-of-eye-tracker/)
- [HTC `XrSingleEyePupilDataHTC` API reference](https://hub.vive.com/apidoc/api/VIVE.OpenXR.EyeTracker.XrSingleEyePupilDataHTC.html)

## Why positional posing may fit VRoid avatars

The maintained VRoid map resolves `J_Adj_L_FaceEye` and `J_Adj_R_FaceEye`. Quaternion-driven eye
rotation is natural for spherical eyeballs, but many cartoon eyes visually behave more like pupils
sliding over a shallow surface. For those rigs, a small rest-relative local translation may look
better than rotation.

This must be verified per avatar. Translating a mapped eye target may move the whole eyeball rather
than an independently authored pupil. The probe must therefore inspect the actual node/skin
influence and compare:

1. rotation of the existing VRoid eye targets;
2. translation of those targets when that is visually valid;
3. translation of dedicated left/right pupil targets if the avatar supplies them;
4. no positional application when the rig has no suitable target.

Never silently assume that every VRoid export has independently translatable pupil geometry.

## Normalized pupil-position state

Add a retained, independently sequenced two-eye pupil-position sample to
`XREyeTrackingHtcComponent`:

```rust
struct EyePupilPositionSample {
    left: Option<[f32; 2]>,
    right: Option<[f32; 2]>,
    sequence: u64,
}
```

Requirements:

- preserve valid source values before calibration;
- reject non-finite values and respect the packet validity bits;
- preserve explicit left/right observations independently;
- use the documented unilateral fallback policy only when explicitly selected for positional
  posing, because mirroring one sensor coordinate can turn asymmetric tracking loss into symmetric
  avatar motion;
- keep pupil-position freshness independent from gaze and closure freshness;
- retain source-native pupil position in `XrEyeTrackingHtcUpdated` for diagnostics.

## Proposed `XREyeTrackingHTC` builders

The first implementation should expose a small experimental surface:

```mms
XREyeTrackingHTC.on()
    .eye_pose_mode("rotation")
```

```mms
XREyeTrackingHTC.on()
    .eye_pose_mode("pupil_translation")
    .pupil_position_center([0.5, 0.5], [0.5, 0.5])
    .pupil_translation_range([0.01, 0.006], [0.01, 0.006])
```

Candidate modes:

- `rotation`: current quaternion-to-eye-rotation behavior and the compatibility default;
- `pupil_translation`: translate suitable left/right targets from pupil position and leave their
  authored rotation intact;
- `rotation_and_pupil_translation`: diagnostic mode only, allowing both signals to be visualized
  together when the rig has distinct eye and pupil targets.

Builder semantics:

- `pupil_position_center(left, right)` defines the per-eye sensor coordinate that maps to the
  authored/rest position;
- `pupil_translation_range(left, right)` defines the maximum local X/Y displacement in engine
  units after centering; both components must be finite and non-negative;
- a later `pupil_position_axes(...)` or sign builder may be added only if live evidence shows an
  axis inversion that cannot be handled by a documented canonical conversion;
- defaults are neutral and preserve current behavior;
- all builders must be registered in the configured runtime, applied by the component registry,
  serialized through MMS, and covered by builder validation tests.

Keep source calibration and rig mapping conceptually separate. These component builders are an
expedient experimental surface for the first VRoid comparison. If multiple avatar conventions need
different target/range policy, move the application range and target selection into an explicit
avatar eye map without changing the retained tracker state.

## Positional conversion

For each valid eye, begin with a linear, rest-relative mapping:

```text
centered_x = pupil_x - center_x
centered_y = pupil_y - center_y
local_translation_x = centered_x * 2 * range_x
local_translation_y = centered_y * 2 * range_y
```

Apply the result relative to the target's immutable rest translation. Never accumulate translation
from the preceding frame. Do not write Z until a real rig demonstrates a requirement. Clamp to the
configured range so sensor spikes cannot pull the eye outside its socket.

The sign and basis must be validated visually against a target looking left/right/up/down. If an eye
target's parent basis differs from the canonical VRoid basis, resolve that through explicit mapping
or calibration rather than a model-name heuristic.

## Implementation slices

### Slice 1 — retain and expose the signal

- Add the per-eye pupil-position runtime sample to `XREyeTrackingHtcComponent`.
- Populate it from the existing Mittens v1 packet without changing ALVR.
- Add focused validity, distinct-eye, unilateral, and freshness tests.
- Add a diagnostic callback/example that displays the two raw `[x, y]` values alongside gaze.

### Slice 2 — builder and positional application

- Add `eye_pose_mode`, `pupil_position_center`, and `pupil_translation_range` builders.
- Resolve suitable left/right positional targets without inventing missing pupil nodes.
- Apply clamped rest-relative local translation independently per eye.
- Release positional ownership and restore authored translations when samples become unavailable or
  the mode changes.
- Preserve the existing quaternion rotation path unchanged in `rotation` mode.

### Slice 3 — VRoid comparison harness

- Add a focused VRoid/Bisket scene that can switch between rotation and pupil translation without
  restarting the headset stream.
- Record raw per-eye pupil position, gaze direction/quaternion, head pose, validity, and applied
  transforms on one shared timeline.
- Test deliberate left/right/up/down gaze while the head is still, followed by eyes-fixed head
  translations, rotations, and small headset slips.
- Capture video and time-series data using identical calibration and motion sequences for both
  modes.

## Evaluation

Compare the two modes on:

- visible correspondence with deliberate eye motion;
- independence of left and right eyes, including winks and asymmetric gaze;
- response to headset rotation, translation, vibration, and fit changes;
- jitter while the user fixes gaze on one point;
- range, centering, clipping, latency, and recovery after invalid samples;
- whether the target motion deforms only the intended pupil/eye region.

Pupil position may be more suitable for cartoon posing, but it may also be more sensitive to the
eye moving relative to the headset sensor when the headset slips. The comparison must preserve that
possibility rather than treating apparent motion as gaze by definition.

## Acceptance

- `XREyeTrackingHTC` retains raw valid left/right pupil positions independently.
- MMS exposes validated, serializable mode, center, and translation-range builders.
- Quaternion rotation remains the default and existing scenes behave unchanged.
- Positional mode applies bounded, rest-relative translation only to explicitly suitable targets.
- Loss or invalidity restores authored target translations.
- The VRoid comparison produces synchronized evidence showing which signal and posing method is
  more stable and visually useful under both intentional gaze and headset motion.

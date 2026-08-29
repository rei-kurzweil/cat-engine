# Eye-tracking head-rotation compensation through sample alignment

Status: freeze experiment implemented / not yet validated on a headset

## Why the first compensation approach is invalid

The initial `CancelHeadRotation` experiment interpreted a tracker gaze vector
as world-relative. AVC transformed it through the inverse of the eye bone
parent's current world rotation before writing the eye-local correction.

That assumption fails in `examples/vtuber-eye-tracking-mirror-eye-stabilize.mms`:
with neutral gaze, the pupils are not visible in the mirror. The avatar's
initial orientation can turn the nominal forward vector from `-Z` to `+Z`,
rotating the eyes away from the viewer. The ordinary mirror scene preserves
neutral pupils at a held head pose, which is evidence that the transport is
already effectively head-relative in its steady state.

Therefore, a static inverse of the current head/eye-parent rotation cannot be
a valid generic definition of `CancelHeadRotation`. It breaks the required
invariant that neutral gaze remains neutral after the head settles.

The standalone `examples/eye-tracking-head-rotation-compensation.mms` cannot
validate this policy because it visualizes raw OSC values only; it has no AVC
eye-bone target. Use the VTuber mirror scene for avatar-level validation.

## Revised hypothesis

The transient counter-rotation may be caused by a gaze vector and HMD/head pose
from different instants. A delayed or filtered gaze sample can be combined with
the current head pose while the wearer rotates, then appear normal again once
head movement stops.

If a gaze sample is expressed in the head basis at time `t_gaze`, the eye-local
conversion needs the relative rotation from that basis to the head basis at
the frame where AVC writes the eye pose, not the inverse of the absolute head
rotation:

```text
head_delta = inverse(head_rotation_at_gaze) * head_rotation_now
aligned_gaze = inverse(head_delta) * raw_gaze
```

The multiplication order and source coordinate convention must be confirmed
with controlled recordings before this becomes production behavior.

## Implemented interim mitigation: freeze during rapid head motion

AVC now exposes an opt-in policy:

```mms
AVC {
    head_motion_gaze_policy("freeze")
}
```

It captures the latest head-relative gaze and holds it while the effective
eye-parent basis turns faster than 30°/s. It resumes live gaze after the basis
remains below 25°/s for 100 ms. The default is `"live"`. The thresholds are
intentionally internal and provisional; headset testing must establish whether
they are useful.

While frozen, the eye bones continue to inherit normal avatar head motion; only
new tracker gaze directions are ignored. This prevents a noisy tracker from
turning the pupils against the head while the wearer moves quickly. It also
suppresses intentional eye motion during the freeze window.

`examples/vtuber-eye-tracking-mirror-eye-stabilize.mms` uses this policy. It
does not enable the rejected absolute `head_rotation_compensation("cancel")`
experiment.

## Deferred experiment: one-frame head-basis delta

Until the transport supplies timestamps, test a deliberately limited
approximation:

1. AVC records the effective head/eye-parent world rotation each frame.
2. For a tracker configured to opt in, AVC computes the relative rotation from
   the previous recorded basis to the current basis.
3. AVC applies the inverse of that *delta* to the incoming gaze before its
   existing rest-relative gaze-to-eye-local conversion.
4. AVC never applies an absolute inverse head rotation.

This can only compensate a one-frame mismatch. It must not be presented as
latency correction for arbitrary transport delay, filtering, or packet loss.
At startup and at a held pose the delta is identity, so neutral pupils remain
visible.

The existing `head_rotation_compensation("cancel")` name is misleading while
it has the absolute-basis behavior. Before enabling the revised experiment,
either replace it with an explicitly temporary policy such as
`"previous_frame_delta"`, or redefine `"cancel"` and document its strictly
bounded one-frame semantics.

## Longer-term implementation: timestamped sample alignment

The correct solution requires a history of head rotations indexed by monotonic
time and a gaze sample timestamp from the transport:

1. Store each valid gaze sample with its source timestamp and coordinate-space
   declaration.
2. Store a short history of effective head-basis rotations with engine frame
   timestamps.
3. Interpolate the head basis at the gaze timestamp.
4. Convert the gaze from that sampled basis into the current target eye-parent
   basis, preserving the immutable eye rest rotation.
5. Reject or report stale samples and samples outside the available history.

Do not infer source timestamps from UDP arrival time unless explicitly testing
that approximation; arrival time includes network and scheduling jitter.

## Acceptance checks

- At startup and with a held rotated head, neutral gaze keeps both pupils
  visible and forward-facing.
- During deliberate yaw, pitch, and roll with neutral gaze, the mitigation
  reduces the transient opposite eye motion without producing an enduring
  offset.
- Intentional left/right/up/down gaze retains its direction during head motion.
- `off` is byte-for-byte equivalent to the pre-compensation AVC output for a
  fixed valid sample.
- Source switching, absent tracker restoration, unmapped eye slots, and HTC
  transport continue to behave as before.
- Record raw gaze, sample/arrival timestamps, current and aligned head-basis
  rotations, and final local eye rotations for headset comparison.

## Related files

- `docs/bugs/eye-tracking-head-rotation-counteracts-gaze.md`
- `src/engine/ecs/system/avatar_control_system.rs`
- `src/engine/ecs/system/xr_eye_tracking_system.rs`
- `examples/vtuber-eye-tracking-mirror.mms`
- `examples/vtuber-eye-tracking-mirror-eye-stabilize.mms`

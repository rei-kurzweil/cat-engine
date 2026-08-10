# VTuber slide-deck XR placement and controls

Status: in progress; hands-on XR verification required.

Related:

- [Manual animation keyframe stepping and XR slide controls](manual-animation-keyframe-stepping-and-xr-slide-controls.md)
- [Transform parent component-ref routing](transform-parent-component-ref-routing.md)
- [Transform pipeline cleanup checklist](transform-pipeline-cleanup-checklist.md)

## Goal

Make `vtuber-slidedeck` comfortable to read in XR, keep its mirror unobstructed, make controller
input observable from ordinary stdout, and change slide placement from continuous XR-rig parenting
to a detached world-space snapshot taken when a slide is selected.

## Reported observations

- The text begins too far left and is only partly inside the XR view frustum.
- A beige backing cube appears to obstruct the mirror.
- Pressing A or B does not visibly change the text.
- The text follows the XR/avatar hierarchy continuously. It should instead copy the current rig
  pose when a slide is selected and remain at that world pose while the user moves away.

## Working checklist

- [x] Shift the current avatar-relative slide placement 0.5 local units to the right,
  changing X from `-1.45` to `-0.95` in the initial state and all five keyframes.
- [x] Remove the beige mirror-backing cube from this example.
- [x] Add Rust-side stdout traces for every XR button down/up event, independent of MMS handler
  dispatch.
- [x] Put MMS ButtonA/ButtonB traces before `previous()`/`next()` so a method-dispatch failure
  cannot hide proof that the script handler ran.
- [ ] Verify on hardware that ButtonB and ButtonA produce Rust `[xr-button]` traces.
- [ ] Verify that the matching MMS `received ButtonB` / `received ButtonA` trace follows.
- [ ] Verify that successful ButtonB presses visit all five text/color/size states and clamp at
  the last state; verify ButtonA walks backward and clamps at the first.
- [ ] Detach `slide_root` from the XR/avatar tree.
- [ ] Snapshot the current rig/world pose when a slide changes, apply the desired local
  presentation offset, and update the detached `slide_root` once.
- [ ] Verify that the placed slide remains stationary when the XR rig subsequently moves.
- [ ] Recalibrate horizontal and forward offsets after detached snapshot placement exists.
- [ ] Recheck mirror visibility, stereo readability, and frame time in release mode.

## Trace interpretation

Expected stdout for one physical B press has two independent layers:

```text
[vtuber-slidedeck][xr-button] edge=down hand=Right control=ButtonB ...
vtuber-slidedeck: received ButtonB; requesting next slide
```

Interpretation:

- Neither line: the OpenXR/gamepad layer did not emit the expected button event, or stdout is not
  the process being observed.
- Rust line only: engine input works, but the MMS scoped handler did not run or did not recognize
  `event.control`.
- Both lines but unchanged text: the failure is after handler dispatch, in step intent processing
  or the keyframe's text/color/transform mutations.
- Both lines and changed text: input and animation stepping work; remaining problems are placement
  and readability.

The trace is edge-based. Holding a button should not repeatedly advance slides without another
button-down event.

## Current transform capability audit

MMS currently supports:

- `transform.translation()`, which returns the transform component's **local** translation as a
  three-number array;
- storing that array in an MMS variable and reading its elements;
- `transform.update_transform(translation, rotation_euler, scale)`, which writes a complete local
  TRS through the intent pipeline;
- `TransformParent.target(...)`, which continuously supplies another transform as a parent-world
  basis.

MMS does not currently expose:

- local rotation or scale getters;
- a world translation, world rotation, or world scale getter;
- a complete local or world transform value/matrix;
- a quaternion-preserving `update_transform` variant for copied poses;
- a one-shot "copy this component's current world pose, then stop following it" operation.

Therefore a detached slide can copy only the rig's local translation today. That is insufficient:
it loses facing/orientation, ignores ancestor transforms, and cannot correctly rotate the slide's
presentation offset into the sampled rig pose. `TransformParent` is also not the answer because it
would preserve the unwanted continuous relationship.

## Candidate implementation seam

Prefer an atomic engine operation over assembling a world-pose copy from several MMS getters. A
candidate surface is:

```mms
slide_root.snapshot_world_pose_from(
    avatar_locomotion_root,
    [-0.95, 0.15, -1.25],
    [0.0, 3.14159, 0.0],
    [0.055, 0.055, 1.0],
)
```

The operation would:

1. sample the source's propagated world matrix at the normal intent drain point;
2. compose the authored local presentation offset with that matrix;
3. convert the result into the detached target's parent space (world space for a root);
4. update the target once without retaining a source reference.

This avoids separately reading translation and rotation at different points in a frame, avoids
lossy quaternion-to-Euler round trips, and makes snapshot semantics explicit. Open questions:

- Should the offset be a structured TRS value rather than three arguments?
- Should the generic name be `copy_world_pose_from`, `snapshot_world_pose_from`, or
  `place_relative_to_now`?
- Must the target be a world root, or should arbitrary target parent spaces be supported?
- Should transition components be bypassed by default for an instantaneous snapshot?
- Should the operation sample the locomotion root, the tracked head/camera pose, or an authored
  presentation anchor beneath the rig?

The last question requires XR review: sampling the locomotion root preserves avatar-level
placement, while sampling the headset better matches what the user is looking at when advancing a
slide.

## Verification command

```bash
cargo run --release --example vtuber-slidedeck
```

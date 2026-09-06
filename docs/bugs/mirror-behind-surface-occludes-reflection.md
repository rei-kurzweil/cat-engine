# Geometry behind a mirror can occlude its reflection

## Status

Open bug / rendering correctness issue.

## Symptom

Geometry immediately on the far side of a mirror can appear in the mirror
capture and cover the reflection that should be visible there. The observed
distance threshold is not yet measured: the failure is apparent when the
geometry is roughly one or two world units behind the mirror, and may persist
farther away.

This is not the expected reflection of geometry in front of the mirror. It is
geometry behind the reflective surface, which a physically correct planar
mirror capture must clip away.

## Repro

Use [examples/e2.mms](../../examples/e2.mms):

1. Start the desktop scene and face `#e2_mirror`.
2. The back wall is a four-cube window frame at `z = -5.9`, behind the initial
   mirror placement, so the sky is visible through the center gap.
3. Grab the mirror with the desktop pointer and move it toward or away from
   the wall while viewing its surface.
4. Observe whether the back-wall sections appear in the mirror capture and
   obscure the avatar, tablet, or rest of the expected reflection.

Record the mirror-to-wall separation at which the artifact starts and stops,
and test each section separately if the result differs by screen coverage.

## Expected behavior

The reflective plane is the boundary of the capture. Scene geometry on the
mirror's far side must be rejected by the reflected-camera clip plane, while
objects on the viewer's side remain eligible to appear in the reflection.

The window gap should show the reflected sky/environment rather than a
depth-incorrect wall layer covering unrelated reflected content.

## Likely investigation area

This is consistent with a missing, incorrectly oriented, or incorrectly
positioned oblique near-plane clip for the mirror capture. Verify that the
clip plane:

- uses the visible reflective face rather than the cube center or back face;
- is transformed into the reflected camera's view space with the correct
  normal sign; and
- is applied to every mirror capture before opaque depth testing.

Relevant code:

- [src/engine/ecs/system/mirror_system.rs](../../src/engine/ecs/system/mirror_system.rs)
- [src/engine/graphics/vulkano_renderer.rs](../../src/engine/graphics/vulkano_renderer.rs)

## Relation to existing mirror bugs

This is distinct from the reflected-pose error in
[mirror-camera-orientation-and-tracking.md](mirror-camera-orientation-and-tracking.md)
and the XR viewer-family selection issue in
[mirror-vr-uses-static-monoscopic-perspective-when-window-camera-is-active.md](mirror-vr-uses-static-monoscopic-perspective-when-window-camera-is-active.md).
Those bugs concern reflection pose/source selection; this one concerns
far-side geometry leaking into an otherwise selected mirror capture.

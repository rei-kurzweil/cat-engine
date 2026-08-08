# Task: transform gizmo minimum screen extent

Date: 2026-08-08

Status: todo / needs measurement

Related:

- `docs/bugs/transform-gizmo-screen-size-varies-with-camera-distance.md`
- `docs/spec/transform-camera-specific.md`
- `docs/spec/screen-space-lines.md`
- `docs/draft/xr-captured-drag-mapping.md`

## Goal

Prevent transform gizmos from becoming too small to identify or acquire while preserving the
existing intended distance compensation at ordinary sizes.

The initial UX target to evaluate is a minimum projected gizmo extent of approximately 10% of the
relevant viewport width. This is a floor, not a request to enlarge every gizmo globally.

## Current state

`TransformGizmoSystem::update_camera_scales(...)` currently derives scale from camera-space depth
and clamps the world-scale result to `[0.02, 20.0]`. A fixed world-scale clamp does not directly
express a minimum screen-space or angular extent and can behave differently across viewport sizes,
FOVs, aspect ratios, and XR eyes.

The existing screen-size bug tracker covers stale or missing distance compensation. This task is a
separate usability floor to apply after that behavior is understood.

## Questions to resolve

- Measure the whole gizmo, the longest axis, or a projected bounding circle?
- Is 10% of viewport width the right floor, or should the metric use `min(width, height)`?
- Should desktop use pixels/NDC while XR uses angular degrees?
- In stereo, should the cyclopean view determine one floor or should both eyes satisfy it?
- Does the maximum size also need an explicit projected-space ceiling?
- How do visible geometry and BVH/raycast bounds remain synchronized after clamping?

## Proposed implementation direction

1. Project representative gizmo bounds with the same camera family used for camera-specific
   transforms.
2. Compute the current projected extent.
3. Apply additional scale only when that extent is below the agreed minimum.
4. Propagate the resulting transform before BVH refit and raycasting.
5. Keep the authored `gizmo.scale` as the ordinary-size multiplier rather than overwriting it with
   the accessibility floor.

## Acceptance criteria

- [ ] Agree on a projected-space metric and document why it works for desktop and XR.
- [ ] A far-away gizmo does not fall below the agreed minimum extent.
- [ ] Near and normally sized gizmos are unchanged by the floor.
- [ ] The floor adapts to window resizing and camera FOV changes.
- [ ] XR behavior is stable between eyes and does not pulse as the head moves.
- [ ] Visible handles and interactive/raycast bounds use the same effective scale in the same
      frame.
- [ ] Automated tests cover near/far targets, multiple aspect ratios, FOV changes, and the clamp
      boundary.
- [ ] Manual checks include both VTuber examples and a desktop editor scene.

## Non-goals

- Increasing the base gizmo size everywhere.
- Fixing the existing stale camera-compensation regression solely by raising the world-space
  minimum.
- Changing drag mapping or grid snapping.


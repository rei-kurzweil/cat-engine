# Transform gizmo planar translation handles

Date: 2026-07-28

Status: follow-up task

Related:

- `docs/task/unified-grid-snap-mode-mms-gizmo-and-paint.md`
- `docs/spec/grid-snapping.md`

## Goal

Add planar translation handles between each pair of transform-gizmo axes. A
planar handle lets the user move a target across two axes without changing the
third and provides a natural interaction for snapping both corresponding grid
coordinates at once.

This document defines the follow-up only. Planar handles are not part of the
initial `GridSnapMode` implementation.

## Handle set and appearance

Add three planar handles:

| Plane | Locked axis | Color |
|---|---|---|
| XY | Z | yellow |
| YZ | X | cyan |
| XZ | Y | magenta |

Each handle should appear as a compact filled or outlined square positioned in
the positive quadrant between its two axis handles. It must remain visually
distinct from the axis arrows and rotation rings at normal editor viewing
distances.

The hit target may be slightly larger than the visible square, but planar
handles must not obscure or steal ordinary axis-handle drags near the origin.

## Interaction contract

On drag start:

- capture the target pose and pointer hit in world space
- resolve the selected plane in the gizmo's current world/local coordinate mode
- construct a stable drag plane from the two allowed axes

During drag:

- intersect or project pointer movement onto the captured plane
- derive every candidate pose from drag-start state rather than accumulating
  frame-to-frame deltas
- change only the two allowed translation degrees of freedom
- preserve the locked coordinate exactly

World/local gizmo mode changes the plane basis in the same way it changes the
existing axis handles.

## Grid snapping

When a grid is selected, planar movement uses the shared snap request introduced
by the unified `GridSnapMode` work.

- If the handle plane corresponds to the selected grid's in-plane XZ plane,
  snap both grid-local X and Z.
- If the handle contains the grid normal, snap the one grid-local in-plane
  coordinate it controls and preserve the plane offset movement.
- For an oblique relationship between gizmo and grid planes, preserve the
  planar constraint. Snap only grid coordinates that can be satisfied without
  moving outside the selected plane.
- `Origin` and `Bounds` choose the anchor in exactly the same way as axis-handle
  translation.

The locked gizmo coordinate must never change as a side effect of grid
quantization.

## Tests and acceptance criteria

- XY, YZ, and XZ handles render with yellow, cyan, and magenta respectively.
- Each handle is independently raycastable and routes to the intended gizmo.
- Dragging changes exactly two coordinates and preserves the locked coordinate.
- World and local coordinate modes produce the expected plane orientation.
- Repeated drag updates do not accumulate drift.
- Grid-aligned XZ movement snaps both in-plane grid coordinates.
- A plane containing the grid normal preserves the appropriate lateral
  coordinate while allowing normal movement.
- Bounds-mode planar movement keeps cell-aligned object edges on grid lines.
- Origin-mode planar movement keeps the transform origin on the applicable grid
  lines.
- Desktop and XR pointers produce equivalent constrained movement.


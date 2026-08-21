# Paint-grid stroke cell deduplication and desktop repro

Date: 2026-08-21

Status: active

## Report

In the paint-stroke demo, a stroke can revisit an already-painted location and
commit another object at the exact same pose. The overlap is invisible until
the objects are selected or otherwise separated. The report is most noticeable
on the vertical diagnostic grid, where initiating a stroke can also be
intermittent and the first placement can appear in an adjacent cell.

This tracker deliberately separates three observations that may share an input
or grid-address cause but must not be conflated:

1. A single stroke must not commit more than one object for the same captured
   grid/cell key.
2. A desktop pointer must be able to start a stroke reliably on the intended
   raycastable surface.
3. The first and subsequent placements must resolve the cell under the pointer,
   not an adjacent cell caused by a coordinate-frame, phase, or hit-mapping
   error.

Repeated independent strokes are not automatically deduplicated by this task;
their intended occupancy/overwrite policy needs an explicit product decision.

## Isolated reproduction

Use the desktop-only scene:

```sh
MITTENS_DEBUG_PAINT_STROKE=1 cargo run --example paint-grids-desktop
```

`examples/paint-grids-desktop.mms` intentionally has no `InputXR`, `CXR`,
`XRHand`, or `XR` nodes. It retains the desktop camera/pointer, vertical grid,
wall targets, and editor panels needed to reproduce the issue without OpenXR
or XR input state participating.

## Required investigation

- Log the captured grid identity, local hit point, `GridAddress`, pointer ray,
  gesture phase, and committed placement pose for every start/move/end.
- Confirm whether Free Draw, Grid Tool, and Spray Can each use a durable
  per-stroke `HashSet<(grid identity, GridAddress)>` before committing.
- Test a back-and-forth stroke over the same vertical-grid cell. It must commit
  exactly one object for that key.
- Test a click and short drag at the center and near each boundary of a vertical
  cell; compare the rendered pointer contact, local coordinates, resolved cell,
  preview, and commit.
- Determine whether difficult stroke initiation is focus/activation gating,
  gesture threshold/capture, raycast ownership, or a grid-frame mapping error.

## Acceptance

- A single stroke never commits duplicate objects for an identical captured
  grid/cell key.
- Desktop stroke initiation is repeatable on the intended target without XR
  services or XR pointer topology.
- Preview and committed placement agree on the same cell and pose.
- The repro scene remains free of OpenXR/XR input dependencies.

## Verification snapshot (2026-08-21)

- `cargo check --example paint-grids-desktop` passes (with unrelated existing
  compiler warnings).
- The focused paint suite currently has one failing regression:
  `grid_drag_keeps_painting_scoped_to_one_editor` expected one painted root but
  observed two. Its movement sequence revisits the same grid cell, making it
  direct evidence that the per-stroke cell-deduplication acceptance criterion
  is not yet met.
- `cargo fmt --check` is presently unsuitable as a change-level gate because
  unrelated files already have widespread formatting drift; no unrelated
  formatting was rewritten for this task.

## Related

- [Grid-aware paint stroke interaction model](grid-aware-paint-stroke-interaction-model.md)
- [Grid + Gizmo + Paint end-to-end UX and test matrix](grid-gizmo-paint-end-to-end-ux-and-test-matrix.md)
- [Editor grid and paint 0.8.0 release gate](editor-grid-paint-0.8.0-release-gate.md)
- [Grid visual-coordinate-space tracker](grid-visual-coordinate-space-tracker.md)

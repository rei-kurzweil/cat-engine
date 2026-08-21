# Grid visual-coordinate-space tracker

Date: 2026-08-21

Status: active

## Purpose

Make the grid visual describe the same coordinate frame and spacing as grid
snapping by default, while retaining an explicit world-coordinate visual mode
for diagnostics and presentation. This is a rendering-coordinate task only:
it must not change paint or gizmo snap behavior.

Related:

- [Grid material spec](../spec/grid-material.md)
- [Grid snapping](../spec/grid-snapping.md)
- [Paint-stroke live diagnostics](../how_to/paint-stroke-live-diagnostics.md)
- [Grid + Gizmo + Paint end-to-end UX and test matrix](grid-gizmo-paint-end-to-end-ux-and-test-matrix.md)
- [Editor Grid and Assets panel widths](editor-grid-and-assets-panel-widths.md)

## Confirmed mismatch

`GridSystem` converts a candidate world point through the selected grid
transform, quantizes its local in-plane `X/Z` coordinates using
`GridComponent.spacing`, then transforms the result back to world space. That
is the frame used by both `snap_hit(...)` and
`snap_point_preserving_plane_offset(...)`.

The live grid visual does not currently use that frame:

- `assets/shaders/grid.vert` exposes world position as `v_world_pos`;
- `assets/shaders/grid-square.frag` evaluates `v_world_pos.xz`; and
- its minor and major spacing are hard-coded to `1.0` and `8.0`.

Thus a translated, rotated, or non-unit grid can draw boundaries other than
the ones that snapping uses. On a vertical grid, world-XZ evaluation also
collapses one coordinate family, leaving only one meaningful stripe family.

## Reproduction

Run the paint-stroke diagnostic scene:

```sh
MITTENS_DEBUG_PAINT_STROKE=1 cargo run --example paint-stroke-debug
```

1. Select `debug_floor_grid_spacing_0_75`, the translated and yawed floor
   grid. Its visible lines do not agree with cells/boundaries in its local
   snap frame.
2. Select `debug_vertical_grid_spacing_0_5`. The grid-local X/Z plane is
   rotated into the wall, but a world-XZ shader evaluation yields only one
   useful line family.
3. Compare the selected-grid trace's local point/address and snapped world
   point with the visual. This isolates a visual-frame mismatch from the
   separate Paint hit-grid resolution issue.

## Target contract

`GridVisualSpace` is a persisted enum on `GridComponent`:

- `Local` is the default. Render in the selected grid's local in-plane
  coordinates, using its authored `spacing`. A line at local `x = n * spacing`
  or `z = n * spacing` must be the same boundary used by snapping.
- `World` is visual-only diagnostic/presentation mode. Snapping remains
  grid-local and unchanged.

Expose the mode in every Grid-panel row and serialize it with `GridComponent`.
The exact MMS spelling may follow the existing builder convention (for example
`Grid.visual_space("local")` / `Grid.visual_space("world")`), but scene
round-tripping must preserve it and old scenes must default to `Local`.

## Rendering design

Use one arbitrary-plane material/shader path for horizontal, vertical,
rotated, and translated grids. Do not add separate horizontal and vertical
grid materials.

### Local mode

Pass a grid-local in-plane coordinate and the authored spacing to the grid
material. The coordinate must be in the grid owner's local frame, not merely
the live visual mesh's scaled coordinates. The material uses that coordinate
for minor, major, axis, and distance/fade calculations where applicable.

The source of truth is the same local `X/Z` convention used by `GridSystem`.
This deliberately leaves the current snap phase intact: intersection-oriented
gizmo snaps land at integer spacing multiples, while paint paths that use
cell addresses may use their explicitly defined cell-center phase.

### World mode

For a grid-plane normal `n`, project each world basis axis `e` into the plane:

```text
p(e) = e - dot(e, n) * n
```

Select the two axes with the greatest projected magnitude (equivalently, the
world axes most parallel to the plane). Break exact or near ties in stable
`X`, then `Y`, then `Z` order. Evaluate the corresponding world-coordinate
families on the plane; these are presentation coordinates and do not redefine
the snap frame or spacing policy.

This gives a vertical grid a `Y` family plus whichever of `X` or `Z` is more
parallel to the plane, rather than degenerating into world-XZ stripes.

## Implementation boundary

1. Add `GridVisualSpace::{Local, World}` to `GridComponent`, defaulting to
   `Local`, with builder/API support and component serialization.
2. Add a Local/World control to every Grid-panel row. Changing it updates only
   the visual/material inputs and rerenders the row; it does not modify active
   grid selection, bindings, or snapping.
3. Extend the grid visual data path so the shader receives local in-plane
   coordinates, authored spacing, mode, and the data needed for World-family
   selection. Do not infer local coordinates from world XZ.
4. Implement deterministic projected-axis family selection for World mode.
5. Keep `GridSystem`, paint placement, and gizmo snapping unchanged except for
   tests proving that the new visual mode has no effect on their results.

## Acceptance matrix

| Grid case | Local visual requirement | World visual requirement |
| --- | --- | --- |
| translated | Lines retain the translated local snap phase. | Lines remain world-coordinate presentation families. |
| yawed/rotated | Both local line families follow the grid plane and spacing. | Two projected world-axis families remain visible. |
| non-unit spacing | Minor-line intervals equal `GridComponent.spacing`. | Presentation intervals use the documented World-mode policy without changing snap spacing. |
| vertical | Local X/Z lines render as two wall-plane families. | `Y` plus `X` or `Z` renders; neither family degenerates. |
| near-vertical | Lines remain stable through small normal changes. | Axis choice uses the deterministic tie-break; no flicker at ties. |

For each case, assert that Local visual boundaries pass through the same local
boundaries—and, where paint intentionally uses them, the same cell centers—as
the corresponding `GridSystem` snap-frame calculation.

## Verification

- Unit-test World coordinate-family selection for horizontal, vertical, and
  arbitrary plane normals, including the deterministic near-tie behavior.
- Add a render/integration test proving that Local material coordinates and
  spacing equal the selected grid's `GridSystem` local snap frame.
- Verify a vertical grid shows two non-degenerate World-mode families.
- Run paint and gizmo snap tests in both visual modes and assert identical
  snap outputs and bindings.

## Non-goals

- World mode is not world-axis snapping.
- This task does not change Paint's current hit-owned-grid resolution or the
  existing intersection-versus-cell-center phase policy.
- This task does not revive the earlier horizontal/vertical material split;
  arbitrary-plane coordinates are the shared solution.

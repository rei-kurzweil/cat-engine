# Unified grid snap mode for MMS, gizmos, and paint

Date: 2026-07-28

Status: implementation plan

Related:

- `docs/spec/grid-snapping.md`
- `docs/task/grid-gizmo-paint-end-to-end-ux-and-test-matrix.md`
- `docs/task/transform-gizmo-planar-translation-handles.md`
- `assets/components/floors/voxel_terrain.mms`

## Summary

Grid selection is now independently persisted by `GridBinding.grid(...)` on
the manipulated transform. `GridBinding` resolves the grid frame first;
`GridSnapMode` remains responsible only for choosing origin-versus-bounds
anchor behavior within that frame.

Add a serializable `GridSnapMode` component with two MMS forms:

```mms
GridSnapMode.origin()
GridSnapMode.bounds()
```

The selected grid provides the default mode for snapping operations. A mode
attached directly to the manipulated object's owner transform overrides the
grid. If neither owner has a mode, snapping retains the current origin-based
behavior.

The component must drive one shared policy for transform-gizmo translation and
paint placement. In bounds mode, moving a cell-aligned object such as a
`voxel_terrain` cube must preserve its existing boundary alignment.

## MMS API and ownership

The Rust representation should use a `GridSnapModeComponent` backed by:

```rust
enum GridSnapMode {
    Origin,
    Bounds,
}
```

`GridSnapMode::Origin` is the component default.

The component is authored as an immediate child of the transform that owns the
policy. It is a sibling of the grid or renderable content rather than a child of
one particular renderable:

```mms
// Grid default.
T {
    Grid {}
    GridSnapMode.bounds()
}

// Object override.
T {
    GridSnapMode.origin()
    R.cube()
}
```

Resolve the effective mode in this order:

1. `GridSnapMode` directly owned by the manipulated target transform
2. `GridSnapMode` directly owned by the selected grid's owner transform
3. implicit `Origin`

Only one directly owned mode is valid per owner. Resolution should be
deterministic if malformed content contains more than one, but the editor and
MMS examples must not produce that shape.

New editor-created grids receive an explicit `GridSnapMode.origin()` component.
Legacy grids without the component display and behave as `Origin`. Once the
mode is changed through the editor, the explicit component is serialized with
the grid.

Painted objects introduce a generated placement/manipulation wrapper. When an
instantiated asset root directly owns a `GridSnapMode`, copy that value onto the
generated painted-object target transform so the authored override remains
effective during both placement and later gizmo manipulation.

## Shared snapping contract

Introduce one internal snap request/result path used by gizmo translation,
paint preview, and paint commit. A request must carry:

- selected active-grid frame and spacing
- resolved snap mode
- candidate object pose
- optional aggregate rendered-subtree bounds
- operation constraint, such as a gizmo axis or free in-plane paint placement
- whether the off-plane distance must be preserved

Snapping is evaluated in selected-grid-local coordinates and transformed back
to world space afterward.

### Origin mode

Use the manipulated target transform's local `(0, 0, 0)` as the anchor.
Quantize only the in-plane coordinate or coordinates allowed by the current
operation.

This is the compatibility fallback for:

- scenes without a `GridSnapMode`
- intersection-authored objects
- bounds mode when usable aggregate bounds cannot be measured

### Bounds mode

Measure the aggregate rendered subtree in the manipulated target's local frame.
At the candidate pose, transform its eight AABB corners into selected-grid-local
space and form the grid-local AABB envelope.

For each in-plane grid axis eligible for snapping:

1. calculate the correction that places the minimum bound on its nearest grid
   line
2. calculate the correction that places the maximum bound on its nearest grid
   line
3. apply the correction with the smaller absolute magnitude
4. prefer the minimum bound when the magnitudes are equal

If the object's size on that axis is a multiple of grid spacing, aligning either
edge also aligns the opposite edge. If it is not a multiple, align only the
nearest edge; do not reject the snap or fall back to the origin.

The bounds are evaluated at the candidate pose, including object rotation,
parent transforms, and grid rotation. For a rotated object, "bounds" means its
grid-local AABB envelope, so the aligned extreme may be a corner rather than an
entire object-local face.

## Transform gizmo behavior

Gizmo translation must use the shared request path while preserving the
selected handle's constraint:

- a handle parallel to grid-local X snaps only grid-local X
- a handle parallel to grid-local Z snaps only grid-local Z
- a handle parallel to the grid normal preserves both in-plane coordinates
- an oblique handle remains strictly constrained to its axis

For an oblique handle, select the grid-local in-plane axis most aligned with the
handle and snap against that coordinate only. The other grid coordinate is not
guaranteed to land on a line. This is preferable to silently moving the object
sideways off the selected handle.

Both modes preserve the target's distance from the grid plane. A one-axis
operation must not introduce movement that the selected handle did not permit.

Bounds mode must eliminate the current `voxel_terrain` failure in which the
first snapped move changes an aligned 3-unit cube from integer boundaries to a
half-cell phase.

## Paint behavior

Paint must resolve snapping from the selected active grid rather than requiring
the grid visual itself to win the raycast. The scene hit continues to provide
surface contact and orientation.

The placement sequence is:

1. instantiate the asset and resolve its optional object override
2. determine the candidate surface-aligned orientation and contact offset
3. measure the candidate asset bounds when bounds mode is effective
4. apply the shared grid snap to its in-plane position
5. retain the original hit height, normal, and off-plane contact offset
6. use the same final pose for preview and commit

This first version is intended for flat or coplanar surfaces such as the voxel
terrain. It does not re-raycast at the snapped lateral position. Curved-surface
projection and discontinuous terrain policies remain follow-up work.

## Grid panel

Extend the selected grid row with a settings strip beneath its existing main
row. The strip uses a horizontal, single-selection layout with two options:

- `Origin`
- `Bounds`

Each item uses `Option` plus a `Data` payload containing:

- grid owner transform
- grid component
- mode value

The settings strip appears only for the selected grid. It projects `Origin` for
a legacy grid without a mode component.

Selecting an option must:

1. find the selected grid owner
2. add `GridSnapModeComponent` if it is absent, or update it if present
3. emit the existing grids-changed event
4. rerender the grid row with the chosen option selected

Object-level overrides remain MMS-authorable only in this version. Adding the
same control to the ordinary transform inspector is a possible follow-up.

## Tests and acceptance criteria

### Component and resolution tests

- MMS parses and round-trips `GridSnapMode.origin()` and
  `GridSnapMode.bounds()`.
- An object mode overrides its selected grid's mode.
- A grid mode is used when the object has no override.
- Missing modes resolve to `Origin`.
- Missing or unmeasurable bounds fall back to origin snapping.

### Snap-math tests

- 1-unit and 3-unit centered cubes retain integer grid-local boundaries in
  bounds mode.
- A non-multiple-sized object aligns its nearest edge without rejection.
- Equal-distance edge selection is deterministic.
- Translated and rotated grids produce the same coordinates in rendering and
  snapping.
- Grid-X and grid-Z translations preserve the unrelated coordinate.
- Grid-normal translation preserves both in-plane coordinates.
- Oblique translation stays on the selected handle axis.

### Editor integration tests

- Moving a `voxel_terrain` cube along grid X or Z keeps its edges on visible
  grid lines.
- The first gizmo movement does not introduce a half-cell jump.
- Paint snaps on an ordinary terrain hit using the selected grid.
- Grid and object modes both affect paint preview and committed placement.
- Paint preview and commit use exactly the same final pose.
- Surface height, normal, and contact offset survive lateral paint snapping.
- The selected grid row shows the settings strip and current effective mode.
- Changing the selection creates or updates the component and survives MMS
  serialization.

Retain
`voxel_terrain_cube_xz_boundaries_land_on_whole_local_units` as the authored
terrain checkpoint and add integration coverage proving gizmo snapping
preserves that phase.

## Out of scope

- rotation or scale snapping
- editor UI for object-level overrides
- curved-surface paint reprojection
- implementation of planar gizmo translation handles

Planar translation handles are specified separately in
`docs/task/transform-gizmo-planar-translation-handles.md`.

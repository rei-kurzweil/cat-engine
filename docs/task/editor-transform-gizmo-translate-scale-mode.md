# Editor transform gizmo translate/scale mode

Date: 2026-07-29

Status: implementation task

Related:

- `docs/task/editor-settings-panel.md`
- `docs/task/transform-gizmo-planar-translation-handles.md`
- `docs/task/unified-grid-snap-mode-mms-gizmo-and-paint.md`
- `docs/spec/editor-gizmo-coord-spaces.md`
- `docs/spec/grid-snapping.md`

## Goal

Add an editor-scoped transform-gizmo setting with two mutually exclusive
choices:

- `translate + rotate`
- `scale + rotate`

Rotation rings remain present in both modes. Changing the setting updates every
live transform gizmo in that editor without rebuilding the editor or the
selected scene object.

When an active grid is selected, scale-handle drags must snap through the grid
policy while remaining constrained to the selected scale axis.

For now, regenerate the complete arm subtree when the mode changes. Do not
attempt to mutate cone heads into square heads in place. The arm subtree should
have an explicit stable slot whose contents can be removed and recreated.

## Existing component status

The gizmo operations already have distinct ECS marker components:

- `TransformGizmoTranslateComponent` marks an X, Y, or Z translation handle.
- `TransformGizmoTranslatePlaneComponent` marks an XY, YZ, or XZ planar
  translation handle.
- `TransformGizmoRotateComponent` marks an X, Y, or Z rotation ring.
- `TransformGizmoScaleComponent` marks an X, Y, or Z scale handle.

The single-axis translation handles therefore already have their own ECS
components. `TransformGizmoScaleComponent` is public and registered with MMS as
`TransformGizmoScale.x()`, `.y()`, and `.z()`.

Scale is not currently exposed by the editor gizmo visual builder. The gizmo
system can resolve and execute a scale operation when it encounters a scale
marker, but it does not spawn any raycastable scale-handle subtree. This task
adds that missing editor-facing path and makes its interaction contract safe
for regular use.

## State and serialization

Add an editor-scoped enum:

```rust
enum TransformGizmoHandleMode {
    TranslateRotate,
    ScaleRotate,
}
```

Store it on `EditorComponent` as `transform_gizmo_handle_mode`.
`TranslateRotate` is the default so existing scenes retain their current
behavior and appearance.

Expose the setting through the editor MMS API:

```mms
Editor {
    gizmo_handle_mode("translate_rotate")
}
```

The accepted values are:

- `"translate_rotate"`
- `"scale_rotate"`

The field must round-trip through MMS serialization. Missing values in legacy
content resolve to `TranslateRotate`.

## Editor settings UI

Add a `Gizmo Handles` row to the editor settings panel. It contains a
single-selection, horizontal pair of options:

- `translate + rotate`
- `scale + rotate`

Use a dedicated `Selection` root for this setting rather than sharing the
interaction-mode selection that currently owns `Select`, `3D Cursor`, and
`Select + Cursor`.

Each option carries a `Data` payload containing:

- the owning editor component
- row kind `TransformGizmoHandleMode`
- the selected mode value

Selecting an option must:

1. update `EditorComponent.transform_gizmo_handle_mode`
2. update the settings-panel selection state
3. cancel an active gizmo arm drag, if one exists
4. refresh the arm subtree of every live gizmo scoped to that editor

The setting is editor-scoped. Changing one editor must not alter gizmos owned
by another editor subtree.

## Gizmo visual topology

Refactor the generated visual subtree to provide stable rotation and arm slots:

```text
gizmo overlay
├── rotation slot
│   └── rotation coordinate-space pipeline
│       ├── X ring
│       ├── Y ring
│       └── Z ring
└── arms slot
    └── active arm coordinate-space pipeline
        └── mode-specific arm handles
```

The slots are stable, non-renderable, non-raycastable nodes. Their generated
children are runtime-owned and excluded from scene serialization.

The rotation slot is not touched by a handle-mode change. This ensures that
rotation rings remain visible and do not churn merely because translation and
scale arms were exchanged.

The arms slot is the refresh boundary. A refresh removes every child subtree
under the slot, then creates and initializes one new arm-space pipeline and its
mode-specific handles.

This slot design should also be reusable when translation coordinate space
changes: regenerate the arm-space pipeline beneath the slot using the current
mode and coordinate-space setting.

## Mode-specific handles

### Translate + rotate

Populate the arms slot with the existing translation controls:

- red X arrow with cone head
- green Y arrow with cone head
- blue Z arrow with cone head
- yellow XY planar handle
- cyan YZ planar handle
- magenta XZ planar handle

Each handle retains its existing independent drag-only raycast root and ECS
operation marker.

Translation arms use
`EditorComponent.transform_gizmo_translation_space`.

### Scale + rotate

Populate the arms slot with three axis scale controls:

- red X stem with compact cube head
- green Y stem with compact cube head
- blue Z stem with compact cube head

Each scale arm has:

- one `TransformGizmoScaleComponent` for its axis
- one independent drag-only raycast root
- a stem and visibly square/cubic head

Do not show the planar translation handles in scale mode. A center uniform-scale
handle and planar scale handles are out of scope for this version.

Scale handles use the target's local axes. This task does not introduce a
separate scale coordinate-space editor setting.

## Scale interaction contract

Because this task makes scale handles reachable in the editor, their drag
behavior must be made stable rather than relying on accumulated frame deltas.

On drag start:

- capture the target scale
- capture the pointer hit in world space
- capture the selected local scale axis in world space

During drag:

- derive the candidate scale from the drag-start state
- project pointer movement onto the captured axis
- change only the selected scale component
- preserve the other two scale components exactly
- enforce the existing nonzero minimum without producing NaN or infinity

Repeated delivery of the same drag update must produce the same scale. Parent
rotation and scale must not make the visible handle disagree with the applied
scale axis.

Desktop and XR pointers use the same constrained scale calculation after their
pointer movement reaches the gizmo.

## Grid snapping for scale

Scale handles must participate in snapping whenever the editor has a selected
active grid. Do not run a scale candidate through the existing point-translation
helper: scale snapping changes a dimension, not the target translation.

Add scale as an operation constraint to the shared grid-snap request/result
path. The request carries:

- the selected active-grid frame and spacing
- the target pose captured at drag start
- the candidate scale derived from the current pointer
- the selected local scale axis
- aggregate rendered-subtree bounds when measurable

Scale snapping remains a one-scalar operation. The snap result may change only
the selected target scale component. It must preserve:

- target translation
- target rotation
- the other two scale components
- the selected handle's local-axis constraint

Use the grid spacing as the dimensional step along all three grid-local axes.
For scaling, grid-local Y uses the same spacing even though the visible grid
lines lie in grid-local XZ. This allows height to snap to cell-sized increments
when scaling along the grid normal.

When usable aggregate bounds are available:

1. transform the candidate bounds into selected-grid-local space
2. determine which grid-local axis is most affected by the selected scale axis
3. choose the positive, dragged-side bound extreme for that grid coordinate
4. snap that extreme to its nearest grid-spacing line
5. solve the required correction by changing only the selected scale scalar

For an oblique relationship, snap only the single grid coordinate most affected
by the scale handle. Do not alter another scale component or translate the
object in an attempt to satisfy multiple grid coordinates.

If the selected scale component has no meaningful effect on the chosen grid
coordinate, leave the candidate unsnapped. If aggregate bounds cannot be
measured, fall back to quantizing the captured handle-axis displacement in
grid-spacing increments rather than inventing bounds or moving the target.

The snapped scale must still respect the nonzero minimum. Ties between grid
lines use the same deterministic rounding rule as other grid operations.
Repeated evaluation of the same pointer position must return the same snapped
scale.

This requirement explicitly extends the unified grid-snap work, where rotation
and scale snapping were originally out of scope. Rotation snapping remains out
of scope; only the axis scale handles introduced by this task are added to the
shared request path.

## Arm refresh lifecycle

Provide one gizmo-system entry point such as:

```rust
refresh_transform_gizmo_arms(world, gizmo, mode, emit)
```

The refresh must:

1. verify that the gizmo and its stable arms slot still exist
2. clear active arm-drag state and remove any debug drag plane
3. remove all existing children of the arms slot through normal subtree cleanup
4. build the correct arm-space pipeline for the requested mode
5. spawn exactly one set of mode-specific handle roots
6. initialize the new subtree exactly once

Normal cleanup is required so removed arm renderables leave the renderer, BVH,
raycast registry, and gesture routing. After the signal drain that performs the
refresh, old arms must not remain visible or hittable.

If the editor owns more than one live gizmo, enumerate and refresh all of them.
Do not assume that only the currently attached shared gizmo exists.

For this version, rebuilding the arm subtree is preferred over retaining and
editing individual stems or heads. Finer partial-tree update semantics can be a
later refactor.

## Tests and acceptance criteria

### Component and state

- The default editor mode is `TranslateRotate`.
- Both mode values parse and round-trip through MMS.
- The setting is isolated between separate editor roots.
- Existing translation, planar translation, rotation, and scale marker
  components remain independently resolvable.

### Settings panel

- The panel shows a `Gizmo Handles` row with the two requested options.
- Exactly one option is selected.
- Selecting an option updates the owning `EditorComponent`.
- The displayed selection remains synchronized after panel rerender.

### Visual refresh

- Translate mode shows three cone-headed translation arms and three planar
  translation handles.
- Scale mode shows three cube-headed scale arms and no planar translation
  handles.
- X, Y, and Z retain red, green, and blue colors in both modes.
- The three rotation rings remain present across either mode change.
- Repeated toggles never duplicate handles, raycastables, or renderables.
- Removed handles are no longer raycastable after the refresh drain.
- Switching mode while dragging cancels the old drag without mutating the
  target again.
- Every live gizmo under the affected editor refreshes.

### Interaction

- Each scale arm routes to its intended `TransformGizmoScaleComponent`.
- Dragging a scale arm changes exactly one scale coordinate.
- Scale drags use drag-start state and do not accumulate drift.
- Scale handles follow target-local axes under rotated targets and parents.
- Translation behavior, planar translation behavior, and rotation behavior are
  unchanged after toggling back to translate mode.

### Grid snapping

- With no selected active grid, scale dragging remains continuous.
- With a selected active grid, all X, Y, and Z scale arms use grid spacing.
- A grid-aligned scale axis snaps the dragged-side aggregate bound to a grid
  line without changing target translation.
- Scaling along the grid normal uses grid spacing as a height increment.
- An oblique scale axis snaps at most one, most-affected grid coordinate.
- Snapping never changes either unselected scale component.
- Missing bounds use quantized handle-axis displacement as the fallback.
- Rotated grids, rotated targets, and parent transforms produce the same
  grid-local snapped dimensions as their unrotated equivalents.
- Repeated drag updates at the same pointer position do not accumulate scale or
  alternate between snap results.

## Out of scope

- center uniform-scale handle
- planar two-axis scale handles
- negative scale or axis-flip UX
- a separate world/local scale-space setting
- preserving individual arm nodes between modes
- generalized keyed reconciliation for arbitrary component subtrees

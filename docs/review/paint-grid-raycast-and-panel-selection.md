# Paint-grid raycasting and grid-panel selection review

Date: 2026-09-05

Status: implemented; focused tests and `cargo check` pass

## Scope and invariants

This note explains the main functions added to let a Paint stroke begin directly on a managed grid
and to let an explicit grid-panel selection show a transform gizmo in Paint or 3D Cursor mode.

The implementation preserves three important boundaries:

- grid hit surfaces are in the raycast BVH only while the editor is in Paint mode;
- grid hit surfaces use `PointerEvents::DragOnly` and remain non-selectable, so they can start a
  paint drag without consuming ordinary clicks;
- selecting a grid in the grid panel may expose its gizmo, but does not change the editor's current
  interaction mode.

The relevant code is concentrated in
[`grid_system.rs`](../../src/engine/ecs/system/grid_system.rs),
[`editor_paint_system.rs`](../../src/engine/ecs/system/editor_paint_system.rs),
[`editor/context.rs`](../../src/engine/ecs/system/editor/context.rs),
[`editor_system.rs`](../../src/engine/ecs/system/editor_system.rs),
[`gizmo_system.rs`](../../src/engine/ecs/system/gizmo_system.rs), and
[`system_world.rs`](../../src/engine/ecs/system/system_world.rs).

## Runtime grid structure

An authored grid is represented by a `GridComponent` beneath an owner `TransformComponent`. The
grid system builds a non-serialized runtime subtree beside that authored component:

```text
owner TransformComponent
|-- GridComponent                         authored grid state
`-- grid_live_root: TransformComponent    runtime-only subtree
    |-- grid_live_selectable: SelectableComponent::off()
    |-- grid_live_serialize: SerializeComponent::off()
    `-- grid_live_shape: TransformComponent
        `-- grid_live_renderable: RenderableComponent
            |-- visual properties
            `-- grid_live_raycastable: RaycastableComponent
```

`grid_live_shape` is a very thin cube covering the finite grid rectangle. Its child raycastable is
created disabled and with `PointerEvents::DragOnly`; Paint-mode synchronization controls whether it
is registered with the raycast acceleration structure.

The authored `GridComponent::selectable` flag is intentionally not part of Paint hit eligibility.
Selection policy and Paint start-surface policy are separate: a non-selectable grid may still be a
valid surface on which to paint.

## How grid-panel selection reaches the gizmo

The selection call chain is:

```text
grid panel SelectionChanged
        |
        v
EditorContextEvent::GridPanelSelectionChanged
        |
        +--> reducer records active_grid_owner_transform
        |
        v
apply_semantic_target_selection_from_grid_panel(...)
        |
        v
apply_semantic_target_selection_with_policy(..., show_gizmo_in_paint = true)
        |
        +--> nearest_editor_ancestor(target)
        +--> nearest_transform_ancestor(target)
        |
        v
select_editor_target_from_panel(...)
        |
        v
select_editor_target_with_gizmo_policy(..., show_gizmo_in_paint = true)
```

### `apply_semantic_target_selection_from_grid_panel`

This is a named policy entry point. It delegates all work to
`apply_semantic_target_selection_with_policy` and passes `show_gizmo_in_paint = true`.

Its purpose is to make the exceptional caller obvious. The ordinary
`apply_semantic_target_selection` function calls the same implementation with `false`, so normal
scene and semantic selection retain their existing Paint behavior.

### `apply_semantic_target_selection_with_policy`

This function converts an arbitrary semantic target into the two identities needed by the editor:

1. `nearest_editor_ancestor` finds which editor owns the target.
2. `nearest_transform_ancestor` finds the transform to which the shared gizmo can attach.
3. If the target is not the editor root and both identities exist, the function invokes either the
   normal selection path or the panel-specific selection path.
4. It records the original semantic target in `EditorContextState::selected_component` and updates
   `active_editor` when one was found.
5. It returns `SemanticEditorSelectionResult`, retaining the resolved editor, gizmo target, and
   whether the full editor-selection path ran.

The distinction between `target_component` and `gizmo_target` matters. A panel payload can identify
a component below a transform, while the transform gizmo must attach to a transform ancestor.

The lower-level `select_editor_target_with_gizmo_policy` ensures and attaches the shared gizmo when
either the editor is not in Paint mode or this explicit panel policy is enabled. It then updates
`EditorComponent::selected`, emits the normal `SelectionChanged` event, and optionally updates the
REPL working component. It never changes `EditorInteractionMode`, so Paint remains Paint and 3D
Cursor remains 3D Cursor.

Afterward, `sync_editor_observer_routes` continues to route scene input according to the unchanged
mode. `sync_gizmo_interaction_mode` keeps the gizmo enabled in Paint only when
`active_grid_owner_transform == selected_component`. `TransformGizmoSystem::gizmo_input_suppressed`
uses the same rule, allowing the visible handles to receive drags while continuing to suppress
ordinary Paint-mode gizmo input.

That equality is currently a compact proxy for “this grid was explicitly selected in the grid
panel”: only the grid-panel event writes `active_grid_owner_transform`. If another feature starts
writing that field, this should become an explicit selection-source flag or enum instead of relying
on the equality invariant.

## How grids enter and leave the raycast BVH

### `GridSystem::sync_paint_raycast_targets`

This function reconciles all managed grids with the current Paint-mode policy:

```text
desired raycast state = paint_mode && grid.enabled && !grid.hidden
```

It first calls `ensure_registry_current`, then snapshots and refreshes every `GridEntry`. For each
entry it locates `#grid_live_raycastable` under the owner transform, forces its pointer policy to
`DragOnly`, and sets its `enable` field to the desired state.

When the state or pointer policy actually changes, it emits one of:

- `RegisterRaycastable` when the surface becomes eligible;
- `RemoveRaycastable` when the surface becomes ineligible.

The change check makes the reconciliation idempotent. Calling it again with unchanged state emits
no duplicate BVH work.

`SystemWorld` invokes this reconciliation every frame after the initial queued-world flush and
then flushes its registration intents before later raycasting work. This covers mode changes as
well as grids created, deleted, hidden, or enabled while Paint is already active. The Paint side
effects also invoke it immediately after processing tool activation, closing the transition window
before the next pointer activation.

Leaving Paint produces the inverse reconciliation and removes every managed grid hit surface from
the BVH. The grid renderables remain available for drawing; only their raycast participation is
changed.

## How `grid_hit_context_for_renderable` works

Raycast and drag events identify the runtime renderable that was hit, not its authored
`GridComponent`. `GridSystem::grid_hit_context_for_renderable` performs the inverse mapping:

```text
hit renderable
    -> walk parents to component labelled grid_live_root
    -> find that runtime root's authored owner transform
    -> find the GridEntry owned by that transform
    -> reject disabled or hidden grids
    -> build ActiveGrid from current authored state and world transform
```

The steps are split across small helpers:

- `grid_owner_from_renderable` walks upward from the supplied renderable until it finds the
  `grid_live_root` boundary, then resolves its owner transform;
- `grid_component_for_renderable` maps that owner through the grid registry to the authored
  `GridComponent`;
- `grid_entry` refreshes cached flags from the live component;
- `active_grid_from_entry` computes the world matrix, inverse matrix, world-space origin and
  normal, and clamps spacing to a safe minimum.

The function returns `None` for unrelated renderables, stale or malformed runtime trees, missing
registry entries, hidden/disabled grids, non-invertible transforms, or degenerate grid normals.
Otherwise it returns an `ActiveGrid` containing enough live geometry to interpret the hit.

## Why the actual hit grid wins at stroke start

`resolve_paint_context` still starts from editor and panel state and may therefore contain the
previously selected grid. During `PaintEvent::StrokeStarted`, the Paint system now asks
`grid_hit_context_for_renderable` whether the captured renderable belongs to a grid. If it does, it
immediately converts the `ActiveGrid` to a `CapturedGrid` and replaces
`context.selected_grid` for that stroke.

```text
StrokeStarted(renderable, hit_point)
        |
        +--> resolve normal PaintContext
        |
        +--> identify grid belonging to renderable
        |        `--> capture its transform, dimensions, and spacing
        |
        `--> use captured hit grid for addressing, snapping, preview, and gesture state
```

This override is local to the new stroke. It does not mutate editor selection or silently switch
the active grid in the panel. Capturing also stabilizes the grid frame: later edits to the grid's
transform or spacing do not change the coordinate system halfway through that gesture.

The BVH supplies the initial world-space `hit_point`; `GridSystem::address_for_point` transforms it
through the captured inverse matrix to derive the finite grid cell address. The current change does
not replace drag continuation with a separate analytic ray/plane intersection. If strokes must
continue after the pointer ray leaves all raycast geometry, that is a distinct follow-up involving
the gesture-to-Paint event data and captured plane mapping.

## Behavior summary

| Situation | Grid in BVH | Grid consumes clicks | Gizmo behavior |
| --- | --- | --- | --- |
| Select mode | No | No | Normal selection policy |
| 3D Cursor mode | No | No | Grid-panel selection may show gizmo; mode stays 3D Cursor |
| Paint mode, visible enabled grid | Yes | No (`DragOnly`) | Hidden unless grid was selected from grid panel |
| Paint mode, hidden or disabled grid | No | No | Not a valid Paint hit surface |
| Leaving Paint | Removed | No | Normal policy for the destination mode |

## Focused coverage

The implementation adds focused tests for:

- enabling all visible, enabled live grids in Paint, ignoring the authored selectable flag,
  excluding hidden grids, avoiding duplicate registration intents, and removing surfaces on exit;
- selecting a grid from the panel in Paint and 3D Cursor modes without changing the mode, while
  attaching/exposing the gizmo;
- suppressing Paint-mode gizmo input except for the explicit grid-panel target.

At the time of this review, those focused tests and `cargo check` pass. The full library run records
789 passing, 43 failing, and 1 ignored test; the failures are outside this focused change and are not
claimed as resolved here.

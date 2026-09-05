# Paint as a first-class editor interaction mode

Date: 2026-09-05

Status: proposed direction; seam audit complete, implementation not started.

Related:

- [Free Draw cannot start a stroke on an empty grid's analytic plane](../bugs/free-draw-cannot-start-on-empty-grid-analytic-plane.md)
- [Grid-aware paint stroke interaction model](grid-aware-paint-stroke-interaction-model.md)
- [Grid Tool and surface-placement follow-ups](grid-tool-and-surface-placement-followups.md)
- [Paint panel selection and panel focus](paint-panel-selection-and-panel-focus.md)
- [Shared editor UI routing and Paint state manager](shared-editor-ui-routing-and-paint-state-manager.md)

## Decision

Promote Paint to a fourth, mutually exclusive editor interaction mode alongside
`Select`, `3D Cursor`, and `Select + Cursor`:

```rust
pub enum EditorInteractionMode {
    Select,
    Cursor3d,
    SelectAndCursor,
    Paint,
}
```

`Paint` owns scene pointer interpretation for every tool exposed by the Paint
panel: Free Draw, Grid Tool, Line, Spray Can, Color, Erase, and Fill when Fill
is implemented. A paint tool chooses what a Paint gesture does; it is not a
separate substitute for the workspace interaction mode.

For the initial implementation, Paint and transform selection are exclusive.
While `EditorInteractionMode::Paint` is active:

- scene clicks and drags must not select or retarget objects;
- neither the Select nor 3D Cursor handler may consume scene gestures;
- transform gizmos must be hidden and non-raycastable;
- stale gizmo state must not be draggable even if a selection existed before
  entering Paint;
- Paint receives the eligible scene/grid gesture and owns preview, commit, and
  cancellation;
- a selected component may remain in editor state for the world tree or
  inspector, but it must not expose an interactive gizmo.

Multi-selection, painting a selected set, and editing a transform while a paint
stroke is active are intentionally deferred. They require an explicit
selection/tool arbitration model rather than overlapping handlers.

## Entry and exit contract

- Choosing any Paint-panel tool enters `EditorInteractionMode::Paint` for the
  active editor before the next scene pointer activation is interpreted.
- The Settings panel exposes a visible `Paint` row with payload value `paint`,
  so the mode can also be chosen directly and its current state is visible.
- Focusing the Paint panel while a valid tool is already selected may enter
  Paint, preserving the current convenience behavior.
- Merely moving focus to another panel does not silently restore an older
  interaction mode. The mode remains Paint until the user explicitly chooses
  Select, 3D Cursor, or Select + Cursor.
- Choosing another editor mode cancels and rolls back an in-progress Paint
  stroke before enabling that mode's handlers.
- Changing Paint tools during a stroke also cancels the current stroke before
  starting with the new tool.

There is no implicit “previous mode” stack in the MVP. That avoids restoring a
stale mode after panel focus or active-editor changes.

## Why this is needed

Paint currently behaves as a panel-focus exception layered over the other
editor modes. `force_cursor_mode_for_paint_activation(...)` changes the editor
to `Cursor3d`, while `sync_editor_observer_routes(...)` independently enables
the Paint handler when the Paint or Color panel is focused. Consequently the
mode shown in Settings, the handlers allowed to observe the gesture, and the
tool that ultimately interprets it can disagree.

An explicit Paint mode gives one authority for:

- selection suppression;
- cursor suppression;
- gizmo visibility and hit testing;
- Paint handler routing;
- active-grid raycast/analytic-plane eligibility; and
- cancellation when ownership changes.

## Seam audit

### 1. Core mode model and serialization

- `src/engine/ecs/component/editor.rs`
  - add `EditorInteractionMode::Paint`;
  - serialize it as `Editor.interaction_mode("paint")`;
  - keep `Select` as the default.
- `src/scripting/component_registry.rs`
  - parse `"paint"` instead of falling through to `Select`;
  - add focused parsing/round-trip coverage.
- `src/scripting/runtime_config.rs`
  - confirm the registered `Editor.interaction_mode` method accepts the new
    value through the configured runtime path.

### 2. Settings panel UI and payload mapping

- `assets/components/internal/panels.mms`
  - add a fourth `editor_settings_mode_row`, labelled `Paint`, with
    `mode_value = "paint"`.
- `src/engine/ecs/system/editor/settings_panel.rs`
  - add `EditorSettingsOption::Paint` and its row-name constant;
  - update `interaction_mode()`, `row_name()`, and `from_mode_value()`;
  - update selection synchronization so Paint is visibly selected;
  - cover click-to-mode and mode-to-selected-row behavior.

### 3. Shared editor context and active-editor propagation

- `src/engine/ecs/system/editor/context.rs`
  - allow `Paint` through every reducer/event synchronization path;
  - keep the shared context and all registered `EditorComponent` roots in
    agreement;
  - define active-editor changes during Paint without restoring a different
    editor root's stale mode;
  - update mode-routing tests and the global interaction-mode synchronization
    tests.

### 4. Observer routing

- `sync_editor_observer_routes(...)` currently enables Paint from
  `paint_focused` and independently blacklists Select/Cursor by mode.
- Replace focus-based scene ownership with a single mode table:

| Mode | Select handler | Cursor handler | Paint handler | Gizmo input |
| --- | --- | --- | --- | --- |
| Select | enabled | disabled | disabled | enabled |
| 3D Cursor | disabled | enabled | disabled | disabled |
| Select + Cursor | enabled | enabled | disabled | enabled |
| Paint | disabled | disabled | enabled | disabled |

Panel focus may affect panel UI, but it must not be the final authority for
scene gesture routing.

### 5. Paint activation and activity predicates

- `src/engine/ecs/system/editor_paint_system.rs`
  - replace `force_cursor_mode_for_paint_activation(...)` with a transition to
    `EditorInteractionMode::Paint`;
  - perform the transition when a tool is chosen, and on Paint focus when a
    valid tool already exists;
  - roll back the active `PaintStrokeRuntime` on mode exit, editor change,
    invalid tool/asset state, lost pointer, or grid disable/delete.
- `src/engine/ecs/system/editor/paint_panel.rs`
  - make `is_paint_active(...)` require Paint mode plus valid tool/asset state;
  - remove panel focus as the fundamental authorization for scene painting;
  - update status text to distinguish “switch to Paint mode” from missing tool,
    asset, grid, or color requirements.

The Color panel may remain part of Paint's UI workspace, but focusing it must
not create a second definition of whether scene painting is active.

### 6. Scene selection and 3D Cursor

- `src/engine/ecs/system/editor_system.rs`
  - add explicit Paint branches to both scoped and global click/drag-start
    handlers;
  - Paint branches do not call `select_editor_target(...)` or update REPL cwd.
- `src/engine/ecs/system/cursor_3d.rs`
  - Paint must not update or show the 3D cursor merely because it uses the same
    surface-placement math;
  - share placement helpers as data/functions, not by masquerading as
    `Cursor3d` mode.

### 7. Gizmo lifecycle and hit testing

- `src/engine/ecs/system/editor/context.rs` owns the shared workspace gizmo and
  its anchor.
- `src/engine/ecs/system/gizmo_system.rs` installs drag handlers directly under
  the gizmo and does not currently participate in the named Select/Cursor
  router blacklist.
- On entry to Paint:
  - hide the shared gizmo visual;
  - disable or pass through every gizmo-handle raycastable;
  - cancel any active gizmo drag;
  - prevent reattachment/retargeting from editor or world-panel selection.
- On exit from Paint, restore visibility and hit testing only when the selected
  component is still a valid gizmo target for the newly chosen mode.
- Add defensive Paint-mode guards in gizmo drag start/move handling so stale
  BVH data cannot mutate a transform during a transition frame.

Disabling only the selection handler is insufficient: a previously visible
gizmo can still win raycast arbitration and capture the drag intended for
Paint.

### 8. Grid raycasting and analytic-plane start

- `src/engine/ecs/system/grid_system.rs` already creates a finite live grid
  renderable, but its `RaycastableComponent` and `SelectableComponent` are
  forced off.
- Paint mode is the correct policy boundary for making the selected, enabled,
  visible grid eligible for Paint drag initiation without making it generally
  selectable.
- Reuse the existing live grid renderable as the BVH broad-phase candidate;
  use `GridSystem::intersect_captured_grid_plane(...)` for the exact finite
  plane hit and continued stroke projection.
- Outside Paint mode, restore the grid visual to its non-raycastable behavior.
- Ordinary scene geometry still wins when it is genuinely nearer than the
  analytic grid plane.

This work must not add another hidden renderable or transform solely to catch
Paint clicks.

### 9. Gesture ownership and frame ordering

- `src/engine/ecs/system/system_world.rs` runs BVH, raycast, gesture reduction,
  and signal drains in a fixed order.
- Ensure a tool/mode selection signal is fully reduced and observer/grid/gizmo
  routing is synchronized before the following pointer press is sampled.
- Mode exit during an active pointer press produces one cancellation, not a
  Paint commit followed by a Select/3D Cursor action.
- Desktop and XR use the same mode ownership even if their drag continuation
  mappings differ.

### 10. Tests and release checks

Add coverage for:

- all four enum/string/settings-row round trips;
- choosing each Paint tool enters Paint;
- Paint remains active across unrelated panel focus changes;
- entering Paint preserves optional inspector selection but hides and disables
  the gizmo;
- scene click and drag in Paint never retarget selection or move a transform;
- switching out of Paint restores only the newly selected mode's handlers;
- a mode switch during a preview rolls it back;
- Free Draw starts on an empty selected grid through the existing grid visual
  plus analytic finite-plane hit;
- nearer scene geometry beats the grid plane;
- desktop mouse and XR trigger follow the same ownership rules; and
- multiple editor roots do not receive duplicate Paint gestures.

## Implementation sequence

1. Add and round-trip the `Paint` enum/string/settings option.
2. Change context observer routing to the explicit four-mode table.
3. Replace the Cursor3d activation shim and mode-gate Paint activity.
4. Suppress/hide gizmos and add defensive mode guards.
5. Enable the selected grid's existing drag candidate only in Paint and wire
   exact analytic-plane start/continuation.
6. Add transition, desktop/XR, multi-editor, and regression coverage.

## Acceptance criteria

- Settings visibly offers Select, 3D Cursor, Select + Cursor, and Paint as peer
  interaction modes.
- Selecting any Paint tool enters Paint before the next scene gesture.
- In Paint, scene input cannot select a transform, attach/retarget a gizmo, or
  manipulate a stale gizmo.
- In Paint, each available Paint tool receives the gesture semantics it owns.
- An empty active grid can initiate a Paint drag without adding hidden scene
  transforms or renderables.
- Leaving Paint is explicit and safely cancels transient Paint state.
- Non-Paint modes retain their current selection and cursor semantics.

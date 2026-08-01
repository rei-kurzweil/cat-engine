# Editor panel minimize and render suspension

Date: 2026-08-01

Status: active pre-0.8 performance task

## Goal

Add a minimize control to editor panel title bars. A minimized panel should
retain only the smallest useful panel shell—its outer container, title bar,
title, and title-bar controls—while its body is absent from rendering,
layout, hit testing, and body-specific refresh work.

This is both a usability feature and a measurable UI-performance experiment.
It should help distinguish the cost of having many panel bodies alive from
the separate selection-driven world/inspector refresh problem.

## Design decision

Treat the visible widget as a reusable MMS accordion/disclosure component, not
as a new engine component primitive.

- Add `assets/components/ui/accordion.mms` for the reusable shell topology,
  title-bar layout, minimize control, per-instance state, and click handler.
- Let `accordion.mms` own the expanded/minimized state machine and physical
  removal of its current body.
- Use callbacks in the accordion options table as the integration seam. Pure
  MMS callers can repopulate directly; editor-panel adapters notify the editor
  runtime to project current model data into the new body.
- Keep model state, dirty-refresh coalescing, and dynamic body projection in
  the existing panel runtime/controllers.
- Use subtree removal, not detach or visibility styling, as the suspension
  operation.

Although "disclosure" is the more precise name for one independently
collapsible section, `accordion.mms` is a useful familiar component name. The
first version is a single-section accordion and does not impose exclusive-open
behavior on sibling panels. A future accordion group can add that policy
without changing the panel/body lifecycle contract.

This division is intentional. MMS is the right layer for both reusable
authored UI structure and the widget's local interaction state. The editor
runtime participates only where the accordion cannot know how panel-specific
data is produced. It should not become an `AccordionComponent` built into the
engine.

## Concrete component sketch

The exact MMS syntax can be adjusted during implementation, but the authored
contract should look like this:

```javascript
import { accordion } from "./ui/accordion.mms"

export fn world_panel_body(working_file_path) {
    return T {
        name = "accordion_body"
        T {
            name = "world_panel_body"
            // path input, content area/slot, selection, and status
        }
    }
}

export fn world_panel(title, items, title_color, panel_color, item_color, path) {
    let title_controls = T {
        name = "panel_title_controls"
        panel_button("save_button", "Save")
        panel_button("load_button", "Load")
    }

    // These are ordinary MMS closures. They adapt the generic accordion
    // callbacks to editor-runtime signals; they do not own world-panel data.
    let on_minimized = fn(panel_root) {
        emit_data(panel_root, "AccordionMinimized")
    }
    let restore_body = fn(panel_root, body_mount) {
        emit_data(panel_root, "AccordionRestoreRequested", body_mount)
    }

    return accordion({
        root_name = "world_panel_root"
        width_gu = WORLD_PANEL_WIDTH_GU
        title = title
        title_color = title_color
        background_color = panel_color
        title_controls = title_controls
        body = world_panel_body(path)
        on_minimized = on_minimized
        restore_body = restore_body
    })
}
```

`accordion(...)` should author this stable topology:

```text
<panel root: stable, draggable through its title bar, focus/select target>
├── Style(width, inline/block layout; no fixed expanded height)
├── title_bar                                  retained
│   ├── title_label                            retained
│   ├── caller title controls                  retained
│   └── accordion_toggle                       retained
│       ├── Data(action = "ToggleMinimized")
│       └── accordion_toggle_label ("−" / "+")
└── accordion_body_mount                       retained, no visual/style work
    └── accordion_body                          removed while minimized
        └── <panel-specific body root/content>
```

The factory owns the toggle so every caller gets the same hit target, spacing,
label, and payload. Callers supply their existing title controls: Save/Load,
Pin, grid visibility, and any future Close button. The factory must reserve a
fixed-width control cell so title text cannot wrap over controls.

The root must derive its expanded height from its children rather than retain
the current fixed total height. The expanded title/body gap belongs to the body
wrapper, not `title_bar`, so removing the body also removes the gap. Width and
the panel root transform remain stable across both states.

The accordion factory registers `on(accordion_toggle, "Click", ...)` inside
the factory. Its handler captures a heap-backed state table containing
`minimized` and `restore_pending`, so state changes persist across handler
invocations. It resolves the current standardized body under its private mount
rather than retaining a removed `ComponentId`. It updates
`accordion_toggle_label` itself.

The options-table callbacks are command-style callbacks:

- `on_minimized(panel_root)` runs after the body removal is requested. It is
  optional and is used by editor integration to forget renderer slots and mark
  the panel minimized.
- `restore_body(panel_root, body_mount)` is responsible for repopulating the
  stable mount. A pure MMS caller can synchronously create and attach a new
  body. An editor-panel adapter emits `AccordionRestoreRequested`; the editor
  runtime materializes the body and projects the newest domain model.

The callback does not return data to the accordion. This keeps asynchronous
host-backed restoration possible and makes ownership clear: the callback must
attach exactly one new body beneath the supplied mount. The accordion rejects
or ignores another restore click while restoration is pending.

### Small MMS/runtime capability needed

MMS already supports the important parts of this design:

- functions can receive an options table containing `Value::Function` values;
- a factory can install `on(...)` handlers scoped to its per-instance toggle;
- closures can capture heap-backed objects for persistent mutable state; and
- live component handles can attach newly materialized children.

With the current call evaluator, the factory should first bind callback fields
to local identifiers (`let restore_body = options.restore_body`) and then call
`restore_body(...)`; direct invocation syntax such as
`options.restore_body(...)` is currently interpreted as a component/object
method call rather than a function-valued table field.

One topology operation is missing. Add a component method such as
`body_root.remove_subtree()` that emits `IntentValue::RemoveSubtree` for that
exact live component. Do not implement collapse with the existing
`remove_child()` or `detach()`: both preserve the subtree as a live world root.
This is a small general MMS lifecycle method, not an accordion engine
primitive.

Rust functions cannot currently be represented directly as MMS `Value`s.
Therefore the editor bootstrap cannot literally construct a native Rust
closure and put it in the options table without adding a new callback ABI.
The smaller design is for `panels.mms` to pass MMS closures that adapt callback
invocation to `DataEvent`s, as sketched above. Existing Rust panel handlers can
consume those signals. If native host callbacks become a general scripting
feature later, they can replace the adapters without changing the accordion
options contract.

### Body factory contract

Each panel must expose its non-title portion as a separately materializable MMS
factory, such as `world_panel_body`, `inspector_panel_body`, or
`paint_panel_body`. The existing full-panel export remains a convenient
composer for MMS callers and examples, while the runtime uses the body export
to restore a removed body.

Body factories must satisfy these rules:

- return exactly one outer root named `accordion_body` (panel-specific named
  roots may live immediately below it);
- contain every non-title visual and interactive child, including status bars,
  toolbars outside the title, path inputs, selections, and dynamic render slots;
- contain no durable editor/model state that cannot be recovered before
  removal;
- be safe to materialize repeatedly; and
- install only subtree-scoped handlers, or rely on the single stable
  editor/panel runtime handlers.

`accordion_body_mount` stays as a plain named transform so the runtime has a
stable restore target. It must not carry `Style`, `Raycastable`, `Selection`, or
renderable descendants of its own.

## Runtime sketch

Extend the existing panel runtime representations rather than introduce a
parallel minimization system:

```rust
enum PanelControlKind {
    // existing controls...
    MinimizeButton,
    MinimizeLabel,
}

struct PanelBodySpec {
    asset_path: String,
    export_name: String,
    root_selector: String,
}

struct PanelInstance {
    // existing identity, root, slots, and controls
    body_mount: ComponentId,
    body_root: Option<ComponentId>,
    body_spec: PanelBodySpec,
    restore_pending: bool,
    dirty_while_minimized: bool,
}
```

`body_root == None` is the runtime's projection of the accordion's minimized
state; do not maintain a second independently toggled `minimized` boolean in
Rust. `restore_pending` covers the interval after the MMS callback requests a
restore and before the editor has attached the replacement body. The
illustrative dirty bit is sufficient for the first slice. If a panel later
needs independently refreshable regions, replace it with flags or a model
generation number without changing the accordion API.

Panel controllers need a narrow lifecycle seam:

```rust
trait PanelBodyLifecycle {
    fn body_args(...);      // build restore args from the current model
    fn refresh_body(...);   // resolve new slots and project the newest model
    fn on_body_removed(...);// forget renderer/controller topology bookkeeping
    fn on_remove(...);      // optional controller bookkeeping cleanup
}
```

This need not literally be a Rust trait; a match on `PanelKind` or registered
callbacks is acceptable. The important boundary is that `accordion.mms` owns
the widget state machine, `panel_system` owns the mounted editor-panel
bookkeeping, and controllers own semantic state.

### Current editor integration points

The current bootstrap path is already close to the required callback target:

- `panel_system::spawn_editor_panel_layout_tree` materializes the MMS panel
  shells;
- `editor::panel_bootstrap::reconcile_editor_panel_layout` attaches the shared
  mount and performs initial world/grid/settings/assets projection;
- `EditorWorkspaceRuntime::resolve_and_cache_static_panels` records the seven
  static panel roots; and
- `editor::inspector_panel::rerender_inspector_panels` separately creates and
  refreshes inspector instances.

Install the accordion `DataEvent` adapters at this existing seam. Static panels
route restore requests through their cached `PanelInstance`; inspector restore
requests route through the same body-lifecycle helper used by its dynamic
instances. The stopgap adapter may still dispatch to those functions during
the refactor, but it should not contain accordion-specific behavior.

### State transitions

Expanded to minimized:

1. The MMS toggle handler ignores the click if a transition is pending.
2. It resolves the current standardized `#accordion_body` below its own body
   mount and calls `remove_subtree()` on that exact handle.
3. It updates its heap-backed state and toggle label, then calls
   `options.on_minimized(panel_root)` if supplied.
4. The editor adapter emits `AccordionMinimized`. The panel runtime receives
   that event, sets `body_root = None`, forgets `DataRendererSystem` tracking
   for the old dynamic slots, and dirties the containing layout once. Add a
   renderer `forget_slot`/`forget_subtree_slots` API for this: calling today's
   `clear_slot` after removing the ancestor could queue an overlapping subtree
   removal.
5. Focus remains on the stable panel root/title shell.

Minimized invalidation:

1. Continue updating the panel's domain model.
2. If a refresh targets a minimized panel, set `dirty_while_minimized = true`
   and return before building UI items, materializing MMS, resolving body
   selectors, or calling `DataRendererSystem`.
3. Repeated invalidations coalesce into that one pending refresh.

Minimized to expanded:

1. The MMS toggle handler marks restoration pending and calls
   `options.restore_body(panel_root, body_mount)`.
2. For editor panels, that closure emits `AccordionRestoreRequested`. The
   runtime asks the controller for body arguments from its current model,
   materializes the standardized `#accordion_body`, and attaches it beneath
   the supplied mount.
3. The runtime resolves all body-owned slots and controls again; it never
   reuses removed `ComponentId`s. It calls `refresh_body` exactly once from the
   newest controller/model state, even if the dirty bit is false because the
   body itself is new.
4. The runtime stores the new body/slot ids, clears the dirty bit, dirties
   layout once, and emits `AccordionBodyRestored` on the stable panel root.
5. A second handler installed by `accordion.mms` consumes that acknowledgement,
   clears its pending state, verifies `#accordion_body` now exists beneath its
   own mount, and changes the toggle label to the expanded form. A restore
   failure emits `AccordionRestoreFailed` so the widget can clear pending state
   and remain minimized.

A pure MMS caller follows the same callback contract: its `restore_body`
closure creates and attaches `#accordion_body`, then emits
`AccordionBodyRestored`. Thus the state machine remains wholly inside the
component even though the editor's body producer happens to live in Rust.

Panel removal:

1. Remove the stable panel root regardless of minimized state.
2. Drop its `PanelInstance`, body spec, renderer slot registrations, and pending
   dirty state.
3. Do not attempt restoration from a late invalidation after removal.

`RemoveSubtree` is important rather than merely convenient. Current `Detach`
only removes the parent edge and turns the body into a world root; it does not
by itself unregister renderables, raycasts, transforms, or scoped handlers.
The existing subtree-removal path unregisters system state, deletes the ECS
nodes, removes handlers scoped within the deleted subtree, and clears removed
text-input focus.

## State ownership decisions

- Minimized state is transient editor UI state for the first implementation.
  Do not serialize it into scene MMS.
- Panel root transform, width, focus identity, and title controls remain live.
- Expanded height is reconstructed from the restored body. If panel resizing
  is added later, store the expanded size in `PanelInstance`, not in the body.
- Domain state stays outside the removable body: world/inspector selection,
  paint tool/color, settings values, grid state, pose state, and asset model.
- Body-local presentation state such as scroll offset may reset on restore in
  v1. This should be documented in the UI behavior, not accidentally treated
  as semantic state.
- Editable values are not presentation state. In particular, the world path
  input and any active inspector field must update controller state as their
  normal change events occur, rather than living only in the removable text
  widget. This removes the need for a synchronous Rust `before_suspend`
  callback and ensures minimizing cannot discard user input.

## Existing panel inventory and migration shape

All current full editor panels are authored in `assets/components/panels.mms`
and use a node named `title_bar`, but their bodies are not normalized yet.

| Panel | Title controls retained | Body to extract |
| --- | --- | --- |
| Settings | minimize | mode and visibility rows, selections |
| Paint | minimize | tool rows, tool selection, status |
| Color | minimize | swatches and color selection |
| Pose capture | minimize | pose controls/list/status |
| World | Save, Load, minimize | path input, scroll/list selection, status |
| Inspector | Pin, minimize | sidebar selection and detail view |
| Assets | minimize | asset content and selection |
| Grid | visibility control, minimize | grid list selection and Add Grid |

World currently places its path input before the title bar; extracting the
body also normalizes the title bar to be the stable first visible child. The
inspector is mounted/refreshed differently from the seven cached static panels,
so it must use the same `PanelInstance` body lifecycle rather than gaining a
second custom minimize path.

The representative first panel should be Color or Settings: both exercise the
shared shell without entangling the initial topology test with world/inspector
refresh complexity. World and Inspector remain the performance-validation
slice after the lifecycle is proven.

## Minimum automated contract

For the representative panel, test the MMS-owned transition and runtime
handshake without relying only on screenshots:

1. Record body descendant, renderable, raycastable, selection, and scoped
   handler counts while expanded.
2. Minimize and assert the body root and all recorded descendants no longer
   exist, while the panel root, title bar, controls, drag behavior, and panel
   focus target still exist.
3. Send several model invalidations and assert zero body materializations and
   zero renderer calls, with one dirty state recorded.
4. Restore and assert one body materialization and one controller refresh using
   the newest model value.
5. Repeat at least 100 minimize/restore cycles and assert stable expanded and
   minimized node/handler counts.
6. Remove the panel while minimized, send a late invalidation, and assert no
   body is recreated.

## Required behavior

- [ ] Add a consistent minimize/restore button to the generic editor panel
      title bar rather than implementing one button per panel.
- [ ] Keep the minimized title bar draggable and focusable using the existing
      panel behavior.
- [ ] Preserve panel placement, size/state needed for restoration, and the
      panel's semantic model.
- [ ] Remove or suspend the body subtree while minimized; clipping, opacity,
      zero scale, or covering it visually is not sufficient.
- [ ] Ensure minimized body renderables cannot draw, receive pointer hits,
      participate in selection, or cause descendant layout work.
- [ ] Ensure body-specific invalidations do not rebuild an invisible body.
      Record one dirty state if needed and perform one refresh on restore.
- [ ] Restore the same panel body and current model state without duplicate
      handlers, stale descendants, or accumulated topology.
- [ ] Make the control usable for all panels using the common panel/title-bar
      construction path.

The exact retained ECS nodes should be decided from the implementation, not
from appearance alone. The intended visible result is the title-bar container;
supporting transform, style, text, button, drag, and focus components may
naturally remain.

## Investigation before implementation

- [ ] Inventory the common title-bar construction and every editor panel that
      bypasses it.
- [ ] Identify which body state can live outside the materialized subtree and
      which state must be reconstructed.
- [ ] Trace whether detaching a body removes it from rendering, layout,
      raycasting, signal routing, and editor refresh registration, or whether
      explicit cleanup is required.
- [ ] Decide whether minimization is transient UI state, serialized editor
      state, or both. Default to transient unless persistence already has a
      natural home.
- [ ] Define panel removal while minimized so the suspended body cannot leak.

## Implementation slices

### Slice 1: one representative panel

- [ ] Add generic minimized state and a title-bar toggle.
- [ ] Exercise it on one inexpensive representative panel.
- [ ] Verify the body subtree count and renderable count fall when minimized.
- [ ] Verify repeated minimize/restore cycles do not grow handlers or nodes.

### Slice 2: world and inspector panels

- [ ] Apply the shared behavior to the world and inspector panels.
- [ ] Suppress their body rerender paths while minimized.
- [ ] Restore from the newest selection/model state with one rebuild.
- [ ] Compare selection and gizmo responsiveness with both panels expanded
      and minimized.

### Slice 3: complete panel coverage

- [ ] Cover remaining editor panels, including any custom title-bar paths.
- [ ] Normalize title-bar spacing and prevent the new control from wrapping or
      obscuring existing controls.
- [ ] Add automated coverage for state transitions and topology cleanup.

## Performance evidence

Capture the same scene and interaction sequence before and after the change:

- panel/body descendant count;
- active renderable and raycastable count;
- layout work attributable to panel descendants;
- world/inspector refresh count;
- selection-to-visible-update duration; and
- frame-time behavior with several minimized versus expanded panels.

Minimizing must produce a real reduction in active work. If selection remains
slow with the expensive panels minimized, continue the narrower refresh-path
investigation rather than considering this task a complete performance fix.

## Acceptance criteria

- [ ] Every supported editor panel can minimize and restore from its title
      bar.
- [ ] Only title-bar shell presentation remains active while minimized.
- [ ] Hidden body content does no render, layout, hit-test, or eager rerender
      work.
- [ ] Current state is shown after restore and no handlers or ECS subtrees
      leak across repeated cycles.
- [ ] The performance delta is recorded using the same reproducible scene.
- [ ] Existing panel dragging, focus, close behavior, selection, and layout
      still work.

## Related documents

- [Panel system](panel-system.md)
- [World/inspector panel selection refresh slowness](editor-panel-selection-refresh-perf-investigation.md)
- [Editor UI rerender audit and clean reducer boundary](editor-ui-rerender-audit-and-clean-reducer-boundary.md)
- [Mittens MMS ownership cutover and 0.8 release](mittens-mms-ownership-cutover-and-0.8-release.md)

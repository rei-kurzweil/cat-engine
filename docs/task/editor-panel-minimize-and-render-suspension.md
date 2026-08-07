# Editor panel minimize and render suspension

Date: 2026-08-01

Status: implemented foundation; Asset/World restoration regressions gate 0.8.0

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
- Use an `AccordionRestoreRequested` `DataEvent` as the integration seam. MMS
  and native owners can both respond by repopulating the stable body mount.
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
    return accordion({
        root_name = "world_panel_root"
        width_gu = WORLD_PANEL_WIDTH_GU
        unit_scale = 1.0
        background_color = panel_color
        children = [
            panel_title(title, title_color),
            panel_button("save_button", "Save"),
            panel_button("load_button", "Load"),
        ]
        body = world_panel_body(path)
    })
}
```

`accordion(...)` should author this stable topology:

```text
<layout slot: shared-layout placement and margins>
└── <panel transform: stable drag offset and focus/select target>
    └── private LayoutRoot(width, unit_scale)
        ├── title_bar                          retained
        │   ├── accordion_toggle               retained, always leftmost
        │   ├── caller title children, in order retained
        │   └── accordion_toggle_icon          retained disclosure glyph
        └── accordion_body_mount               retained, no visual/style work
            └── accordion_body                 removed while minimized
                └── <panel-specific body root/content>
```

The factory owns the toggle so every caller gets the same hit target, spacing,
animated disclosure icon, and payload. The icon uses a 0.6-beat ease-out
transform transition (300 ms at the default 120 BPM). Its seamless disclosure
glyph is the named authored polygon `ui/accordion/down-chevron/v1`, rather
than overlapping cube bars; the polygon cache gives it stable mesh identity
and avoids z-fighting. Editor title-bar runtime integration remains a separate
follow-up. Callers supply their existing title controls: Save/Load,
Pin, grid visibility, and any future Close button. The factory must reserve a
fixed-width control cell so title text cannot wrap over controls.

The root must derive its expanded height from its children rather than retain
the current fixed total height. The expanded title/body gap belongs to the body
wrapper, not `title_bar`, so removing the body also removes the gap. The title
text slot and standardized body wrapper each provide `1gu` padding on all
sides. Width and the panel root transform remain stable across both states.

The accordion factory registers `on(accordion_toggle, "Click", ...)` inside
the factory. It derives state from topology by querying for its standardized
body under the private mount rather than retaining a second state machine or a
removed `ComponentId`.

- If the body exists, the handler calls `remove_subtree()`, rotates its
  disclosure icon to the collapsed/opposite-facing state, and emits
  `AccordionMinimized` for renderer bookkeeping.
- If the body is absent, the handler rotates its own disclosure icon to the
  expanded/down-pointing state and emits `AccordionRestoreRequested` with the
  stable body mount as the optional component payload. The event is its only
  external side effect; the icon change is private widget presentation.

The accordion does not listen for `DataEvent`, wait for an acknowledgement, or
interpret restoration failures. The MMS or Rust owner of the content responds
to the request and attaches exactly one new `#accordion_body` beneath the
mount. The owner never reaches into the accordion's private icon state.

### Small MMS/runtime capability needed

MMS already supports the important parts of this design: a factory can install
an `on(...)` handler scoped to its per-instance toggle, query beneath its own
live component handles, emit arbitrary named `DataEvent`s, and let an MMS
consumer attach newly materialized children.

One topology operation is missing. Add a component method such as
`body_root.remove_subtree()` that emits `IntentValue::RemoveSubtree` for that
exact live component. Do not implement collapse with the existing
`remove_child()` or `detach()`: both preserve the subtree as a live world root.
This is a small general MMS lifecycle method, not an accordion engine
primitive.

No native-callback ABI is required. Rust registers a normal `DataEvent` handler
at the panel/editor scope and reads the event's optional component payload to
obtain `accordion_body_mount`. An MMS consumer receives the event name and can
capture or query its accordion's body mount directly.

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
    dirty_while_minimized: bool,
}
```

`body_root == None` is the runtime's projection of the accordion's minimized
state; do not maintain a second independently toggled `minimized` boolean in
Rust. The illustrative dirty bit is sufficient for the first slice. If a panel
later needs independently refreshable regions, replace it with flags or a
model generation number without changing the accordion API.

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

Install the accordion `DataEvent` responders at this existing seam. Static panels
route restore requests through their cached `PanelInstance`; inspector restore
requests route through the same body-lifecycle helper used by its dynamic
instances. The stopgap adapter may still dispatch to those functions during
the refactor, but it should not contain accordion-specific behavior.

### State transitions

Expanded to minimized:

1. The MMS toggle handler resolves the current standardized `#accordion_body` below its own body
   mount and calls `remove_subtree()` on that exact handle.
2. It rotates its disclosure icon to the collapsed state and emits
   `AccordionMinimized` with the body mount as its component payload.
3. The panel runtime receives
   that event, sets `body_root = None`, forgets `DataRendererSystem` tracking
   for the old dynamic slots, and dirties the containing layout once. Add a
   renderer `forget_slot`/`forget_subtree_slots` API for this: calling today's
   `clear_slot` after removing the ancestor could queue an overlapping subtree
   removal.
4. Focus remains on the stable panel root/title shell.

Minimized invalidation:

1. Continue updating the panel's domain model.
2. If a refresh targets a minimized panel, set `dirty_while_minimized = true`
   and return before building UI items, materializing MMS, resolving body
   selectors, or calling `DataRendererSystem`.
3. Repeated invalidations coalesce into that one pending refresh.

Minimized to expanded:

1. The MMS toggle handler finds no `#accordion_body`, rotates its private
   disclosure icon to the expanded state, and emits
   `AccordionRestoreRequested` with the stable mount as its component payload.
   It performs no restoration work.
2. The runtime asks the controller for body arguments from its current model,
   materializes the standardized `#accordion_body`, and attaches it beneath
   the supplied mount.
3. The runtime resolves all body-owned slots and controls again; it never
   reuses removed `ComponentId`s. It calls `refresh_body` exactly once from the
   newest controller/model state, even if the dirty bit is false because the
   body itself is new.
4. The runtime stores the new body/slot ids, clears the dirty bit, and dirties
   layout once. It neither updates the accordion icon nor sends an
   acknowledgement back to the accordion.

A pure MMS owner listens for `AccordionRestoreRequested`, captures or queries
the appropriate mount, creates `#accordion_body`, and attaches it. The same
one-way event contract therefore covers MMS and native responders.

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

## Open 0.8 restoration regression

The initial minimize path landed, but restoration is not release-ready:

- restoring the Asset panel can fail to render its body content; and
- restoring the World or Asset panel can create or enlarge a bright
  emissive-yellow background quad far beyond the intended panel bounds.

Track reproduction, diagnosis, and focused acceptance coverage in
[Accordion panel restoration loses content and corrupts backgrounds](../bugs/accordion-panel-restore-content-and-background-corruption.md).
The implementation status above does not mark the restoration lifecycle or
the original repeated-cycle acceptance contract complete.

## Related documents

- [Panel system](panel-system.md)
- [World/inspector panel selection refresh slowness](editor-panel-selection-refresh-perf-investigation.md)
- [Editor UI rerender audit and clean reducer boundary](editor-ui-rerender-audit-and-clean-reducer-boundary.md)
- [Mittens 0.8 and 0.9 release roadmap](release-roadmap-0.8.0-0.9.0.md)

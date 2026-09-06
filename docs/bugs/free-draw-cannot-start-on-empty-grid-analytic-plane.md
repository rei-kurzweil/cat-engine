# Free Draw cannot start a stroke on an empty grid's analytic plane

## Status: still failing after attempted fix

2026-09-05: the user revalidated empty-grid Free Draw after the recent grid
raycast/BVH participation changes and reports that it still does not start.
Keep this issue open. This is the highest current interaction priority in the
[desktop meta tracker](../desktop/interaction-priorities.md), ahead of the grab
and attachment epic. The report is user validation; the source audit below is
not a fresh interactive reproduction or a confirmed complete root cause.

## Follow-up validation: toggling does not repair stroke startup

The user explicitly disabled/re-enabled the grid in the running
`examples/paint-grids-desktop.mms` example and retried: drags still require one
of the backing squares. Presses on the exposed grid edges do not start drawing.
The authored-grid lifecycle finding below is therefore **not a sufficient fix
for this bug**. Track initial visibility separately as priority 2 in
[the existing visibility issue](default-grid-visibility-ui-state-out-of-sync.md).
Empty-grid stroke startup remains priority 1.

The live diagnostic log now contains corroborating input evidence:

- `/tmp/mittens-empty-grid-investigation.log`, lines 942, 967, 984, 999, 1565,
  and 1588: gesture presses report `drag_hit=None click_hit=None hits=<none>`.
- Successful Paint runtime starts (for example lines 1611 and 3699) capture
  backing renderables `ComponentId(13v1)` and `ComponentId(9v1)` and report
  `actual_paint_grid=(none)` despite selected grid owner `ComponentId(20v1)`.
- These IDs/line numbers belong to this session. The log does not label each
  press with its UI action or record grid registration/BVH membership, so the
  user's report supplies the toggle/edge-click context.

This narrows the observed failure to before `DragStart`, rather than a Paint
preview that receives a valid grid start and then fails. Next inspect the live
mode/eligibility, marker enable state, BVH entry/bounds, and raycast filtering
for the visible, toggled grid. A successful lifecycle-only probe does not prove
any of those later stages. Include this post-toggle case in regression coverage.

## Summary

`Free Draw` can project an already-active stroke onto a grid, but it cannot begin that stroke by
clicking or pressing the trigger on an otherwise empty part of the grid. A stroke only starts when
the initial `DragStart` ray hits an existing renderable, such as an object painted by an earlier
stroke. This makes painting an empty grid depend on prior painted scene geometry.

## Reproduction

1. Open an editor scene with a visible grid and select `Free Draw` plus a paintable asset.
2. Ensure there are no renderables over the part of the grid to be painted.
3. Click and drag with the desktop pointer, or press and move the XR trigger, beginning on that
   empty grid area.
4. Observe that no paint stroke begins.
5. Paint or otherwise place a renderable on the grid, then begin the drag on that renderable.
6. Observe that painting starts and can continue across the grid while the pointer/trigger remains
   held.

## Current behavior

- The grid's analytic-plane intersection is available for continuation of a captured paint drag.
- It does not provide a hit that can initiate the initial `DragStart` on an empty grid area.
- The initial press therefore needs a raycastable scene renderable under it; an existing painted
  object happens to satisfy that requirement.
- Releasing the pointer/trigger ends the stroke as expected, but the next stroke has the same
  renderable-at-start requirement.

## Expected behavior

- While the editor is in Paint mode, users can begin a Free Draw stroke anywhere inside any
  enabled, visible grid's finite bounds, even when no scene renderables exist there.
- The same grid plane should supply both the initial stroke point and later drag-projection points.
- Starting a paint stroke from a grid surface must not select, move, rotate, or otherwise treat the
  grid as a gizmo target.
- Selecting a grid explicitly from `grid_panel` is a separate command path: it selects the grid's
  owning transform and attaches the normal transform gizmo even when the editor is currently in
  Paint or 3D Cursor mode. It does not make ordinary grid-surface paint hits select the grid.
- Outside the finite grid bounds, normal no-surface behavior should remain unchanged.

## Source audit after the failed validation

The attempted registration fix is present, but registration alone has not
established the end-to-end behavior:

- [GridSystem](../../src/engine/ecs/system/grid_system.rs)
  `sync_paint_raycast_targets` enables `DragOnly` and emits registration/removal
  intents for each managed live grid when `paint_mode && enabled && !hidden`.
  This is Paint-mode plus grid eligibility, not merely selected-grid activation.
- [SystemWorld](../../src/engine/ecs/system/system_world.rs) calls that sync
  and flushes its intents before the later BVH/raycast/gesture work.
- The existing grid live renderable is a thin `CUBE` with a small visual Y
  offset. [RaycastSystem](../../src/engine/ecs/system/raycast_system.rs) infers
  `Box` for that mesh. This is a box hit path, not the exact logical grid plane.
- `GridSystem::intersect_captured_grid_plane` exists, but a source-wide caller
  search finds only its definition and unit-test calls. The proposed exact
  finite-plane integration is not wired into production hit resolution.
- [GestureSystem](../../src/engine/ecs/system/gesture_system.rs) starts a drag
  only from the pointer's existing hits whose pointer policy captures drags.
  With no qualifying hit, the press is skipped before `DragStart` is emitted.
- [EditorPaintSystem](../../src/engine/ecs/system/editor_paint_system.rs) can
  recognize a grid renderable and capture its grid when handling `StrokeStarted`.
  That downstream recognition cannot create a missing initial gesture hit.
- `paint_mode_raycast_sync_tracks_all_visible_live_grids_and_is_idempotent`
  checks enabled state and emitted intents. It does not prove a populated BVH,
  successful raycast, gesture delivery, or a painted item on an empty grid.

### Confirmed authored-grid lifecycle failure in the supplied example

At revision `faecddc7`, a headless probe evaluated the exact
`examples/paint-grids-desktop.mms` file into a live `World` using
`MeowMeowRunner::eval_with_world_at_path`, then called
`GridSystem::sync_paint_raycast_targets(..., true)`. Evaluation succeeded.
The authored `debug_vertical_grid_spacing_0_5` grid was enabled and not hidden,
but Paint synchronization left both live runtime and raycast marker absent:

```text
grid=ComponentId(21v1) owner=Some("debug_vertical_grid_spacing_0_5") enabled=true hidden=false
after Paint sync: live root=None, raycast marker=None
after off/on and Paint sync: live root=Some(ComponentId(29v1)), marker enabled=true
```

The last line is a controlled comparison: the probe disabled and re-enabled
that same grid through `set_grid_enabled`, then synchronized Paint again.
Component IDs are run-specific. Probe source and executable for this session:
`/tmp/mittens-grid-lifecycle-probe.rs` and `/tmp/mittens-grid-lifecycle-probe`.
It linked against the release library built by the requested example launch;
no engine source modifications were needed.

The failure chain is concrete:

1. MMS `Grid` construction adds a `GridComponent`; its component implementation
   does not initialize a live renderable subtree.
2. Registry discovery finds authored grids but does not materialize that subtree.
3. `sync_paint_raycast_targets` searches for `#grid_live_raycastable` and silently
   continues when it is absent. It never calls `ensure_live_runtime`.
4. Live runtime creation currently occurs through `spawn_grid_for_editor` or
   changed enabled/hidden state. Calling `set_grid_enabled(true)` on an already
   enabled grid returns early and does not repair it.
5. Therefore the freshly authored grid has no candidate to register or hit.
   A ray hitting one of the example's three wall renderables can still start
   a gesture; an empty part of the logical grid cannot use that fallback.

This confirms a missing-candidate cause for the freshly loaded authored grid.
It does not prove that all failures after a grid has been toggled or created
through the editor have the same cause. In particular, a visible live grid
that already has a raycast marker needs the remaining hit/routing trace.
The exact finite analytic-plane helper still has no production callers; that
is an additional integration gap, not the cause of the missing subtree.

The registration regression test uses `spawn_grid_for_editor`, so its grids
already have live runtime. It cannot catch the authored-grid lifecycle failure.

### Live launch and validation limits

The requested launch reached the window/render loop successfully with existing
diagnostics enabled:

```sh
MITTENS_DEBUG_PAINT_STROKE=1 CAT_DEBUG_GESTURE=1 CAT_DEBUG_RAYCAST=1 \
  cargo run --release -- load examples/paint-grids-desktop.mms
```

The session log is `/tmp/mittens-empty-grid-investigation.log`. The initial audit captured only the headless lifecycle result. Subsequent user
interaction produced the failed-press and background-stroke traces recorded
above; no successful empty-grid stroke or XR verification has been captured.
The example is desktop-only and includes backing walls: compare a ray within
the grid bounds but outside those walls against one hitting a wall.

### Proposed repair and focused regression

Make managed authored/loaded grids establish their normal live runtime before
Paint eligibility synchronization attempts registration. Use the same lifecycle
as editor-created grids, preserving enabled/hidden policy and idempotence.
Do not create a second invisible catch surface or require users to toggle a
grid to initialize it. Keep creation/removal and registration synchronized
before the next BVH/raycast pass.

First regression: load the supplied MMS through the ordinary load path, with
no manual grid toggle or editor-spawn helper, enter Paint, and verify that its
live renderable and enabled drag marker exist. Then execute a ray/gesture on
an empty finite-grid area and verify Paint starts. Add a comparison for an
editor-created grid and for disable/re-enable and mode exit/re-entry.
After the lifecycle fix, connect and validate the exact finite-plane test;
the thin box's visual offset must not define the logical paint plane.

### Next investigation, in priority order

1. Follow up the confirmed authored-grid lifecycle failure with a full interactive
   reproduction in a scene with an enabled, visible finite grid and no backing
   geometry. Record scene/revision, Paint mode, selected tool/asset, grid IDs,
   pointer ray, and whether desktop and XR both fail.
2. Trace mode/eligibility → registration intent application → live renderable
   bounds and BVH membership → raycast hit list → gesture `drag_hit` → Paint
   `StrokeStarted` → preview/commit. Identify the first missing stage.
3. Compare an empty-grid press with a press on existing scene geometry using
   [paint-stroke diagnostics](../how_to/paint-stroke-live-diagnostics.md).
   No stroke trace alone cannot distinguish no activation from no eligible hit;
   inspect the gesture press hit list and BVH membership as well.
4. Wire the finite analytic-plane exact test into initial candidate resolution
   using the existing grid live renderable as broad phase, respecting bounds
   and nearer eligible surfaces. Do not add an invisible catch surface.
5. Add an integration regression that executes registration, BVH, raycast,
   gesture, and Paint startup on an empty grid rather than injecting a prebuilt
   `DragStart`. Revalidate interactively before marking the issue fixed.

## Required registration and routing contract

Paint is now intended to be a first-class `EditorInteractionMode`, mutually exclusive with Select,
3D Cursor, and Select + Cursor. Use entering and exiting that mode as the policy boundary for grid
raycast/BVH participation:

- on entry to Paint mode, consult `GridSystem`, iterate every enabled, visible live grid it manages,
  and register each grid's existing live renderable as a raycast/BVH broad-phase candidate;
- while Paint mode remains active, keep that membership synchronized when grids are created,
  deleted, enabled, disabled, shown, or hidden;
- on exit from Paint mode, consult `GridSystem` again and unregister every managed grid renderable
  that was made raycastable for Paint;
- make the transition idempotent so repeated mode synchronization cannot duplicate registration or
  leave stale BVH entries;
- use the existing finite analytic-plane helper for the exact hit, grid choice, and continued
  projection after broad-phase candidate discovery; and
- keep grid paint surfaces non-selectable. A grid-surface hit initiates Paint; it is not a scene
  selection or transform-gizmo hit.

The `grid_panel` selection action is an explicit exception to mode-based scene selection routing.
It should resolve and select the grid's owning transform and attach the normal transform gizmo
without first forcing the editor into Select mode. This applies in Paint and 3D Cursor modes as
well as Select modes. Only a deliberate panel selection or a dedicated gizmo handle may target the
grid transform; clicking or dragging the grid's BVH/analytic paint surface must never do so.

Do not add an invisible renderable or transform solely to catch the initial pointer activation. See
[Paint as a first-class editor interaction mode](../task/paint-as-first-class-editor-interaction-mode.md)
for the complete routing and mode-transition work. That broader task's blanket rule that Paint
hides the gizmo needs to preserve the explicit `grid_panel` selection exception documented here.

## Acceptance criteria

- Fresh empty grids accept an initial desktop click-drag and XR trigger-drag for Free Draw while
  Paint mode is active.
- The first painted item is placed at the grid-plane intersection, using the same grid coordinate
  and snapping rules as subsequent stroke points.
- A continuous stroke remains constrained/projected to the grid while held.
- Entering Paint registers all enabled, visible managed grids for raycast/BVH broad-phase testing;
  exiting Paint removes that Paint-only participation from every managed grid.
- Creating, deleting, enabling, disabling, showing, or hiding a grid while Paint is active updates
  BVH participation without leaving a stale or duplicate entry.
- Outside Paint mode, grid renderables do not participate in raycasting merely because they are
  visible or managed by `GridSystem`.
- Grid interaction does not regress into the known behavior where a grid drag arms a gizmo or
  monopolizes selection.
- Selecting a grid row selects its owning transform and attaches the gizmo without changing an
  existing Paint or 3D Cursor interaction mode.
- A Paint gesture cannot retarget or manipulate the grid gizmo unless its initial hit is a dedicated
  gizmo handle.
- Regression tests cover a `DragStart` whose ray intersects a managed grid's existing live
  renderable but no other raycastable scene renderable, Paint entry/exit with multiple grids, and
  grid-panel selection while Paint and 3D Cursor modes are active.

## Related trackers

- [Free Draw paint placement does not snap to grid while Grid Tool placement does](./free-draw-paint-does-not-snap-to-grid-while-grid-tool-placement-does.md)
- [Grid tool can leave a grid as the only selectable target, and dragging the grid rotates the gizmo](./grid-tool-leaves-grid-as-only-selectable-target-and-grid-drags-rotate-gizmo.md)
- [Editor 3D Cursor GLTF Coverage and Grid Alignment](./editor-cursor-3d-gltf-and-grid-alignment.md)
- [Paint as a first-class editor interaction mode](../task/paint-as-first-class-editor-interaction-mode.md)

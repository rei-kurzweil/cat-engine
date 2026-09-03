# Selecting side-scene boxes stalls vtuber-desktop

## Status

Open performance investigation.

## Symptom

Selecting some decorative boxes positioned to the side of
`examples/vtuber-desktop.mms` with Select or Select-and-Cursor causes a
multi-second visible stall. Animation and interaction pause, indicating that
the main thread is blocked rather than merely routing selection incorrectly.

The repro is not specific to the side boxes: it now occurs on every ordinary
scene selection in this example. `Select` and `Select + Cursor` both stall for
roughly five seconds; `3D Cursor` by itself does not. This separates the
selection path from cursor placement/raycasting alone.

## Captured trace

The following is a representative trace from a `Select + Cursor` click:

```text
🐈 /20v1:/534v1:editor_auto_raycastable/33v1:
🧲🛠🐛 select_editor_target called editor_root=ComponentId(20v1)
  target_transform=ComponentId(39v1) mode=SelectAndCursor update_repl_cwd=true

[EditorContext][trace] world_panel selection_root=ComponentId(1194v1)
  clicked_row=Some(ComponentId(21427v9)) payload=Some(ComponentId(18762v9))
  authored_target=Some(ComponentId(33v1)) active_editor=Some(ComponentId(20v1))

🐈 /20v1:/538v1:editor_auto_raycastable/39v1:
```

The `world_panel` trace is important: it is an automatic UI synchronization
following the scene selection, not evidence that the initial ray hit was a
World-panel row. The large, generation-tagged component IDs also show that the
runtime UI has materialized/recycled a large component population by this
point; selection should not rebuild that population synchronously.

## Repro

1. Run `cargo run --release --example vtuber-desktop`.
2. Activate Select or Select-and-Cursor mode.
3. Click several decorative cube groups at either side of the scene.
4. Observe that some selections stall the application for several seconds.

## Expected behavior

Scene-object selection and gizmo placement should complete within normal
interactive frame time, without visibly pausing the scene.

## Notes

This may be a focused manifestation of the existing
`docs/task/editor_selection_and_paint_perf.md` investigation, which already
records main-thread stalls while selecting renderables nested under an editor.
This report pins down a reproducible scene and target class before treating it
as the same root cause.

## Initial investigation targets

- `select_editor_target(...)` and shared gizmo attachment in
  `src/engine/ecs/system/editor_system.rs`.
- `SelectionChanged` downstream work: inspector refresh, world-panel refresh,
  signal/command processing, and runtime UI topology churn.
- Whether the affected targets resolve to authored transforms or generated
  editor/raycast wrappers.

## Confirmed selection path and ranked suspects

`Cursor3d` does nothing in the editor selection-mode switch. `Select` and
`SelectAndCursor` both call `select_editor_target(...)`, which currently:

1. scans all world components to find the shared gizmo (or creates/registers it
   on the first selection);
2. queues an `Attach` of that gizmo to the selected transform;
3. emits `SelectionChanged` for the editor root;
4. queues `ReplExec("cd <guid>")` and `ReplExec("pwd")` because
   `update_repl_cwd=true`;
5. has an editor-panel refresh handler which synchronizes the World-panel row
   selection and unconditionally calls `sync_and_refresh_inspector_panels(...)`.

That World-panel synchronization emits the `SelectionChanged` responsible for
the captured `[EditorContext][trace]` line. The inspector refresh then builds
inspector models and rerenders their dynamic UI tree through `DataRenderer`.

The leading current suspect is therefore **inspector panel rebuild / dynamic
UI materialization on every scene selection**, with World-panel selection sync
as the event that makes it visible. The gizmo lookup is also unnecessarily
linear in all components and must be timed, but a scan of roughly 20k IDs is a
weaker explanation for a repeatable five-second stall. REPL CWD updates are a
third independent path that must be measured rather than assumed cheap.

This is not yet proof of the dominant phase. The trace alone does not tell us
whether the time is spent before the World-panel event, while handling it, or
when queued topology/REPL work is flushed.

## Comparison: `bisket-desktop-demo`

This comparison strongly supports the World-panel/topology hypothesis. In
`examples/bisket-desktop-demo.mms`, the large animated cube/background scene
is authored outside `ED.active()`. The editor-owned subtree is primarily the
single `desktop_scene_bisket` GLTF branch. In `vtuber-desktop.mms`, the
decorative scene objects and the avatar are inside `ED` blocks.

The World-panel model recursively enumerates only children of editor roots.
Objects outside `ED` can still be selected through the active editor's global
scene-selection handler, but they are not rows in that model. Crucially, a
gizmo reparent to an outside-editor target also should not schedule the
editor-owned topology refresh that rebuilds World-panel content.

That explains why selecting thousands of outside-`ED` Bisket scene cubes can
be cheap while selecting a much smaller editor-owned VTuber subtree is slow:
the relevant cost is likely **editor membership and resulting UI/topology
work**, not raw renderable or cube count. It remains a hypothesis until one
measurement records both row count and which refreshes ran for each click.

Required comparison measurements:

```text
[perf][select] scene=bisket target=<id> inside_editor=false world_panel_rows=<n> topology_refresh=false
[perf][select] scene=vtuber target=<id> inside_editor=true  world_panel_rows=<n> topology_refresh=true
```

## Required measurements

Add per-phase timing for the selected component id:

```text
[perf][select] target=<id> phase=gizmo_attach dt_ms=...
[perf][select] target=<id> phase=inspector_refresh dt_ms=...
[perf][select] target=<id> phase=world_panel_sync dt_ms=...
[perf][select] target=<id> phase=selection_total dt_ms=...
```

For this repro, split the existing suggested timings further:

```text
[perf][select] target=<id> phase=find_shared_gizmo dt_ms=...
[perf][select] target=<id> phase=queue_gizmo_attach dt_ms=...
[perf][select] target=<id> phase=editor_selection_handlers dt_ms=...
[perf][select] target=<id> phase=world_panel_sync dt_ms=...
[perf][select] target=<id> phase=inspector_model_build dt_ms=...
[perf][select] target=<id> phase=inspector_render dt_ms=...
[perf][select] target=<id> phase=repl_cwd_queue_and_exec dt_ms=...
[perf][select] target=<id> phase=post_selection_queue_flush dt_ms=...
```

Compare a fast side box and a slow side box before proposing a fix.

The current report says all scene targets are slow, so first compare one
ordinary cube with selection disabled via `Cursor3d` mode, then the same cube
in `Select` mode. That establishes the zero-selection baseline and avoids
misattributing regular avatar/render cost to the editor path.

## Related

- `docs/task/editor_selection_and_paint_perf.md`
- `docs/bugs/panel-clicks-blocked-by-selectable-scene-objects-behind-ui.md`
- `examples/vtuber-desktop.mms`

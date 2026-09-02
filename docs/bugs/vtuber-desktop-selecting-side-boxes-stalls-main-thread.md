# Selecting side-scene boxes stalls vtuber-desktop

## Status

Open performance investigation.

## Symptom

Selecting some decorative boxes positioned to the side of
`examples/vtuber-desktop.mms` with Select or Select-and-Cursor causes a
multi-second visible stall. Animation and interaction pause, indicating that
the main thread is blocked rather than merely routing selection incorrectly.

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

## Required measurements

Add per-phase timing for the selected component id:

```text
[perf][select] target=<id> phase=gizmo_attach dt_ms=...
[perf][select] target=<id> phase=inspector_refresh dt_ms=...
[perf][select] target=<id> phase=world_panel_sync dt_ms=...
[perf][select] target=<id> phase=selection_total dt_ms=...
```

Compare a fast side box and a slow side box before proposing a fix.

## Related

- `docs/task/editor_selection_and_paint_perf.md`
- `docs/bugs/panel-clicks-blocked-by-selectable-scene-objects-behind-ui.md`
- `examples/vtuber-desktop.mms`

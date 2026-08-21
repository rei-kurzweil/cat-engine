# Editor Grid and Assets panel widths

Date: 2026-08-21

Status: implemented; visual/manual acceptance remains open

## Goal

Increase the editor panel widths needed by the current controls without tying
the result to a particular demo scene:

- make the Grid panel approximately **45% wider** than its current shell; and
- make the Assets panel approximately **20% wider** than its current shell.

The Grid-panel change is needed now that each grid row includes the Local/World
visual-space control alongside visibility, enablement, binding, and deletion.
The Assets-panel change is an independent readability/usability adjustment.

## Scope

Start by changing the shared panel layout/shell dimensions used by the editor.
Do not assume the current `paint-stroke-debug` scene is the source of the
problem, and do not make scene-local scale tweaks to compensate.

This should also determine whether `EditorUIComponent` needs an authored
per-panel dimension override. That API is a follow-up only if shared shell
dimensions cannot serve the editor layouts cleanly; it is not required merely
to apply these two width changes.

## Acceptance

- Grid rows show their full label and all five controls without crowding,
  clipping, or overlapping adjacent panels.
- The Grid panel’s effective width is about 1.45 times the pre-change width.
- The Assets panel’s effective width is about 1.20 times the pre-change width.
- The panel arrangement remains usable in the diagnostic scene and ordinary
  editor scenes, at desktop and XR UI scales.
- Existing panel selection, scrolling, and Grid-panel action payload routing
  remain unchanged.

## Implementation notes

1. The shared MMS panel-shell constants in
   `assets/components/internal/panels.mms` were the current width source:
   `GRID_PANEL_WIDTH_GU = 29.5` and `ASSET_PANEL_WIDTH_GU = 39.0`.
2. Applied the requested ratios at that source: Grid is now `42.775 GU`
   (`29.5 × 1.45`) and Assets is now `46.8 GU` (`39.0 × 1.20`), preserving height and
   placement policy unless a layout constraint requires an explicit adjustment.
3. Test a Grid row in both Local and World modes, plus a populated Assets
   panel with long module/asset labels.
4. Only then decide whether a general `EditorUIComponent` per-panel sizing API
   is warranted; if so, specify defaults, override precedence, serialization,
   and responsive behavior separately.

## Verification record

The shared shell source is updated; no `EditorUIComponent` per-panel override
was needed. Automated compile and focused test results are recorded alongside
this change. Manual checks still need to cover the Grid row in Local and World
modes and long Assets labels at desktop and XR UI scales.

## Related

- [Grid visual-coordinate-space tracker](grid-visual-coordinate-space-tracker.md)
- [Grid panel and grid inspector](grid-panel-and-grid-inspector.md)
- [Panel system](panel-system.md)
- [Assets selection and paint panels](assets-slection-and-paint-panels.md)
- [Editor asset/world panel performance](editor-asset-world-panel-performance.md)

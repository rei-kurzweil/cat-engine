# Editor, grids, rasterized placement, and paint

Date: 2026-09-02

Status: active and ready. This page is a desktop-workbench entry point; the
0.8 release gate remains the canonical checklist.

[Back to the desktop workbench](README.md)

## Outcome

Make level editing dependable for both deliberate object placement and
raster-like drawing: select an asset, choose or create a grid, preview a cell or
stroke, commit exactly the visible result, then select and manipulate it without
losing editor state.

## Highest-priority validation failure

2026-09-05: the user retested Free Draw on an empty grid after the BVH/raycast
participation changes and it still fails. [The existing bug](../bugs/free-draw-cannot-start-on-empty-grid-analytic-plane.md)
now records the implementation audit and the remaining runtime diagnosis.
Investigate this first under the [interaction priority tracker](interaction-priorities.md).
Registration is present; the user confirms stroke startup still fails after
toggling the grid. [Startup visibility](../bugs/default-grid-visibility-ui-state-out-of-sync.md)
is the second priority: the UI says visible before the authored grid renders.

## Canonical trackers

- [Editor grid and paint 0.8 release gate](../task/editor-grid-paint-0.8.0-release-gate.md)
- [Grid + gizmo + paint end-to-end UX and test matrix](../task/grid-gizmo-paint-end-to-end-ux-and-test-matrix.md)
- [Grid-aware paint stroke interaction model](../task/grid-aware-paint-stroke-interaction-model.md)
- [Desktop paint/grid deduplication reproduction](../task/paint-grid-stroke-cell-deduplication-and-desktop-repro.md)
- [Unified grid snap mode](../task/unified-grid-snap-mode-mms-gizmo-and-paint.md)
- [Editor icon and preview layout observations](editor-icon-and-preview-layout-observations.md)

## Work groups

### Restore the basic desktop journey

- [ ] Reproduce asset selection → Paint focus → preview → one committed object
      in `examples/paint-grids-desktop.mms`.
- [ ] Make inactive reasons visible instead of leaving a silent no-op.
- [ ] Verify preview and commit have identical pose, color, and asset identity.
- [ ] Verify a click commits once and a drag does not duplicate a visited cell.

### One grid contract

- [ ] Use one authoritative selected-grid frame, spacing, plane, anchor, and
      enabled/visible state across rendering, paint, cursor, and gizmos.
- [ ] Verify translated, rotated, vertical, horizontal, and non-unit grids.
- [ ] Preserve cell phase for cell-centered objects instead of snapping only
      their transform origin to a grid line.

### Rasterized tools

- [ ] Finish the Free Draw interaction and cell-deduplication contract.
- [ ] Implement Line from deterministic integer grid addresses and shared
      preview/commit primitives.
- [ ] Keep Spray and Erase on the same address and stroke-continuity model.
- [ ] Defer Fill until accelerated occupancy queries and cancellation limits
      have an explicit design, as recorded by the release gate.

### Level-editing confidence

- [ ] Make newly placed objects immediately selectable and gizmo-editable.
- [ ] Verify save/reload retains scene content and asset references without
      serializing editor-only UI.
- [ ] Run the detailed desktop matrix before comparing XR-specific routing.

## Nearby bug trackers

- [Free Draw does not snap to the selected grid](../bugs/free-draw-paint-does-not-snap-to-grid-while-grid-tool-placement-does.md)
- [Grid tool selection/gizmo interference](../bugs/grid-tool-leaves-grid-as-only-selectable-target-and-grid-drags-rotate-gizmo.md)
- [Grid panel does not refresh after placement](../bugs/grid-panel-does-not-refresh-after-grid-tool-placement.md)
- [Editor cursor GLTF/grid alignment](../bugs/editor-cursor-3d-gltf-and-grid-alignment.md)
- [Shared editor UI duplicates paint handlers](../bugs/shared-editor-ui-root-duplicates-editor-scoped-paint-handlers.md)
- [MMS save includes editor UI and omits GLTF URI](../bugs/mms-save-includes-editor-ui-and-omits-gltf-uri.md)

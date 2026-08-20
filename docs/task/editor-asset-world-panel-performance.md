# Asset and World panel performance

Date: 2026-08-20

Status: discovery tracker; do not begin implementation from this document

Related:

- [Editor panel minimize and render suspension](editor-panel-minimize-and-render-suspension.md)
- [Event-driven CPU culling for flat stencil clips](event-driven-stencil-clip-culling.md)
- [Bounds and BVH flow](../draft/bounds-and-bvh-flow.md)
- [Document rendering pipeline](../draft/document-rendering-pipeline.md)
- [Nested reducers for panels](../draft/nested-reducers-for-panels.md)
- [Asset selection and paint panels](assets-slection-and-paint-panels.md)

## Purpose

Make large Asset panels, and secondarily large World panels, responsive without
coupling their semantic selection state to temporary UI rows or prematurely
committing to a general renderer-culling architecture.

The immediate observations are:

- the Assets panel eagerly materializes every module header, asset item, and
  asset preview during panel construction;
- collapsing the whole panel removes its body subtree and noticeably improves
  responsiveness, which is useful evidence that live panel content is a major
  cost; and
- editor-owned MMS modules are discovered as regular placeable assets, adding
  noise and unnecessary preview/list work.

## Current facts

### Selection survives a panel collapse by design

Asset selection is stored outside the removable accordion body in
`EditorContextState.selected_asset_payload` and in Paint's `PaintState` after
the selection event is handled. The panel body holds the `SelectionComponent`
and visible highlight, but is not the authority for the selected asset.

Therefore, selecting an asset and then collapsing the Assets accordion should
not clear the operand used by Paint. Restoring the body may need to recreate
its visual selected-row state; existing Asset/World accordion restoration
regressions mean this is an expected test requirement, not an assumption to
leave unverified.

### Current asset enumeration and materialization

`AssetSystem::scan_assets_dir()` scans the immediate `assets/components/`
directory for `.mms` files. Every exported function becomes an `AssetItem`.
`spawn_assets_panel()` then eagerly constructs one module header and one item
shell per item, and tries to instantiate/measure a preview for each item.

The current temporary exclusion is only a preview heuristic based on module
name containing `panel`; it still discovers and lists those exports. It is not
a durable editor-internal asset policy.

### CPU culling: prepared, not delivered for panels

We have useful groundwork, but not viewport list virtualization:

| Capability | Current state |
| --- | --- |
| Local/world bounds source | `BoundsComponent` and BVH refactor prepared the data path. |
| General non-raycastable BVH/frustum query | planned, not implemented. |
| Stencil masking plus CPU-side `VisualWorld` exclusion | separately designed and deferred; stencil remains the exact GPU mask while conservative CPU exclusion omits fully outside instances before stream construction. |
| Panel row/tile viewport windowing | not implemented. |

The planned stencil-mask/`VisualWorld` exclusion work is renderer-wide, event-driven, and conservative. A
panel window is a UI projection concern: it decides which row/tile subtrees to
materialize and which previews to keep live. They may share geometry and
visibility information later, but neither should be blocked on the other.

## Workstreams to compare

### 1. Per-module accordion sections

Each MMS module becomes one independently collapsible section with a chevron.
Expanding it materializes or reveals its exported asset rows; collapsing it
removes/suspends those rows and previews.

Requirements to decide:

- section expanded/collapsed state has a stable module-path key and lives
  outside removable rows;
- a selected asset remains selected when its module section or parent panel is
  collapsed;
- reopening a section restores its visible selection marker without emitting a
  false deselection;
- the module header is inexpensive and remains available to expand the section;
- item previews are created lazily only for expanded/near-visible sections;
- module ordering and keyboard/pointer scrolling remain stable.

This is likely the first useful optimization because it reduces both UI nodes
and expensive preview instantiation before viewport virtualization exists.

### 2. Separate editor-internal MMS modules from placeable assets

Editor UI MMS modules live under `assets/components/internal/`. Asset discovery
is deliberately shallow: only direct `.mms` children of the configured asset
root are catalog candidates, so the `internal/` subtree can never become
Asset-panel entries. `button.mms` and `icons.mms` remain public top-level
modules.

This is a catalog boundary, not a generic module-loading restriction. Internal
modules remain explicitly loadable/importable by the editor runtime and tests;
they are only excluded from the placeable Asset catalog.

### 3. Viewport-window heuristic for rows and wrapping asset tiles

The target behavior is the viewport's visible rows plus one overscan row above
and below. For an inline/wrapping asset grid, the unit is a **computed visual
row**, not an individual tile: several tiles and section headers can occupy a
row, and tile width/line wrapping may change on resize.

Candidate levels:

1. **Measured row window.** After layout provides actual item/header bounds,
   retain or materialize only subtrees overlapping the scroll viewport expanded
   by one computed row. This is the most correct general route.
2. **Fixed-size provisional heuristic.** Use a known tile/header size and
   panel viewport height to mount `ceil(viewport / row_height) + 2` rows. It is
   cheaper to introduce but must fall back safely when wrapping, font scale, or
   variable section content breaks the estimate.
3. **Preview-only window.** Keep lightweight row shells/list labels live but
   defer expensive preview subtrees outside the overscan window. This may give
   most of the Assets benefit with less focus/selection/scroll churn.

Do not choose an approach until profiling separates construction, layout,
renderer instance work, hit-testing, and preview instantiation costs. The World
panel is primarily a vertical variable-row list, while Assets is a grouped,
wrapping tile layout; they should share a viewport contract only where it fits.

### 4. Whole-panel suspension remains complementary

The existing accordion body-removal mechanism is not item virtualization, but
it is valuable: a collapsed panel should have no body layout, hit-testing,
renderer, or preview work. Preserve that behavior while adding section and
viewport-level policies.

## Measurements and tests before implementation

- [ ] Record Asset panel item count, module count, preview count, renderable
      count, layout time, renderer preparation time, and input-frame time with
      the panel expanded versus collapsed.
- [ ] Separate initial-open cost from scroll cost, selection cost, and
      collapse/restore cost.
- [ ] Confirm selection -> collapse -> Paint retains the selected asset
      payload, then restore and verify the selected-row highlight is correct.
- [ ] Reproduce and characterize existing Asset/World accordion restore
      regressions before layering section accordions on top.
- [ ] Inventory `assets/components/` exports into public/placeable,
      editor-internal, reusable-UI, and ambiguous groups.
- [ ] Identify all MMS import paths, Rust hard-coded paths, tests, examples,
      asset keys, and serialized references affected by an `internal/` move.
- [ ] Prototype section accordion state/model without committing to item
      virtualization.
- [ ] Establish a measured row/tile geometry source suitable for a viewport
      window; do not infer rows from raw item count.
- [ ] Compare preview-only windowing with full row-subtree windowing on the
      same large Assets workload.
- [ ] Decide whether World and Assets share a list-window abstraction or only
      share measurement/overscan primitives.

## Acceptance criteria for a later implementation

- Collapsing a module or whole Assets panel never clears the selected asset or
  causes a false Paint deselection.
- Reopening faithfully restores the latest selection state without duplicate
  rows, handlers, or preview subtrees.
- Internal editor modules are absent from the asset catalog by explicit policy,
  not by a title/name heuristic.
- Large panels materialize and keep live only bounded visible/overscan content,
  with a correct path for wrapping tiles and section headers.
- Off-window content cannot render or steal pointer hits; re-entry is stable.
- Panel windowing does not depend on completing global CPU frustum/clip culling.
- Measurements demonstrate the intended improvement on a reproducible large
  catalog and large World tree.

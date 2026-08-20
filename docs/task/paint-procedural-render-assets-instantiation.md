# Paint procedural RenderAssets instantiation

Date: 2026-08-20

Status: discovery tracker; no implementation architecture selected

Related:

- [Grid-aware paint stroke interaction model](grid-aware-paint-stroke-interaction-model.md)
- [Current paint-stroke pipeline investigation](../analysis/grid-aware-paint-stroke-current-pipeline.md)
- [Live MMS module preview components vs panel materialization](live-mms-module-preview-components-vs-panel-materialization.md)
- [Asset selection and paint panels](assets-slection-and-paint-panels.md)

## Problem

The first focused live Free Draw stroke reached Paint with a valid asset
selection and gesture, but preview creation failed before there was a preview
root:

```text
asset_key="assets/components/primitives.mms::heart"
error=paint failed: asset spawn error:
procedural Renderable constructors require live RenderAssets
```

This is the first confirmed Free Draw blocker. It is upstream of grid
selection, snapping, preview pose, drag continuity, and commit.

`heart()` is a procedural `Renderable` constructor. Its MMS host binding uses
the thread-local live `RenderAssets` scope. The asset-panel preview path
already has a caller that supplies `&mut RenderAssets` and therefore uses
`spawn_mms_module_component_uninitialized_with_assets(...)`. Paint instead
uses `spawn_mms_module_component_uninitialized(...)` with no live asset scope.

Spray Can currently shares Paint's `spawn_asset_subtree` path and is expected
to fail for the same reason. Line remains a distinct no-placement baseline.

## Scope

Restore a deliberate, renderer-asset-aware live instantiation path for
placement preview and committed placement. It must support procedural meshes
without weakening the template/materialization path used by editor panel
factories.

This task does not define grid-aware stroke behavior, Line cells, Spray Can
semantics, or a general asset-browser redesign.

## Constraints

- `RenderAssets` is mutable renderer-owned state. A solution cannot create
  aliased mutable access or leave a live-assets pointer installed beyond the
  synchronous instantiation scope.
- Paint handlers currently run through reactive `RxWorld` closures, while the
  `SystemWorld` registration path owns the mutable `RenderAssets` reference.
  The ownership boundary must be explicit rather than hidden in a second
  global.
- Existing panel factories still need a CE/template materialization path. Do
  not solve Paint by changing all module factories to live evaluation.
- Preview and committed placement must use the same asset-instantiation
  contract. A preview that works but a Spray/commit that fails is not an
  acceptable split.
- Failure must be visible in Paint status/logging; it must not silently become
  `preview_root=None`.

## Options to compare

### A. Give the Paint effect path scoped `RenderAssets` access

Thread `&mut RenderAssets` through the frame/system boundary that dispatches
Paint effects, then call the existing asset-aware runner API directly.

Benefits: smallest semantic change; reuses the proven asset-panel preview
mechanism; direct failures are available at the point of user action.

Risks/questions: current reactive handler signatures do not carry that borrow;
we need to decide whether Paint effects stay in handlers or are deferred to a
system-owned phase that can safely borrow renderer assets.

### B. Queue typed placement-instantiation requests for `SystemWorld`

Paint records a typed request/preview intent. A system phase that owns
`RenderAssets` instantiates it synchronously during the same frame, then
returns success/failure to Paint's preview state.

Benefits: clean ownership and an extensible seam for asset types beyond MMS.

Risks/questions: request/result ordering, preview latency, cancellation before
execution, and direct error/status delivery need a contract. Avoid turning a
simple placement into an unbounded asynchronous job system.

### C. Extract a scoped asset-instantiation service

Make a narrow service owned by `SystemWorld`/`AssetSystem` that exposes only
`instantiate_live(template, args, world, emit)` while it holds the renderer
asset borrow. Both asset-panel previews and Paint call that one service.

Benefits: one authoritative policy for runner mode, argument defaults,
renderer-asset scope, error reporting, and future MMS/GLTF asset kinds.

Risks/questions: the service still needs a safe way to be invoked from Paint's
reactive flow; do not disguise Option A's borrow problem behind a singleton.

### D. Broaden the runner's explicit module-evaluation modes first

Extend the existing template-vs-live module-factory plan with an explicit
live-with-render-assets mode, then make Paint and asset previews opt into it.

Benefits: the API documents both live component registration and procedural
renderer resource requirements.

Risks/questions: this may be a useful enabling refactor but is larger than the
immediate Paint failure. It must not delay a focused 0.8 repair if the caller
ownership solution is otherwise clear.

## Questions to answer before choosing

1. Where can the main frame borrow `RenderAssets` while still allowing Paint to
   create/update a preview in the same interaction frame?
2. Is a caller-level live-with-assets runner API sufficient, or is a typed
   placement-instantiation request boundary needed for correctness?
3. Which parts of spawn failure are recoverable, and how should Paint status
   distinguish missing asset selection, factory evaluation failure, procedural
   resource failure, missing bounds, and unsupported surface?
4. What is the cleanup rule when asset spawn succeeds but later preview-frame
   or bounds setup fails? The current early-return path can leave temporary
   subtrees behind.
5. Which asset classes require renderer assets today (heart/star/polygon/etc.)
   and which focused assets prove both the no-resource and resource-backed
   paths?

## Investigation and acceptance plan

- [x] Capture a focused live Free Draw failure with a procedural asset.
- [ ] Trace the exact `SystemWorld`/reactive-dispatch borrow boundary and list
      every runner entry point used by asset preview, Free Draw, and Spray Can.
- [ ] Compare Options A-D against that boundary; select the smallest one that
      preserves synchronous preview feedback and Rust aliasing safety.
- [ ] Add failure cleanup and user-visible status coverage for every preview
      start stage.
- [ ] Verify Free Draw click, short drag, and drag commit with `heart()` and a
      non-procedural asset.
- [ ] Verify Spray Can with the same two asset classes.
- [ ] Verify asset-panel previews still instantiate procedural assets and
      retain their intended live/template behavior.
- [ ] Verify panel factory materialization remains CE/template based where
      required.
- [ ] Resume grid-aware Free Draw/Spray/Line testing only after a preview can
      be created and committed reliably.

## Exit criteria

- A selected procedural asset produces a visible, movable Free Draw preview
  and commits at the same pose.
- Spray Can can create the same asset without a renderer-assets error.
- The asset-panel preview and Paint placement paths share an explicit,
  documented live-with-assets contract.
- No global mutable renderer-assets escape hatch or long-lived alias is added.
- Failure messages identify the failed stage and do not leak partially spawned
  preview subtrees.


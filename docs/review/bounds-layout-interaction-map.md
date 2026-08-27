# Bounds and Layout Interaction Map

Status: review scaffold; fill out after the data-visualization placement work is
verified end to end.

## Immediate priority

Finish and verify
[`docs/task/layout-owned-renderable-content-placement.md`](../task/layout-owned-renderable-content-placement.md)
against `examples/data-viz-json-file.mms` before expanding this review.

The end-to-end acceptance case is:

- the layout system derives intrinsic size from the bars' renderable bounds;
- the rendered bars are horizontally centered and bottom-aligned inside their
  layout containers;
- labels remain in flow above the bars;
- the MMS example specifies the data-driven scale, but does not compensate for
  mesh origins with `width / 2`, `height / 2`, or equivalent translations;
- repeated layout and transform updates do not accumulate placement drift.

The placement contract being developed for that work is documented in
[`docs/spec/layout-visual-placement-component.md`](../spec/layout-visual-placement-component.md).

## Seams to map

This is the initial index of the code and design seams that the later review
should connect. It is intentionally not yet a claim that the entire pipeline is
correct or fully documented.

### Local renderable bounds and caching

- `src/engine/ecs/component/bounds.rs`
- `src/engine/ecs/system/renderable_system.rs`
- [`docs/review/mesh_component.md`](mesh_component.md)

Questions to record later: who creates and invalidates each cached bound, which
coordinate space it uses, and whether all renderable kinds follow the same
contract.

### Transform-aware subtree measurement

- `src/engine/ecs/system/bounds_system.rs`
- [`docs/draft/component-tree-bounds-measurement-v1.md`](../draft/component-tree-bounds-measurement-v1.md)
- [`docs/draft/bounds-and-bvh-flow.md`](../draft/bounds-and-bvh-flow.md)
- [`docs/task/transform-aware-intrinsic-layout-bounds.md`](../task/transform-aware-intrinsic-layout-bounds.md)

Questions to record later: subtree boundaries, transform composition,
root-local versus world-space results, cache reuse, and dirty propagation.

### Layout measurement and flow

- `src/engine/ecs/system/layout/measure.rs`
- `src/engine/ecs/system/layout/block.rs`
- `src/engine/ecs/system/layout/inline.rs`
- `src/engine/ecs/component/layout_bounds.rs`

Questions to record later: how text, child flow, and visual bounds contribute to
auto sizing; when `max(child-flow, visual-height)` or a bounds union is valid;
and where alignment changes measurement versus placement.

### Visual placement and transform resolution

- [`docs/task/layout-owned-renderable-content-placement.md`](../task/layout-owned-renderable-content-placement.md)
- [`docs/spec/layout-visual-placement-component.md`](../spec/layout-visual-placement-component.md)
- `src/engine/ecs/system/transform_system.rs`

Questions to record later: ownership and lifecycle of layout placement
metadata, how it composes with authored transforms, system ordering, and how
measurement excludes previously resolved placement so relayout cannot drift.

### Other bounds consumers

- `src/engine/ecs/system/fit_bounds_system.rs`
- `src/engine/ecs/system/bvh_system.rs`
- [`docs/task/fit-bounds-layout-container-and-presentational-subtree.md`](../task/fit-bounds-layout-container-and-presentational-subtree.md)
- [`docs/task/generalized-mesh-bounds-visualization-and-combine-mesh-aabbs.md`](../task/generalized-mesh-bounds-visualization-and-combine-mesh-aabbs.md)
- [`docs/task/editor-workspace-width-from-post-layout-bounds.md`](../task/editor-workspace-width-from-post-layout-bounds.md)
- [`docs/task/layout-root-computed-size-and-shift-event.md`](../task/layout-root-computed-size-and-shift-event.md)

Questions to record later: which consumers need geometry-local, subtree-local,
post-layout, or world-space bounds, and which should share traversal without
sharing layout-specific boundary policy.

## Working pipeline to verify

The review should eventually validate or correct this provisional chain:

```text
Renderable geometry
  -> cached local BoundsComponent
  -> transform-aware subtree bounds
  -> intrinsic layout measurement and flow
  -> layout-owned visual placement metadata
  -> effective transform/world matrix
  -> rendering, BVH, picking, FitBounds, and other consumers
```

## Deferred review checklist

After the data-visualization acceptance case passes:

- [ ] Draw the actual producer/consumer and system-ordering map.
- [ ] Label the coordinate space and ownership of every bounds value.
- [ ] Document cache creation, invalidation, and dirty propagation.
- [ ] Document caller-defined subtree boundaries and why layout's policy differs
      from general bounds consumers.
- [ ] Separate intrinsic measurement, flow allocation, visual placement, and
      final world-space bounds.
- [ ] Explain how text bounds, visual bounds, and nested child-flow dimensions
      combine, replacing provisional rules where necessary.
- [ ] Verify component-tree topology and lifecycle for layout-owned placement
      metadata.
- [ ] Check all downstream consumers: rendering, BVH/raycasting, FitBounds,
      editor previews, and debug visualization.
- [ ] Record invariants and regression tests for relayout, animation, nested
      transforms, non-centered mesh origins, and empty/unresolved renderables.
- [ ] Reconcile stale or overlapping draft, task, spec, and review documents.


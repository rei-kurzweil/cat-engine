# Correct multilayer transparency for layout-generated backgrounds

Status: proposed

Related:

- `docs/bugs/layout-background-transparency-order-varies-between-launches.md`
- `docs/task/single-layer-transparency-depth-write-contract.md`
- `docs/task/layout-transparent-background-overlap-classification.md`

## Goal

Make nested translucent `Style.background_color` quads composite deterministically and in the
intended back-to-front order.

This is the immediate correctness tracker. It does not depend on first solving automatic overlap
classification. A conservative implementation may initially treat every translucent
layout-generated `__bg` as multilayer.

## Current problem

Layout reliably creates the affected `__bg` quads, but translucent backgrounds currently enter the
single-layer transparent path. That path may reorder compatible instances for batching and does not
preserve the layout stacking relationship. Nested panel, chart, and item backgrounds can therefore
change apparent color, opacity, or visibility between launches.

## Proposed direction

- Give translucent layout-generated backgrounds a render classification that selects correct
  multilayer composition without multiplying their authored alpha.
- Preserve opaque generated backgrounds on the opaque path.
- Ensure the selected path orders nested backgrounds correctly for the active view.
- Keep this classification internal initially; do not require MMS authors to add opacity helper
  components to generated layout internals.

## Work tracker

- [ ] Add a focused reproduction with nested translucent layout backgrounds.
- [ ] Add ECS assertions that generated backgrounds retain their authored RGBA and transforms.
- [ ] Route translucent generated backgrounds through a correct ordered transparency path.
- [ ] Verify deterministic output across repeated fresh launches.
- [ ] Cover nested backgrounds, sibling backgrounds, text, clipping, overlays, and opaque panels.
- [ ] Measure draw calls and transparent fragment cost on a representative editor panel.
- [ ] Record whether the initial conservative classification should remain or be replaced by the
      layout overlap analysis tracked separately.

## Acceptance criteria

- The `data-viz-json-file` backgrounds look identical across repeated launches.
- All overlapping translucent layout backgrounds contribute to the final color in the intended
  order.
- Opaque layout backgrounds do not pay the multilayer cost.
- Correctness does not depend on component or hash-map iteration order.

## Non-goals

- Redefining the engine-wide single-layer transparency contract.
- Detecting the minimal set of overlapping layout backgrounds.
- Implementing general-purpose order-independent transparency.


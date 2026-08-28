# Translucent layout backgrounds vary or disappear between launches

Date: 2026-08-27

Status: open / reproduced in `data-viz-json-file`

Follow-up trackers:

- `docs/task/layout-background-multilayer-transparency-correctness.md`
- `docs/task/single-layer-transparency-depth-write-contract.md`
- `docs/task/layout-transparent-background-overlap-classification.md`

## Summary

Overlapping layout-generated `__bg` quads with alpha below `1.0` can appear with different colors
or opacity between otherwise identical launches. A background may also look completely absent.

The layout system is creating the quads consistently. The visible variation appears later, when
the translucent instances are registered and drawn in an order that does not preserve their
authored front-to-back relationship.

## Reproduction

Run:

```sh
cargo run --release -- load examples/data-viz-json-file.mms
```

The example contains nested translucent layout backgrounds:

- the enclosing panel background
- the chart-region background
- one background for each bar

Across fresh launches, observe whether all bar backgrounds are visible and whether their apparent
color and opacity remain stable.

`InspectLayout` can make the scene harder to read, but the problem remains when it is disabled.

## Expected behavior

- The same scene produces the same composited result on every launch.
- A layout-generated background does not disappear because unrelated renderables registered in a
  different order.
- Nested translucent backgrounds composite according to their spatial/layer order.

## Current findings

A repeated ECS-side probe found that every launch created exactly one `__bg` for each of the three
bars. Their authored RGBA values and resolved transforms were stable. This rules out nondeterministic
layout creation as the immediate cause.

The affected backgrounds have alpha below `1.0` and use the single-layer transparent path. That
path batches for throughput rather than sorting overlapping instances by view depth. Renderable
registration can originate from hash-backed pending collections, so equal batch keys do not provide
a stable semantic layer order. Nested quads can therefore composite in a different order and a
nearly opaque outer quad can visually cover an inner quad.

Relevant areas:

- `src/engine/ecs/system/layout/block.rs`
- `src/engine/ecs/system/renderable_system.rs`
- `src/engine/graphics/visual_world.rs`
- `src/engine/graphics/vulkano_cbb.rs`
- `examples/data-viz-json-file.mms`

## Possible solution directions

- Route translucent layout-generated backgrounds through the existing depth-sorted multi-layer
  transparent path.
- Give layout backgrounds an explicit stable layer/order key and preserve it through render-stream
  construction.
- Add a dedicated UI/layout transparency phase whose ordering follows the layout tree and resolved
  layer values.
- Treat fully opaque layout backgrounds as opaque, while requiring authors or generated helpers to
  opt overlapping translucent backgrounds into correct multi-layer composition.

The preferred solution should keep opaque and non-overlapping UI batching inexpensive while making
overlapping translucent layout backgrounds deterministic.

## Validation

- Launch the reproduction repeatedly and verify identical output each time.
- Confirm all three bar `__bg` quads retain their intended color and opacity.
- Cover nested translucent panels, sibling backgrounds, and opaque backgrounds in focused tests.
- Verify that fixing draw order does not introduce regressions in clipping, overlays, or text
  backgrounds.

This report is separate from the missing star background being investigated in the same example;
the current evidence does not establish that both symptoms share a cause.

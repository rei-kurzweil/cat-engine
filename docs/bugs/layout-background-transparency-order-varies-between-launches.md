# Translucent layout backgrounds vary or disappear between launches

Date: 2026-08-27

Status: open / reproduced in `data-viz-json-file` and `planar-auto-transparency-optimization`

Follow-up trackers:

- `docs/task/layout-background-multilayer-transparency-correctness.md`
- `docs/task/single-layer-transparency-depth-write-contract.md`
- `docs/task/layout-transparent-background-overlap-classification.md`
- `docs/task/visual-world-automatic-transparency-scope-transactions.md`

## Summary

Overlapping layout-generated `__bg` quads with alpha below `1.0` can appear with different colors
or opacity between otherwise identical launches. A background may also look completely absent.

In the original `data-viz-json-file` reproduction, the layout system creates the quads consistently
and the visible variation appears later in registration or drawing. The simplified 12 x 12
reproduction still needs the same ECS-to-`VisualWorld` count before assuming it has the identical
failure boundary.

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

### Simplified 12 x 12 reproduction

Run:

```sh
cargo run --release -- load examples/planar-auto-transparency-optimization.mms
```

The benchmark contains a `LayoutRoot` with 12 explicitly sized block rows. Each row contains 12
explicitly sized inline-block transforms with:

```mms
background_color([0.64, 0.07, 0.34, 0.50])
```

There is no cube or other authored visual content inside the cells. The intended layout result is
exactly 144 reddish-purple, half-transparent generated `__bg` quads. At the time of this report the
individual squares are not visible in the running scene.

Unlike the nested `data-viz-json-file` case, adjacent cell rectangles have positive margins and
should not overlap one another in layout-local XY. This is an important challenge to the narrower
"incorrect ordering only between overlapping generated backgrounds" hypothesis. Possible cases to
distinguish are:

- the empty styled cells do not generate or register their `__bg` renderables;
- their transforms, dimensions, alpha, or render-phase flags differ between ECS and `VisualWorld`;
- a large transparent scene plane composites over or suppresses them;
- the single-layer stream loses or incorrectly batches non-overlapping translucent instances;
- the quads exist but face, depth-test, or camera/frustum state makes them invisible.

Do not treat the original hash-backed registration-order explanation as sufficient for this
simplified reproduction until the 144 generated components and their registered visual instances
have been counted directly.

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
- `examples/planar-auto-transparency-optimization.mms`

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
- In the simplified benchmark, verify 144 styled cells produce 144 `__bg` transforms, 144 nested
  renderables, and 144 registered `VisualWorld` instances before investigating draw order.
- Temporarily hide the ocean plane and trusses to determine whether unrelated transparent/opaque
  scene content affects the grid.
- Confirm all 144 non-overlapping squares are visible and stable before adding overlapping layout
  backgrounds back to the benchmark.

This report is separate from the missing star background being investigated in the same example;
the current evidence does not establish that both symptoms share a cause.

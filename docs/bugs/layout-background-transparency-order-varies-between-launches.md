# Translucent layout backgrounds vary or disappear between launches

Date: 2026-08-27

Status: open / reproduced in `data-viz-json-file` and `planar-auto-transparency-optimization`

Follow-up trackers:

- `docs/bugs/runtime-spec-component-body-control-flow-drops-children.md`
- `docs/task/layout-background-multilayer-transparency-correctness.md`
- `docs/task/single-layer-transparency-depth-write-contract.md`
- `docs/task/layout-transparent-background-overlap-classification.md`
- `docs/task/visual-world-automatic-transparency-scope-transactions.md`

## Summary

Overlapping layout-generated `__bg` quads with alpha below `1.0` can appear with different colors
or opacity between otherwise identical launches. A background may also look completely absent.

Two separate failure modes were initially conflated. The legacy MMS evaluator creates the benchmark
quads consistently and exposes the transparency-order problem. The RuntimeSpec evaluator used by
`cargo run -- load`, however, drops component children produced by control flow inside component
bodies. In the 12 x 12 benchmark, `LayoutRoot` is therefore empty before LayoutSystem runs. The
current total disappearance is an evaluator materialization bug, not a GPU draw/composite failure.

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

### Simplified 24 x 24 reproduction

Run:

```sh
cargo run --release -- load examples/planar-auto-transparency-optimization.mms
```

The benchmark contains a `LayoutRoot` with 24 explicitly sized block rows. Each row contains 24
explicitly sized inline-block transforms with:

```mms
background_color([0.64, 0.07, 0.34, 0.50])
```

Each cell also contains an opaque white cube at local Z = `-2`, behind its generated background.
The intended layout result is exactly 576 reddish-purple, half-transparent generated `__bg` quads
over 576 opaque cubes. At the time of this report the individual squares are not visible in the
running scene.

Unlike the nested `data-viz-json-file` case, adjacent cell rectangles have positive margins and
should not overlap one another in layout-local XY. Once RuntimeSpec loop materialization is fixed,
this remains a useful transparency benchmark. Until then it does not reach the transparency path.

## Expected behavior

- The same scene produces the same composited result on every launch.
- A layout-generated background does not disappear because unrelated renderables registered in a
  different order.
- Nested translucent backgrounds composite according to their spatial/layer order.

## Current findings

A focused trace using the legacy evaluator found:

- 144 matching `StyleComponent`s;
- 144 generated `__bg` transforms with valid world matrices and nonzero dimensions;
- 144 matching colors and nested square renderables;
- 144 renderable handles after `prepare_render`;
- 144 matching `VisualInstance`s with color `[0.64, 0.07, 0.34, 0.5]`, opacity `1.0`, and
  `multiple_layers = false`;
- all 144 instance indices in the single-layer transparent render stream;
- an active window camera whose view and projection place the grid in front of the camera.

That result is still useful for the eventual transparency benchmark, but it does not describe the
CLI loader. A matching test using `eval_with_runtime_spec_at_path`, plus a live windowed audit,
instead found:

- every `LayoutRoot` existed with zero children;
- zero `StyleComponent`s from the nested layout loops existed;
- zero generated `__bg` components existed;
- computed layout heights were `-0.0` and the roots were then clean;
- only top-level, non-layout control renderables reached `VisualWorld`.

The RuntimeSpec materializer stores a component body containing `for` as `deferred_block`. The
Mittens host only consumes that deferred payload for imperative owners such as keyframes; it does
not execute it to populate an ordinary component such as `LayoutRoot`. Consequently, nested loop
results never become children. Relevant code is in
`crates/meow-meow-script/src/evaluator.rs`, `src/scripting/host.rs`, and
`src/scripting/component_registry.rs`.

The affected backgrounds have alpha below `1.0` and use the single-layer transparent path. That
path batches for throughput rather than sorting overlapping instances by view depth. Renderable
registration can originate from hash-backed pending collections, so equal batch keys do not provide
a stable semantic layer order. Nested quads can therefore composite in a different order and a
nearly opaque outer quad can visually cover an inner quad.

Relevant areas:

- `crates/meow-meow-script/src/evaluator.rs`
- `src/scripting/host.rs`
- `src/scripting/component_registry.rs`
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
- In the simplified benchmark, verify 576 styled cells produce 576 `__bg` transforms, 576 nested
  background renderables, and 576 registered `VisualWorld` instances before investigating draw
  order. Verify the same count for the opaque cubes behind them.
- Temporarily hide the ocean plane and trusses to determine whether unrelated transparent/opaque
  scene content affects the grid.
- Confirm all 576 non-overlapping squares are visible and stable before adding overlapping layout
  backgrounds back to the benchmark.

This report is separate from the missing star background being investigated in the same example;
the current evidence does not establish that both symptoms share a cause.

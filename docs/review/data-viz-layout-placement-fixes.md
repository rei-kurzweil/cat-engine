# Review: Data-viz Layout Placement Fixes

This review covers the worktree changes following the initial implementation of
layout-owned renderable placement. The JSON data-viz example now renders its
bars horizontally centered and bottom-aligned in their intended boxes, while
the layout inspector agrees with the actual box model.

Related documents:

- [`layout-owned-renderable-content-placement.md`](../task/layout-owned-renderable-content-placement.md)
- [`layout-visual-placement-component.md`](../spec/layout-visual-placement-component.md)
- [`bounds-layout-interaction-map.md`](bounds-layout-interaction-map.md)

## What changed

### Inline `vertical-align` no longer moves inner content twice

`block::apply_text_align` previously interpreted `vertical-align: bottom` as
content alignment even on an `inline-block`. Inline layout had already used the
same property to align the atomic item in its line box, so the fallback inner
transform—here, the cube—received a second incorrect Y translation.

For `inline` and `inline-block` boxes, `apply_text_align` now leaves descendant
content alone. Their `vertical-align` value belongs to line-item alignment;
block-level content alignment retains its existing behavior. This also keeps
the authored cube translation unchanged instead of baking layout into it.

### Box-model visualization uses the real quad center

The inspector quads applied a stale half-unit correction to both axes. Removing
that correction makes their centered transforms match the computed margin,
border, padding, and content rectangles. This fixes the apparent right/down
shift and the previously misleading `margin_bottom` visualization.

### Layout helpers cannot feed back into intrinsic measurement

Visual subtree measurement now prunes internal nodes whose labels begin with
`__`, in addition to existing layout and transform boundaries. Generated
background and inspection geometry therefore cannot enlarge intrinsic bounds
on later layout passes.

### The example exposes its intended boxes more clearly

`examples/data-viz-json-file.mms` gives the chart region and individual bar
containers translucent diagnostic backgrounds. `background_z(-0.01)` places
each generated background slightly behind its own content without changing the
authored visual transform.

The third bar's diagnostic background is still sometimes not visible. Its box
and bar placement are correct, so this remains a separate translucency,
depth-order, or background-rendering issue rather than a layout correction to
fold into the bar transforms.

## Placement contract exercised by the example

- Layout coordinates progress downward in negative local Y.
- Bottom-aligned visuals share the content boxes' minimum-Y baseline.
- The cube geometry extends upward from that baseline toward greater Y.
- Horizontal placement matches the visual AABB center to the content-box
  center.
- Layout placement composes separately from authored TRS and does not
  accumulate across repeated layout passes.

`VisualContentMeasurement` is transient layout-system data: it couples a
measured visual AABB with the selected content root for one pass. It is not an
ECS component. `LayoutVisualPlacementComponent` is the persistent ECS result
consumed when effective transforms are resolved.

## Regression coverage

The MMS fixture test is now an end-to-end runtime test. It loads the three JSON
values, processes intents, runs layout twice, and checks that:

- all three bar items are `inline-block` layout boxes;
- each selected visual is centered in its content box;
- all effective visual bounds share a bottom baseline;
- heights/tops preserve the ordering of values `4 < 7 < 12`;
- authored visual translations remain `[0, 0, 0]`.

The focused layout tests, data-viz runtime test, `cargo check`, and
`git diff --check` pass for this worktree. Existing unrelated compiler warnings
remain unchanged.

## Files changed

- `src/engine/ecs/system/layout/block.rs`
- `src/engine/ecs/system/layout/box_model_viz.rs`
- `src/engine/ecs/system/layout/measure.rs`
- `src/scripting/tests.rs`
- `examples/data-viz-json-file.mms`

## Follow-up

Investigate the missing third diagnostic background independently. The current
MMS depth override is `background_z`; `z_index` exists in style data but is not
yet the active general ordering mechanism for this case. Once the diagnostic
work is complete, the extra translucent bar-container backgrounds can be
removed or toned down.

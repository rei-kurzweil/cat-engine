# Task: Style-driven placement of bounded visual content

## Goal

Make bounded renderable content obey the same resolved `Style` alignment rules
as other content while preserving the useful ability for transformed
renderables to contribute intrinsic width and height to auto-sized layout
items.

`__layout_visual_placement` should remain runtime metadata that records a
computed placement. It must not impose an unconditional visual-alignment
policy of its own.

This task is motivated by the editor UI regression recorded in
[`docs/desktop/editor-icon-and-preview-layout-observations.md`](../desktop/editor-icon-and-preview-layout-observations.md), but the behavior belongs to the generic layout contract.

Runnable focused reproduction:

- [`examples/triage/layout-visual-placement-alignment.mms`](../../examples/triage/layout-visual-placement-alignment.mms)

Launch it from the repository root:

```sh
cargo run -- load examples/triage/layout-visual-placement-alignment.mms
```

The scene labels its two scenarios and expected colors. Leave
`InspectLayout` commented for the first visual comparison, then enable it in
the scene to compare the generated boxes with the rendered geometry.

### Observed baseline: `bisket-desktop-demo` build

- Scenario 1 behaves as expected: the short orange item and tall cyan item are
  bottom-aligned. This confirms that the inline post-pass can align complete
  item boxes within a line.
- Scenario 2 does not honor its three authored internal alignments. The red,
  green, and blue shapes all appear slightly below and to the right of the
  bottom-right corner of their respective white slots.

The identical result for `top`, `middle`, and `bottom` confirms that
`sync_visual_placement()` is not consuming those Style values. The fact that
the shapes are also outside the right edge means replacing the hard-coded
bottom policy with a vertical enum match will not be sufficient by itself.
The source visual AABB and target content AABB are likely being compared in
different local bases, or the resulting parent-local correction is being
composed at the wrong transform level.

## The two meanings that must not be confused

There are two distinct coordinate changes in inline layout.

### 1. Align the item box within its line

Given a short and a tall inline-block:

```mms
T {
    Style { display("inline-block") height(2.0) vertical_align("bottom") }
}
T {
    Style { display("inline-block") height(5.0) }
}
```

the line is 5 GU tall. Bottom-aligning the short item moves its entire 2 GU
margin box down by 3 GU:

```text
line top       +---------------- tall item ----------------+
               |                                           |
               |        3 GU external item offset           |
               |        +--------- short item --------+     |
line bottom    +--------+-----------------------------+-----+
```

This is external alignment: it changes the styled item's transform relative
to its siblings. The current inline post-pass in `layout/inline.rs` implements
this behavior.

### 2. Align visual content within the item box

Suppose the short item has a 1 GU-tall icon inside its 2 GU content box:

```mms
T {
    Style {
        display("inline-block")
        height(2.0)
        vertical_align("middle")
        text_align("center")
    }
    T { icon }
}
```

Centering the icon produces a separate 0.5 GU internal offset:

```text
short item top       +-----------------------------+
                     |          0.5 GU              |
                     |        [   icon   ]           |
                     |          0.5 GU              |
short item bottom    +-----------------------------+
```

This is internal alignment: it changes only the layout-owned correction for
the selected visual content root. It does not move the styled item or its
background relative to siblings.

The final world placement composes both operations:

```text
parent/line placement
  × styled-item placement, including external vertical-align offset
  × layout visual-placement correction inside the content box
  × authored visual transform
```

An author who needs different external and internal alignments can express
the two boxes explicitly:

```mms
T {
    // This outer inline item aligns with sibling item boxes.
    Style { display("inline-block") vertical_align("bottom") }

    T {
        // This inner slot aligns its text or visual content.
        Style {
            display("block")
            width(4.0)
            height(4.0)
            text_align("center")
            vertical_align("middle")
        }
        T { icon }
    }
}
```

This is the semantic wrinkle: the operations are different even when the same
Style vocabulary is resolved contextually. Nesting supplies separate style
contexts when the desired values differ.

## Current gap

The three relevant paths currently disagree:

1. `apply_inline_vertical_align()` reads `Style.vertical_align` and moves an
   inline item's whole margin box relative to the line.
2. `apply_text_align()` positions text-bearing transforms inside a content
   box, but deliberately treats internal `vertical_align` as `Auto` for
   `inline` and `inline-block` items so the property remains available to the
   line-level pass.
3. `sync_visual_placement()` does not read Style alignment at all. It always
   computes horizontal center plus bottom-edge alignment:

   ```text
   offset_x = center_x(content_box) - center_x(visual_bounds)
   offset_y = bottom_y(content_box) - bottom_y(visual_bounds)
   ```

The third rule promoted one data-visualization placement choice into a global
policy. Fixed editor slots therefore receive bottom-aligned visual content
even when they author `text_align("center")` and
`vertical_align("middle")`.

The component is not inherently wrong. Its producer is assigning the wrong
translation because the target region and alignment policy are hard-coded.

The focused reproduction additionally demonstrates a horizontal/basis error:
three symmetric visual roots intended to be horizontally centered all land
slightly beyond the target box's right edge. The implementation must verify
the coordinate space of both AABBs and the transform that consumes the
correction before treating the alignment-policy change as complete.

## Proposed semantic contract

### Style applies consistently to atomic content

- `text_align("left" | "center" | "right")` horizontally aligns inline
  content inside the content region. Atomic bounded visual content participates
  in this rule just as text does.
- `vertical_align("top" | "middle" | "bottom")` supplies the vertical
  alignment applicable in the current context. For an inline item it still
  participates in line-box alignment; for bounded content it also determines
  placement within that item's assigned visual region.
- `Auto` must have a documented neutral/default placement. It must not be an
  undocumented synonym for bottom alignment.
- Text and renderables use shared edge/center calculations and the same local
  layout-axis convention. Their different bounds sources do not justify
  different alignment semantics.

### Sizing and alignment remain separate

- On an auto axis, eligible transformed visual bounds may contribute the
  intrinsic item size. Placement normalizes the complete AABB into the
  resulting box without modifying authored scale.
- On an explicit axis, the Style dimension remains authoritative. Visual
  bounds do not resize that axis.
- Uniformly scaling arbitrary visual content to fit an explicit editor slot is
  still a fitting operation, not an alignment operation. It should be handled
  by the established renderable-only `FitBounds` boundary or an equivalent
  explicit layout-content fitting policy.
- A visual may not both determine an auto target size and be fitted back into
  that same target on the same axis.

### Mixed text and visual content needs an explicit region

The existing graph example measures direct text plus a visual and places the
visual beneath the text. That behavior should be represented as two content
regions or ordinary nested layout boxes. `__layout_visual_placement` should be
given the resolved visual target region; it should not infer that every visual
belongs against the bottom of the complete item box merely because one graph
needed that result.

## `LayoutVisualPlacementComponent` recommendation

Keep the component as derived transform metadata:

```rust
pub struct LayoutVisualPlacementComponent {
    pub source_bounds_parent_local: Aabb,
    pub translation_parent_local: [f32; 3],
}
```

Do not store `TextAlign`, `VerticalAlign`, editor-specific flags, or fitting
policy in it. Those are inputs used by `LayoutSystem` while computing the
translation. Keeping only source bounds and the result means
`TransformSystem` remains a policy-free consumer.

If diagnostics need to explain a placement, optional debug-only provenance
could record the target region and resolved alignment, but rendering and
transform propagation must not resolve Style themselves.

## Implementation outline

### 1. Represent the visual target region

Extend `VisualContentMeasurement` or the layout-pass data beside it to retain:

- selected presentational `content_root`;
- complete pre-placement visual AABB;
- the resolved region allocated to that visual; and
- whether each item dimension was intrinsic or explicit.

For a visual-only item the region is normally the content box. For mixed
text/visual content, derive the visual region after text/child-flow allocation
or require an explicit nested layout slot.

### 2. Centralize alignment math

Add policy-free helpers that align one AABB inside another:

```text
horizontal left   -> target.min_x - source.min_x
horizontal center -> target.center_x - source.center_x
horizontal right  -> target.max_x - source.max_x

vertical top      -> target.max_y - source.max_y
vertical middle   -> target.center_y - source.center_y
vertical bottom   -> target.min_y - source.min_y
```

The Y formulas follow Mittens layout's local negative-Y downward convention.
Use the same helpers for text and bounded visual placement where their
semantics overlap.

### 3. Make visual placement consume resolved Style

Change `sync_visual_placement()` to receive or resolve the relevant
`TextAlign` and `VerticalAlign`, select the visual target region, and calculate
the correction from those inputs. Remove unconditional bottom alignment.

Define contextual behavior for inline items explicitly. The inline post-pass
continues to own external item-box alignment; visual placement owns only the
internal correction. Both transformations may use the same authored
`vertical_align` value unless a nested styled slot supplies a different value.

### 4. Keep metadata lifecycle unchanged where possible

Continue to create/update one `__layout_visual_placement` beneath the selected
authored visual transform, exclude its previous correction from intrinsic
measurement, preserve authored TRS, and remove stale metadata when the visual
root is no longer eligible.

Changing the component's data shape is not required for the first fix. Change
its creation/use contract first; only extend the schema if implementation
proves that the computed translation alone is insufficient.

### 5. Encode editor fitting explicitly

Update built-in Paint icons, asset previews, and Grid icons to use fixed styled
slots with centered/middle content alignment. Move their three independent
manual scale/offset paths onto one renderable-only fit-to-slot mechanism.

This editor authoring pattern must not disable intrinsic visual sizing
globally. Data visualizations continue to use auto dimensions and transformed
intrinsic bounds.

## Required tests

- A fixed block icon slot with `text_align("center")` and
  `vertical_align("middle")` centers an off-origin AABB in both axes.
- Top, middle, and bottom visual placement use actual AABB edges/center rather
  than assumed half-extents.
- Left, center, and right visual placement work for asymmetric bounds.
- An inline-block's external line offset and internal visual offset are tested
  independently and compose without overwriting each other.
- A nested outer `vertical_align("bottom")` item plus inner
  `vertical_align("middle")` slot produces the example above.
- The unequal-height graph bars still derive intrinsic height from authored
  renderable scale and share a bottom baseline.
- Explicit editor slots do not acquire intrinsic width/height from their
  fitted visual content.
- Paint icons and labels remain separated and centered on first completed
  layout and after a dirty re-layout.
- The Grid delete X is centered in its fixed red button.
- Asset preview cases cover an origin-centered primitive, an off-origin
  primitive, an icon subtree, and one currently correct comparison asset.
- Repeated layout passes do not drift or compound placement.

## Acceptance criteria

- `__layout_visual_placement` contains a translation derived from resolved
  Style and actual source/target AABBs, not a universal bottom-placement rule.
- The same alignment intent produces the same visible result for text and
  atomic bounded visual content.
- Inline line-box alignment and internal content alignment are separately
  testable and compose predictably.
- Built-in editor visuals are layout-slot-owned, centered, and uniformly
  fitted without affecting label flow.
- Transform-aware intrinsic sizing and bottom-aligned data visualizations keep
  working without editor-specific branches in the generic layout engine.

## Relevant files

- `src/engine/ecs/system/layout/block.rs`
- `src/engine/ecs/system/layout/inline.rs`
- `src/engine/ecs/system/layout/measure.rs`
- `src/engine/ecs/component/layout_visual_placement.rs`
- `src/engine/ecs/component/style.rs`
- `src/engine/ecs/system/transform_system.rs`
- `src/engine/ecs/system/fit_bounds_system.rs`
- `assets/components/internal/panel_items.mms`
- `assets/components/internal/asset_item.mms`
- `src/engine/ecs/system/editor/grid_panel.rs`

## Related documents

- [`LayoutVisualPlacementComponent` contract](../spec/layout-visual-placement-component.md)
- [`Layout-owned placement for bounded renderable content`](layout-owned-renderable-content-placement.md)
- [`Transform-aware visual bounds for intrinsic layout`](transform-aware-intrinsic-layout-bounds.md)
- [`FitBounds layout-container targeting and presentational subtree split`](fit-bounds-layout-container-and-presentational-subtree.md)
- [`Desktop editor icon and preview observations`](../desktop/editor-icon-and-preview-layout-observations.md)

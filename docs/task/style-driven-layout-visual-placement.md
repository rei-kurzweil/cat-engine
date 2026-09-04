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
MITTENS_TRACE_LAYOUT_VISUAL_PLACEMENT=1 \
    cargo run -- load examples/triage/layout-visual-placement-alignment.mms
```

The scene labels its two scenarios and expected colors. Both repro rows are
exactly three 10-GU blocks wide with two 1-GU gaps; each row is 33 GU wide
(including its 0.5-GU side padding), and the containing panel is 35 GU wide
(with 1 GU padding). There is no unused horizontal space or accidental wrap
that could make an edge-placement error ambiguous. The environment flag
logs one
`[InspectLayout][visual-placement]` line per bounded visual. Each line includes
the resolved Style alignment, target content AABB, source visual AABB,
generated offset, and predicted placed AABB. Transform propagation also emits
`[InspectLayout][visual-transform]` with the parent and resolved world
positions plus predicted and independently measured actual world AABBs. If
those two world AABBs differ, the placement correction is being composed in a
different basis than its declared parent-local basis.

The current trace results show those two AABBs matching exactly for all three
cubes. This verifies the ECS-side bounds and transform calculation, but the
`actual_world` field is still derived from ECS state; it is not a readback of
the model matrix stored in `VisualWorld` or consumed by the renderer. Because
the white background slots look correct while the cubes visibly do not, the
next boundary to inspect is the handoff from the resolved ECS transform to the
render instance/model matrix. Compare the final model matrix associated with
each cube's render handle against its resolved transform, then—if those also
match—compare their projected clip/screen coordinates with the corresponding
background quad. This will distinguish a stale/wrong render-instance transform
from a projection or render-pass discrepancy.

The first transform-propagation probe emitted no `[render-model]` lines for
these nodes because their GPU meshes and render handles did not exist during
that traversal. They are inserted later by `RenderableSystem::flush_pending`.
The environment-gated `[InspectLayout][render-register]` trace therefore runs
at the actual insertion point. `registered_pos` is the world translation used
to create the instance, `stored_pos` is read back through the newly allocated
render handle, and `max_abs_diff` compares every matrix entry.

Early `[background-transform]` entries may report `actual_world=None` for the
same reason: the generated background's mesh bounds have not yet been attached
when transform propagation first encounters it. Those entries describe
initialization order, not evidence that the visible background has no bounds.

In the settled ECS trace, the first slot spans approximately X `[-1.8, -1.0]`
and Y `[0.03, 0.51]`, while its cube spans X `[-1.46, -1.34]` and Y
`[0.03, 0.15]`. The numbers describe a horizontally centered, bottom-aligned
cube inside the slot even though the rendered image shows it outside at the
bottom-right. This concrete disagreement moves the investigation beyond the
layout/background calculation and into render-instance registration or later
GPU draw-data handling.

The registration trace then identified a concrete stale position. The final
correct ECS centers for the top, middle, and bottom cubes are approximately
`[-1.4, 0.09]`, `[-0.52, 0.09]`, and `[0.36, 0.09]`, but their render instances
were created at `[-1.0, -0.15]`, `[-0.12, -0.15]`, and `[0.76, -0.15]`.
Every registered cube is therefore displaced by `[+0.4, -0.24]`: exactly half
of a slot's `0.8 × 0.48` world-space dimensions. The matrix stored in
`VisualWorld` exactly matches this incorrect registration matrix. This points
to an ECS-side transform mutation before registration, rather than a renderer
matrix-storage defect or a concurrent/threaded data race. The expanded trace
below identifies the writer.

### Identified double-placement writer

The expanded registration trace shows that this is more specific than a
generic stale-cache problem. Each cube was authored at `[0, 0, 0.03]`, but by
registration its authored transform is `[0.4, -0.24, 0.03]`.
`apply_text_align()` in `layout/block.rs` is the writer:

- `find_alignable_direct_child()` falls back to the first non-helper transform
  even when that transform contains no text.
- With no text descriptor, `apply_text_align()` treats the child as a
  zero-sized text anchor.
- `text_align("center")` moves that zero-sized anchor by half the 0.8-world-unit
  content width: `+0.4` X.
- For an inline-block, internal vertical alignment is reduced to `Auto`; the
  legacy “text-align implies vertical centering” branch then moves the anchor
  by half the 0.48-world-unit content height: `-0.24` Y.
- `LayoutVisualPlacement` subsequently adds its own `[+0.4, -0.42]` bounded-
  visual correction, so the same visual root is positioned twice.

This explains both observed offsets exactly. It also separates two changes
that should not be conflated:

1. Text alignment must not rewrite a non-text bounded visual's authored
   transform as though it were a zero-sized glyph block.
2. Once that accidental first placement is removed, bounded visual placement
   must intentionally resolve the appropriate `Style` alignment instead of
   retaining its unconditional center-X/bottom-Y policy.

Removing only the first write should bring all three cubes back inside their
slots, but all three would still be bottom-aligned. The top/middle/bottom
behavior remains the semantic/API task described below.

The `InspectLayout` geometry overlay remains commented in this reproduction
because of the separate transparency-ordering bug. The environment flag
provides the same console diagnostics without adding overlay geometry.

### Observed baseline: `bisket-desktop-demo` build

- Scenario 1 behaves as expected: the short orange item and tall cyan item are
  bottom-aligned. This confirms that the inline post-pass can align complete
  item boxes within a line.
- Scenario 2's white slots appear correctly positioned, sized, centered in the
  row, and separated by the authored margins. The slots are authored as
  10-GU-wide by 6-GU-tall rectangles. The red, green, and blue cubes are the
  items that appear wrong: each is horizontally centered on the slot's right
  edge and vertically displaced below the slot by approximately one cube
  height (the same apparent displacement for all three alignment values).

The identical result for `top`, `middle`, and `bottom` confirms that
`sync_visual_placement()` is not consuming those Style values. At this stage
we are documenting the behavior, not selecting a source fix. The generated
backgrounds are not the apparent cause: their geometry is in the expected
location and has the expected dimensions. The remaining question is why the
bounded renderable's authored placement does not visually coincide with the
slot despite the layout placement diagnostics reporting the expected target.

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
- For compatibility, bounded visual `Auto` currently resolves to horizontal
  center and vertical bottom placement. This default is now explicit rather
  than hidden inside unconditional placement arithmetic.
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

The first two implementation steps below are now present in the working tree:
text alignment only selects text-bearing transforms, and bounded visual
placement resolves horizontal and vertical Style alignment through centralized
AABB edge/center calculations.

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

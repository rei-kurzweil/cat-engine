# Intrinsic Text Measurement in Layout Subtrees

How the layout system finds local text content to measure when computing a
TC's intrinsic content size (width or height).

## Why this needs a rule

A styled, text-bearing box in MMS commonly looks like:

```mms
T {
    Style { display("inline-block") padding_xy(0.6, 0.6) text_align("center") }
    T.position(0.0, 0.0, 0.05) { Text { "label" } }
}
```

The text isn't a direct child of the styled T — it sits behind a positioning
inner `T.position(…)`. Naïvely halting the descent at any nested
`TransformComponent` would miss it; naïvely descending through everything
would bleed text from unrelated sub-boxes back up into the parent's intrinsic
measurement.

## The boundary rule

`find_text_in_local_content_subtree` (`src/engine/ecs/system/layout/measure.rs`)
walks the subtree rooted at a TC and returns the first local `TextComponent` or
`TextInputComponent`, subject to these boundaries (`node != root` only):

1. If the node *is* a `LayoutComponent` → halt. That subtree is its own
   measured tree (e.g. a nested `LayoutRoot` or world panel).
2. If the node has a child `StyleComponent` or `HtmlElementComponent` → halt.
   That node is its own styled box and its text belongs to *its* intrinsic
   measurement, not the ancestor's.
3. Otherwise (plain `TransformComponent`, or wrapper TCs carrying
   `ColorComponent` / `RenderableComponent` / etc.) → descend.

The root is exempt from boundary checks — it's the box we're measuring, so
its own Style/Html/Layout siblings don't stop the search.

### TextInput bridge

`TextInputComponent` is recognized before applying rule 2. Its direct
`StyleComponent` belongs to the input's text content, not to a nested layout
item. The input's current editable value is measured with `TextSystem::measure`;
the private generated `TextComponent` supplies the effective text defaults and
remains the target for glyph rebuild/font/wrap synchronization.

The generated Text is not independently measured or flowed as a sibling layout
item. See [`docs/review/text-and-textinput-layout-ownership.md`](../review/text-and-textinput-layout-ownership.md)
for the complete ownership map.

## Consumers

- `text_intrinsic_height` — used by block auto-height to size a row to its
  wrapped line count.
- `text_intrinsic_width` — used by `intrinsic_block_width` when the box is
  `display: inline-block` (CSS-aligned shrink-to-fit). `display: block`
  text-bearing boxes return `None` here and fill the available width so
  text wraps inside them. `Style.text_align` no longer affects sizing —
  it controls only glyph placement within the resolved box.

## Test anchors

- `auto_height_container_does_not_measure_text_behind_nested_transforms`
  (`measure.rs`) — asserts a `LayoutComponent` child halts the walk.
- `row_text_wrapper_still_measures_intrinsic_height` — asserts an
  unstyled TC wrapper does *not* halt the walk.

## Related

- [`layout-block-sizing.md`](./layout-block-sizing.md) — block-axis sizing
  contract; the auto-height path is what triggers intrinsic text height.

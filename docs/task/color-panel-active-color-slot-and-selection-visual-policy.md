# Color panel active color slot and selection visual policy

Date: 2026-09-05

Status: planning; implement the single-slot first slice before expanding to Color A / Color B

## Goal

Give the color panel a persistent, prominent display of the currently selected color without
recoloring palette swatches as selection feedback.

The first implementation should:

1. let an MMS-authored `Selection` disable its automatic visual highlight while preserving its
   selection state and events
2. disable that visual feedback for the color palette selection only
3. place one large selected-color square to the left of the two-row palette
4. update that square whenever a palette color is selected
5. widen the color panel enough to fit the new square, a generous separator gap, and the existing
   two palette rows

This first slice deliberately implements one active color display. A later slice can turn it into
the proposed Color A / Color B pair.

## Current state

The color panel authors 16 palette swatches as `Option` items under
`#color_panel_selection`. Clicking a swatch updates the selection and emits `SelectionChanged`,
which the editor paint system already consumes to update the current paint color.

Selection presentation is currently inseparable from selection state:

- `SelectionComponent` has no visual-feedback policy
- `SelectionSystem::add_selection_highlight(...)` always attempts to show feedback
- a styled option has its authored `background_color` replaced with the shared yellow selection
  color
- an option without a styled target receives a generated `selection_highlight` overlay instead

For color swatches, changing the background is actively misleading because the background is the
color value being previewed. The selected swatch should retain its exact authored RGBA color.

Relevant code:

- `assets/components/internal/panels.mms::color_panel_body()`
- `src/engine/ecs/component/selection.rs::SelectionComponent`
- `src/engine/ecs/system/selection_system.rs::add_selection_highlight()`
- `src/scripting/component_registry.rs` (`Selection` construction and MMS calls)
- `src/engine/ecs/system/editor_paint_system.rs` (`SelectionChanged` color bridge)

## First slice

### 1. Add an MMS-accessible selection visual policy

Add an explicit flag to `SelectionComponent`, defaulting to visual feedback enabled so existing
selections do not change behavior.

The MMS API should allow the color panel to author the disabled policy directly. Candidate syntax:

```mms
Selection.root("#palette_options").visual_feedback(false) {
    name = "color_panel_selection"
}
```

The final method name can follow established component API naming, but it must clearly govern only
the built-in highlight/background presentation. Disabling it must not disable clicks, state
updates, payload resolution, or selection events.

When visual feedback is disabled:

- do not replace a styled option's background color
- do not create a `selection_highlight` overlay
- remove any previously generated or style-based highlight if the policy changes at runtime
- continue updating `selected_entries`, `selected_component`, and `selected_payload`
- continue emitting `SelectionAdded`, `SelectionRemoved`, `SelectionCleared`, and
  `SelectionChanged` under their existing conditions

This should be a general `Selection` capability even though the color panel is its first consumer.
The default remains enabled to avoid disrupting other panels.

### 2. Add one large active-color square

Add a square at the left edge of the palette content. It represents the current editor color and
updates to the RGBA value of the most recently selected palette swatch.

“2 by 2” means approximately two normal swatch cells wide and two palette rows tall, so the square
reads as a separate active-color well rather than another palette option. It should align with the
top and bottom of the two-row palette.

Leave a horizontal separator after the square equal to roughly three times the ordinary space
between adjacent swatches. Keep that spacing explicit so the active-color well is visually grouped
separately from the built-in palette.

The active-color square is initially a display, not another palette option. The existing palette
selection remains the source of `SelectionChanged`, and the same event should update both:

- the editor's effective paint color
- the active-color square's background RGBA

Choose and document a deterministic initial color so the well and editor paint state agree before
the first click.

### 3. Widen without adding a LayoutRoot dependency

Increase the color panel's explicit width enough to contain:

- the two-row active-color square
- the three-times-normal separator gap
- eight palette swatches per row
- content padding

Do not make this slice depend on auto-sized `LayoutRoot` support. Continue using an explicit
`COLOR_PANEL_WIDTH_GU` and tune it from the actual authored margin-box dimensions. The separate
LayoutRoot auto-dimensions task tracks the more general sizing capability.

## Event and ownership contract

The color panel should keep using `Selection` rather than replacing it with independent click
handlers. This preserves one shared source of truth and lets both the panel and its ancestors
observe the same selection transition.

For the first slice:

```text
palette Option click
  -> palette Selection state changes
  -> SelectionChanged bubbles through the panel hierarchy
  -> color panel updates the active-color square
  -> editor paint owner updates the effective paint color
```

The internal panel listener must not consume or replace the event in a way that prevents owning
editor systems from observing it.

## Later Color A / Color B extension

Color A and Color B are two stored color slots. They are useful when an operation needs two colors,
such as a gradient or a two-material object, and otherwise provide places to retain colors that are
not part of the built-in palette. A future eyedropper can write sampled colors into the active slot.

That extension should introduce a second, independent single-selection scope for choosing which
slot is active:

- the palette selection answers “which built-in color was clicked?”
- the slot selection answers “is Color A or Color B active?”

Both selections emit their normal `SelectionChanged` events. Selecting a palette color writes that
RGBA value into the active slot and makes it the effective editor color. Selecting Color A or Color
B makes that slot's stored value the effective editor color. Clicking a slot should therefore work
through the same effective-color path as clicking a built-in palette color.

Possible later presentation: place Color A before the first palette row and Color B before the
second, with both wells visually separated from the palette. Do not build this topology in the
single-slot first slice.

## Acceptance criteria for the first slice

- [ ] MMS can disable built-in visual feedback on one `Selection` instance.
- [ ] Visual feedback remains enabled by default for every existing selection.
- [ ] A no-visual selection still updates all selection state and emits the same events and payloads.
- [ ] Selecting a color does not replace or obscure that swatch's authored RGBA background.
- [ ] One large active-color square appears to the left of the palette and spans approximately the
      palette's two-row height and two swatch-cell widths.
- [ ] The gap after the square is approximately three times the normal inter-swatch spacing.
- [ ] The panel is wide enough that all 16 palette colors remain in exactly two rows.
- [ ] Clicking any palette swatch updates the large square and the editor's effective paint color.
- [ ] Initial active-color display and initial editor paint color agree.
- [ ] Selection events remain observable by both the panel-local handler and ancestor editor owners.
- [ ] Existing palette payload routing and paint behavior continue to work on desktop and XR.

## Tests

- Unit-test a disabled-visual `Selection` over styled options:
  - selection state changes
  - `SelectionChanged` is emitted
  - the option background remains unchanged
  - no `selection_highlight` helper is created
- Confirm the default policy still applies and removes highlights exactly as before.
- Materialize `color_panel_body()` and assert the active-color well, palette selection, and 16
  payload-bearing swatches exist.
- Run layout and verify the well plus palette fit without clipping and the palette wraps to exactly
  two rows.
- Select at least two contrasting swatches and verify the active well tracks the latest RGBA value
  while the palette swatches retain their own colors.

## Related

- [Color panel palette and color-space picker](./color-panel-palette-and-color-space-picker.md)
- [LayoutRoot auto dimensions and computed size](./layout-root-auto-dimensions-and-computed-size.md)
- [Single-select option deselects on re-click](../bugs/single-select-option-deselects-on-re-click.md)

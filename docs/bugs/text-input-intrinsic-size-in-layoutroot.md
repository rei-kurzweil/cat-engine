# TextInput has no intrinsic size inside LayoutRoot

## Status

Open design/implementation tracker. No source changes made.

## Symptom

In the Pose panel, a pose library's editable asset name can extend beyond the
white input background and overlap the following header controls. The desired
result is for the editable input's containing layout item to grow to the
entered name's natural width, not for the panel to impose a larger fixed or
minimum width.

The immediate example is `pose_library_name_input` in the Bisket desktop Pose
panel. Its styled wrapper is currently fixed at 10.5 GU, so longer legal
library names have no matching expansion path.

## Confirmed cause

`LayoutRoot` intrinsic text measurement currently recognizes only a
`TextComponent` in a local content subtree. It reads that component's text,
authored wrapping settings, and font size, then uses the same text measurement
path as normal display text.

`TextInputComponent` owns editable text and caret state, but is not recognized
by that walker and does not offer an equivalent intrinsic measurement contract.
Consequently an auto-sized styled wrapper around a TextInput has no editable
text contribution to its width or height. The rendered glyphs can therefore
outgrow the layout box and its layout-owned background.

This is a general `TextInput` × `LayoutRoot` capability gap, not a
pose-panel-only style bug.

## Current authored topology

```text
pose_library_header (flex)
|- pose_library_name_wrap (styled white background; fixed 10.5 GU today)
|  `- TextInput(asset_name_draft)
|     `- Style(font size 1.2 GU)
|- Capture
|- Reset
`- Save
```

The white rectangle belongs to the styled wrapper / layout-owned background,
not to the glyph renderer. It can only follow the name if the wrapper receives
the input's intrinsic layout dimensions.

## Required behavior

- An auto-width layout item containing a TextInput derives its intrinsic width
  from the current editable value and effective text style.
- Its intrinsic height uses the same wrapping, font-size, line-height, and
  glyph-unit rules as a comparable non-editable `TextComponent`.
- As text changes, layout is invalidated so the input background, hit region,
  and following flex siblings are updated together.
- A parent with a definite available width still applies its normal overflow,
  wrapping, and flex rules. Intrinsic input sizing must not make controls
  silently overlap each other.
- Explicit `Style.width` / `Style.height` remain authoritative; this work
  enables `auto`, not a global TextInput minimum width.
- TextInput-specific caret, selection, focus, and glyph-hit helpers remain
  interaction metadata and do not become a second text layout algorithm.

## Proposed seam

Extend the existing local-content text measurement query to identify a
TextInput and obtain its current value plus the effective text style. Reuse the
same `TextSystem::measure` / wrapping rules already used for `TextComponent`;
do not copy character advance or wrap calculations into TextInput code.

The result should be a shared text-layout input abstraction rather than making
the Pose panel inspect text itself. That keeps regular Text and TextInput in
sync when typography or wrapping behavior changes.

## Investigation and acceptance coverage

1. Put a `TextInput` alone in an auto-sized `inline-block` inside a
   `LayoutRoot`; verify width and height match a `TextComponent` with identical
   text and style.
2. Change the input from short to long text and back; verify its layout-owned
   background, click region, and following flex items move in the same layout
   update without requiring a manual refresh.
3. Test effective font-size and word-wrap behavior, including a constrained
   parent width.
4. Verify explicit input-wrapper dimensions still override intrinsic size.
5. Replace the Pose library name wrapper's fixed 10.5 GU width with auto only
   after this general contract is working; verify long library names no longer
   overpaint Capture/Reset/Save.

## Related

- `src/engine/ecs/system/layout/measure.rs`
- `src/engine/ecs/component/text_input.rs`
- `src/engine/ecs/system/editor/pose_panel.rs`
- `docs/task/text-input-layout-position-for-index.md`
- `docs/task/layout-owned-text-transforms.md`
- `docs/bugs/text-input-editing-rebuilds-too-much.md`

# Review: Text and TextInput ownership in LayoutRoot

Status: current implementation review, following TextInput intrinsic sizing.

## Executive model

`TextInput` is the authored, interactive, and layout-facing component. Its
internally generated `TextComponent` is a private rendering/text-layout target.
They describe one editable text value, not two independent layout items.

```text
styled layout item                 owns box, flow position, background
`- TextInput                       owns value, focus, caret, edit semantics
   |- authored Style                owns input typography / text overrides
   `- __text_input_content          private runtime topology
      `- __text_input_text: Text    glyph layout + generated text block
         `- __text_block            private glyph container
            `- glyph transforms
```

The outer styled item is what LayoutRoot measures and positions. `TextInput`
is the semantic source of editable content within that item. The inner `Text`
exists so TextSystem can continue to own glyph generation, wrapping, caret
coordinates, and the stable private text-block topology shared by normal Text
and inputs.

## Normal Text

For authored display text:

```mms
T {
    Style { display("inline-block") }
    Text { "Save" }
}
```

Layout measurement walks local content beneath the styled transform and finds
the first `TextComponent`, subject to layout boundaries. It uses the same
`TextSystem::measure` routine used by rendering to calculate the intrinsic
width and wrapped height.

An auto-width `inline-block` uses that intrinsic width (capped by available
inline space). A block item normally fills its available width and uses
intrinsic text primarily for auto height. Later in the layout pass, layout
updates the found Text's effective font size and wrap settings, then asks the
text system to rebuild its glyphs if those values changed.

The important boundary rule is that plain transform wrappers are local content,
while nested styled items, HTML elements, and nested LayoutRoots are separate
layout boxes. A parent must not measure text that belongs to one of those child
boxes.

## Why TextInput needed explicit handling

For authored editable text:

```mms
T {
    Style { display("inline-block") background_color(...) }
    TextInput { "library_name" Style { font_size(1.2) } }
}
```

TextInput initialization creates its inner Text later at runtime. The authored
Style is a direct child of `TextInput`, not of that generated inner Text. The
old local-content walk saw the input's Style as the start of a nested styled
box and stopped before it reached the generated Text. Consequently the input
contributed no intrinsic width or height even though its glyphs were visible.

That explains the previous Pose-panel symptom precisely:

```text
fixed wrapper/background box
    + visible TextInput glyphs beyond its right edge
    = text overpainted following controls
```

It was not a background-quad sizing problem in isolation. The background was
correctly following its styled wrapper; the wrapper lacked a measurement for
the editable content it contained.

## Current bridge contract

The layout measurement query now recognizes a TextInput before applying the
nested-styled-item boundary rule.

For a TextInput it reads:

- **current editable value** from `TextInputComponent.text` (authoritative,
  including an edit that may be queued for glyph rebuild);
- **font/wrap defaults** from its generated Text when initialized, or normal
  Text defaults before initialization; and
- **direct input Style** as the nearest text-style owner, ahead of the
  containing layout item's style.

It then calls `TextSystem::measure`, exactly as normal Text measurement does.
There is no second character-advance, line-break, or font-sizing algorithm in
TextInput.

The same bridge is used during the layout application pass to locate the inner
Text and synchronize its effective font size and wrapping. Thus the following
three things use the same effective text contract:

```text
TextInput current value + direct Style
  -> intrinsic width/height used by LayoutRoot
  -> effective inner Text font/wrap used by TextSystem
  -> generated glyphs and caret coordinates
```

Layout does interact with the generated inner Text, but only as the private
rendering target for the enclosing TextInput. It must not treat that inner Text
as a sibling layout item, independently position it in flow, or allow its
private `__text_block` / glyph subtree to become authored layout topology.

## Edit and invalidation path

```text
keyboard edit
  -> TextInputSystem changes TextInput.text
  -> SetText targets the generated inner Text for glyph rebuild
  -> mutation executor recognizes TextInput ownership
  -> enclosing LayoutRoot(s) are marked dirty
  -> intrinsic input measurement, background, and sibling positions refresh
```

The invalidation is intentionally limited to Text owned by a TextInput. Plain
`SetText` retains its previous behavior, avoiding broad relayout churn from
status labels and other display-only text updates.

## Pose-library result

`pose_library_name_wrap` is now `display: inline-block; width: auto`. Its
white layout-owned background therefore grows with `pose_library_name_input`'s
intrinsic TextInput width and height rather than relying on the old fixed
10.5-GU width.

This resolves the field/background mismatch, but it does **not** promise that
an arbitrarily long name fits inside the current Pose-panel shell.

## Why the panel does not grow with the input

`LayoutComponent` / LayoutRoot has an authored `available_width` (and optional
available height). Its layout pass measures children and records a
`computed_size_wu` result, but does not feed that result back into its own
available width or into the surrounding panel transforms.

The ownership direction is currently:

```text
panel shell's authored width
  -> LayoutRoot available width
    -> flex/inline child allocation and intrinsic measurement
      -> LayoutRoot computed content size (reporting only)
```

There is no content-to-panel sizing feedback loop. That is deliberate for
fixed editor panels and avoids a recursive parent/child layout cycle, but it
means an intrinsic input can push header siblings past the panel's right edge
when their combined preferred widths exceed the shell.

### Interim option

If the immediate desktop presentation needs more room, increasing the shared
Pose panel width from 29.5 GU to the previous 38.35-GU probe is a valid,
localized presentation workaround. It does not replace the TextInput fix and
does not create an authorable per-panel sizing API.

### Separate future design

Making a panel grow to content needs an explicit parent/shell policy, not a
change to TextInput measurement. Options to evaluate separately:

- a panel-shell `width: auto` / max-width policy with a well-defined viewport
  cap;
- a per-panel `EditorUI` width configuration contract; or
- a responsive header policy (wrap, overflow menu, or horizontal scrolling)
  for controls that cannot all fit on one line.

Any such design must state which box may grow, which ancestor constrains it,
and how it converges without a LayoutRoot measuring itself through descendants.

## Invariants and regressions to preserve

- TextInput always has one semantic editable value; the generated Text is not
  separately authorable data.
- Text and TextInput share `TextSystem::measure` and glyph layout rules.
- Input-specific focus, caret, whitespace hit areas, and selection remain
  TextInput interaction metadata rather than layout boxes.
- Explicit dimensions still override intrinsic dimensions.
- A nested styled child or LayoutRoot remains an intrinsic-measurement
  boundary for its parent.
- Layout does not serialize or reparent TextInput's private generated content.
- Plain Text updates do not acquire TextInput's relayout behavior.

## Relevant implementation and follow-up documents

- `src/engine/ecs/system/layout/measure.rs`
- `src/engine/ecs/system/text_input_system.rs`
- `src/engine/ecs/system/text_system.rs`
- `src/engine/ecs/system/editor/pose_panel.rs`
- `docs/spec/layout-intrinsic-text-measurement.md`
- `docs/task/layout-owned-text-transforms.md`
- `docs/bugs/text-input-intrinsic-size-in-layoutroot.md`
- `docs/bugs/bisket-desktop-pose-panel-apply-button-clipped.md`

# Text-owned block transforms for layout

Status: phase 1 implemented, including the repository example/component audit. Per-line alignment
is deferred to phase 2.

Related:

- [Text origin contract for layout-aligned UI](text-origin-contract-for-layout-aligned-ui.md)
- [VTuber slide-deck XR placement and controls](vtuber-slidedeck-xr-placement-and-controls.md)

## Goal

Every initialized `TextComponent` owns exactly one stable private transform which contains all
generated glyph transforms:

```text
TextComponent
└── __text_block Transform       generated, non-serialized, stable across rebuilds
    ├── glyph Transform          generated, non-serialized
    ├── glyph Transform
    └── ...
```

The text system owns this node's creation and lifetime. Layout may position it, but must not
bootstrap Text internals. MMS authors should not need an otherwise meaningless `T {}` merely to
make `text_align` or `vertical_align` work.

## Ownership contract

- `TextSystem` creates `__text_block` before it creates glyphs.
- Rebuilding text removes/replaces glyph children but preserves the block transform and its ID.
- All generated glyph transforms are direct children of `__text_block`.
- `__text_block` carries `Serialize.off()` and is treated as private generated topology.
- An authored transform outside `Text` remains author-owned placement state.
- Layout may emit transform updates for `__text_block`; it never creates or deletes it.
- `TextInput` uses the same Text-owned block rather than defining a second glyph-container model.

## Phase 1: one block transform around all glyphs

Phase 1 establishes topology and whole-block placement only.

- [x] Add an idempotent `TextSystem` helper which finds or creates `__text_block`.
- [x] Parent normal glyphs, text-input whitespace hit areas, and shadow glyphs beneath it.
- [x] Preserve the block while clearing glyphs during `SetText` rebuilds.
- [x] Migrate TextInput hit-testing and known direct-glyph-child assumptions.
- [x] Let layout target `__text_block` for a naked `Text` while preserving explicit authored
  wrapper behavior.
- [x] Cover stable identity, serialization-off, rebuild cleanup, and raw-Text alignment in tests.
- [x] Remove author `T` wrappers whose only purpose was giving layout an alignment target.

The first migrated scene was `examples/vtuber-slidedeck.mms`. The follow-up audit removed the
remaining identity-only wrappers from `examples/fit-bounds-demo.mms` and the shared UI examples
under `assets/components/`. Authored transforms which position, scale, rotate, animate, clip, own
interaction, define a separate styled layout item, or provide a stable named query target remain.
In particular, non-zero text depth/lift transforms are not part of this mechanical cleanup.

The block's phase-1 origin follows the current glyph coordinate behavior. The separate
[text-origin contract](text-origin-contract-for-layout-aligned-ui.md) owns the top-left/half-glyph
cleanup so this topology change does not silently combine two foundational migrations.

## Phase 2: wrapped-line alignment

Deferred intentionally:

- [ ] Make `text_align("left" | "center" | "right")` affect each wrapped line, not only the
  complete text block.
- [ ] Decide whether TextSystem applies per-line glyph offsets directly or owns private line
  transforms beneath `__text_block`.
- [ ] Measure alignment from the effective post-wrap line widths.
- [ ] Add mixed-width explicit-newline and automatic-wrap coverage.

Phase 1 may center the complete rectangle occupied by all text. It does not promise that every
short line inside that rectangle is independently centered.

## Compatibility rules

Given an explicit authored wrapper:

```mms
T {                    // author-owned
    Text { "hello" }   // owns private __text_block
}
```

layout may continue positioning the explicit wrapper where existing behavior requires it. The
private block remains the glyph container and stays identity unless Text/layout needs it.

Given naked text:

```mms
T {
    Style { text_align("center") }
    Text { "hello" }
}
```

layout finds the descendant Text's `__text_block` and positions that block. It must not synthesize
an authored-looking wrapper.

## Acceptance criteria

1. Every built Text has exactly one `__text_block` transform.
2. Repeated registration and `SetText` preserve its component ID.
3. No generated glyph transform remains a direct child of Text.
4. Raw Text can participate in whole-block horizontal and vertical alignment.
5. Existing explicitly wrapped Text remains functional.
6. TextInput glyph hit testing and caret behavior remain functional.
7. Serialization does not emit the private block or glyph topology.

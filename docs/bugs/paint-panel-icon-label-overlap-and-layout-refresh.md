# Paint-panel icons overlap labels before and after layout refresh

## Status

Open cross-example layout regression. No source changes made.

## Symptom

In at least `vtuber-desktop` and `bisket-desktop-demo`, Paint-panel tool icons
initially overlap their labels (for example, Free Draw). A later layout dirty /
UI refresh changes the arrangement, but it still does not look correct.

The report has not yet checked an XR example, so XR coverage is explicitly
unknown rather than assumed unaffected.

## Why this is a distinct bug

Existing paint-panel reports cover missing icons, oversized icons, brush
selection, and block-versus-inline tool layout. This report specifically tracks
the temporal behavior:

1. first materialization has icon/text overlap;
2. a layout refresh changes it;
3. the refreshed result remains visibly wrong.

That shape points to conflicting initial intrinsic measurement, layout-owned
placement, and presentational icon scaling—not simply a wrong static icon
scale.

## Current authored shape

Each Paint tool is an inline-block tile with a fixed 7.5 GU height. Its icon
slot has a 4.0 GU block height, while the icon itself is a scaled renderable
subtree. The label is a subsequent block containing `Text`.

Recent work allowing scaled renderables with bounds to contribute intrinsic
layout dimensions is a strong suspect. If icon bounds now feed back into the
layout box that is supposed to contain the icon, first-pass measurement and
later layout invalidation can disagree. That would violate the required
ownership direction:

```text
LayoutRoot/container resolves the tile box
  → icon presentation is fitted/placed within that box
```

not:

```text
scaled icon bounds redefine the tile box
  → label placement moves or overlaps
```

This is a hypothesis; it needs before/after box measurements.

## Repro matrix

| Scene | Mode | First render | After layout refresh |
|---|---|---|---|
| `vtuber-desktop` | desktop | verify overlap | verify remaining defect |
| `bisket-desktop-demo` | desktop | verify overlap | verify remaining defect |
| one XR editor scene | XR | not yet tested | not yet tested |

## Required measurements

- Tile, icon-slot, icon-renderable, and label resolved local boxes before and
  after the first layout pass.
- Which component marks the tile/layout root dirty, and why.
- Intrinsic measurement source for the scaled icon subtree.
- Whether the icon slot's explicit 4.0 GU height is preserved or overwritten.
- Whether the label receives its expected block-flow Y position and hit box.

## Desired behavior

- Tool icon and label never overlap on first presentation.
- The first completed layout is visually stable; later dirty passes do not
  materially rearrange the tile without an authored state change.
- The tile's layout box is owned by layout; icon fitting/scaling is
  presentational and cannot resize the label's flow container.

## Related

- `docs/bugs/paint-panel-oversized-icons-and-incorrect-brush-selection.md`
- `docs/bugs/paint-panel-tool-layout-inline-block.md`
- `docs/task/fit-bounds-layout-container-and-presentational-subtree.md`
- `assets/components/internal/panel_items.mms`
- `assets/components/internal/panels.mms`

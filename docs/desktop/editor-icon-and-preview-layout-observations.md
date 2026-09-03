# Desktop editor icon and preview layout observations

Date: 2026-09-03

Status: expected-state baseline. Record observed desktop behavior in the last
column before deciding whether the shared fix belongs in layout placement,
preview fitting, or a specific panel.

[Back to editor/grid/paint workbench](editor-grid-paint.md)

## Shared expectation

For a styled transform that paints the immediate visual container:

```mms
T {
    Style {
        width(...)
        height(...)
        background_color([1, 1, 1, 1])
    }
    // icon or preview content
}
```

the `Style` owns the container's resolved box and background. Its icon or
preview is presentational content inside that box:

- it is entirely inside the intended content region, subject to deliberate
  padding;
- it is centered in the region unless the UI explicitly asks for another
  alignment;
- it preserves aspect ratio when fitted;
- it does not alter an explicit container width or height; and
- it cannot move adjacent text, overlap the label, or escape into a sibling's
  background.

The background need not literally be white in the shipped UI. White is a
useful diagnostic because it makes the intended container boundary obvious.

## Observation table

| Surface | Authored visual container | Expected desktop result | Actual desktop observation |
|---|---|---|---|
| Paint tool tile icon | The Paint tile is `7.0 × 7.5` GU; its icon slot reserves `4.0` GU height above the label. The tile background is on the outer styled transform. | Each tool icon is wholly within its 4 GU icon region, centered horizontally and vertically as a group. It has consistent visual breathing room across Pencil, Grid, Line, Spray, Color, and Erase. The label begins below the icon region, is readable, and never overlaps the icon. | _Pending user observation._ |
| Asset-panel preview | The asset tile is `8.5` GU wide; `preview_slot` is `8.5 × 5.0` GU. The unavailable-preview placeholder is the current explicit painted background; successful previews replace it with a separate preview shell. | The preview is wholly within the `8.5 × 5.0` slot, centered in both axes and uniformly fitted with visible margins. It does not resize the slot or push/cover the asset label below. If the white placeholder is visible, its text is centered in the same slot. | _Pending user observation._ |
| Grid panel delete / X button | The delete button itself has a red styled background of `3.5 × 2.3` GU; `delete_x_icon` is a separately scaled child transform. | The X is visually centered in the red button with even apparent left/right and top/bottom margins. Both diagonal arms remain inside the button; it neither affects the adjacent inline controls nor crowds them. | _Pending user observation._ |

## What a mismatch would mean

- Correct backgrounds/slots but misplaced visual content: investigate the
  renderable's placement or fitting transform.
- Icon or preview changes the background size or label flow: investigate an
  intrinsic-measurement ownership leak.
- Correct after a refresh but wrong on first display: record the first and
  second completed layout states separately; that may identify an invalidation
  or preview-bootstrap timing defect.

## Current implementation shape (for diagnosis)

- Paint icons use a manually authored scale beneath the 4 GU icon slot.
- Asset previews use an `asset_preview_shell` with separately computed scale
  and offset beneath the fixed preview slot.
- The Grid delete X uses a manually authored scaled icon wrapper beneath the
  fixed button.

None of these production paths currently use `FitBounds`; they are three
instances of the same desired ownership rule implemented separately.

## Related trackers

- [Layout-owned visual content is misaligned across editor UI](../bugs/layout-owned-visual-content-misaligned-in-editor-ui.md)
- [Paint-panel icons overlap labels before and after layout refresh](../bugs/paint-panel-icon-label-overlap-and-layout-refresh.md)
- [FitBounds layout-container targeting and presentational subtree split](../task/fit-bounds-layout-container-and-presentational-subtree.md)

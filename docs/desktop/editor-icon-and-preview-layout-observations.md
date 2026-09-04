# Desktop editor icon and preview layout observations

Date: 2026-09-03

Status: desktop observation recorded for `bisket-desktop-demo`; root cause is
not yet confirmed. The differing corner direction and the correctly centered
asset exceptions are important discriminators, not noise.

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

## Editor ownership rule versus general intrinsic layout

The engine needs both behaviors, but a given visual subtree must have one
unambiguous sizing owner.

### Built-in editor UI

For Paint icons, asset previews, Grid controls, and other built-in editor
visuals, layout owns the available slot size. The renderable is uniformly
fitted and aligned inside that resolved slot. Its fitted bounds must not feed
back into the slot's explicit width or height or alter adjacent text flow.

Built-in icons should also use a normalized authoring convention: centered
local bounds with a consistent nominal extent, such as fitting within a
`1 × 1 × 1` cube. Normalized source assets make icon sizing predictable, but
layout/fitting must still use the measured AABB rather than assume the origin
is centered. Asset previews can contain arbitrary authored geometry and cannot
depend on the normalization convention.

### General authored layout and data visualization

An auto-sized styled transform may intentionally derive intrinsic width and
height from a transformed renderable subtree. This is the useful graph/bar
case: differently scaled bars produce differently sized layout items, and
normal inline `vertical_align("bottom")` can make their item boxes share a
baseline.

That intrinsic mode preserves the authored renderable scale. It must not also
fit the renderable back into the auto-sized box derived from that same
renderable, because that would create circular sizing ownership.

## Layout-managed metadata involved

Layout currently manages two internal children on styled transforms:

- `__layout_bounds` stores the resolved content and padding AABBs in the
  styled transform's local coordinate system.
- `__layout_visual_placement` stores the selected visual root's complete
  pre-placement AABB and a layout-owned translation. Transform propagation
  composes that translation outside the authored transform, preserving its
  scale and rotation.

The second component is the extra origin/basis bridge: renderable bounds are
centered or otherwise authored in graphics coordinates, while a layout content
box starts at its content origin and extends down local negative Y.

The current placement calculation always centers X and aligns the visual's
bottom edge to the content box's bottom edge. It does not consult an editor
fit/alignment policy and it does not scale the visual. This is a strong
explanation for the consistent bottom placement in the observations: a rule
introduced for bottom-aligned visual content is being applied to fixed editor
slots whose intended result is centered fitting. The remaining left-versus-
right differences still require comparing source AABB coordinates and subtree
selection.

## Observation table

| Surface | Authored visual container | Expected desktop result | Actual desktop observation |
|---|---|---|---|
| Paint tool tile icon | The Paint tile is `7.0 × 7.5` GU; its icon slot reserves `4.0` GU height above the label. The tile background is on the outer styled transform. | Each tool icon is wholly within its 4 GU icon region, centered horizontally and vertically as a group. It has consistent visual breathing room across Pencil, Grid, Line, Spray, Color, and Erase. The label begins below the icon region, is readable, and never overlaps the icon. | In `bisket-desktop-demo`, Free Draw, Grid Tool, Line, Spray Can, Color, and Erase all appear left-justified at the bottom-left of their white tile backgrounds. The icons overlap their respective labels there; the labels are also at the bottom-left rather than flowing below an icon region. |
| Asset-panel preview | The asset tile is `8.5` GU wide; `preview_slot` is `8.5 × 5.0` GU. The unavailable-preview placeholder is the current explicit painted background; successful previews replace it with a separate preview shell. | The preview is wholly within the `8.5 × 5.0` slot, centered in both axes and uniformly fitted with visible margins. It does not resize the slot or push/cover the asset label below. If the white placeholder is visible, its text is centered in the same slot. | In `bisket-desktop-demo`, icons from `icons.mms` are centered over the bottom-right of their preview slots. The names appear broadly correct: they wrap at the full tile width and stay near the bottom. Most primitive previews are also bottom-right. The truss is centered correctly. The Star, Icosahedron, Heart, and Partial Annulus 2D previews also look roughly centered; all other observed primitives are bottom-right. |
| Grid panel delete / X button | The delete button itself has a red styled background of `3.5 × 2.3` GU; `delete_x_icon` is a separately scaled child transform. | The X is visually centered in the red button with even apparent left/right and top/bottom margins. Both diagonal arms remain inside the button; it neither affects the adjacent inline controls nor crowds them. | In `bisket-desktop-demo`, the red X is at the bottom-right of its red button instead of centered. |

## What a mismatch would mean

- Correct backgrounds/slots but misplaced visual content: investigate the
  renderable's placement or fitting transform.
- Icon or preview changes the background size or label flow: investigate an
  intrinsic-measurement ownership leak.
- Correct after a refresh but wrong on first display: record the first and
  second completed layout states separately; that may identify an invalidation
  or preview-bootstrap timing defect.

### Current comparison

- Paint is not just an icon-fitting failure: both icon and label collapse into
  the tile's bottom-left. That points to a parent/child layout placement or
  coordinate-origin failure before the separate icon-scale wrapper is even the
  only concern.
- The Grid X and most asset previews are bottom-right, while Paint is
  bottom-left. A single hard-coded "place every visual at the lower corner"
  explanation is therefore insufficient; compare the root-local AABB `min`,
  `max`, and center with each target box.
- The correctly centered truss, Star, Icosahedron, Heart, and Partial Annulus
  2D previews show that the asset panel's slot and label flow can work. Their
  bounds/origin or preview-tree shape likely differs from the bottom-right
  cases and should be the first before/after comparison pair.

### Focused triage reproduction

`examples/triage/layout-visual-placement-alignment.mms` isolates external
line-box alignment from internal bounded-visual alignment:

- The short orange and tall cyan item boxes are correctly bottom-aligned.
- The three white slots are correctly sized and positioned as 10-GU-wide by
  6-GU-tall rectangles, with visible margins between them.
- The red, green, and blue visual shapes appear horizontally centered on the
  right edge of their slots and vertically below the slot by about one cube
  height, despite authoring `vertical_align("top")`,
  `vertical_align("middle")`, and `vertical_align("bottom")` respectively.

This reproduces both parts of the suspected gap: visual placement ignores the
authored internal alignment. The slot backgrounds themselves are not currently
the cause. Instrumentation identifies an exact double placement: generic
`apply_text_align()` mistakes the non-text visual transform for a zero-sized
text anchor and writes `[+0.4, -0.24]` into its authored position. The bounded-
visual placement path then adds its own correction. Removing that accidental
text-placement write should put the cubes back inside the slots; separately,
bounded visual placement still needs to honor the intended internal Style
alignment to distinguish top, middle, and bottom.

The working-tree fix now implements both separations: text alignment only
selects text-bearing transforms, and bounded visual placement maps
`text_align`/`vertical_align` to the source and target AABB edges or centers.
The focused reproduction should therefore render the red cube at the top, the
green cube in the middle, and the blue cube at the bottom, with all three
horizontally centered and their authored transforms preserved.

### Post-fix editor observation

The focused cubes and most production editor visuals now align correctly. The
Grid delete X is centered. Two narrower follow-up cases remain:

- Asset previews: Icosahedron, Star, Heart, Partial Annulus 2D, and the truss
  now place their visual origin at the preview slot's top-left corner. These
  were the previews that appeared centered before the general fix. The four
  primitives use procedurally generated meshes, while the truss is produced by
  `CombineMesh`; unlike ordinary built-in meshes, their final bounds are not
  available during the asset shell's first measurement. The asset system has a
  `pending_remeasure` queue and a post-layout remeasurement pass, but
  `build_asset_item_shell()` currently never adds an unmeasurable preview to
  that queue. The removed non-text fallback had accidentally supplied the
  half-slot centering translation for these bounds-late previews.
- Paint tiles: the icon slot itself is authored with
  `text_align("center")`/`vertical_align("middle")`, but the label's own styled
  block has no `text_align`, so its effective/default placement is left. The
  outer tile is centered, but `apply_text_align()` can select its unstyled
  direct wrapper by finding text inside a deeper styled label item; that moves
  the entire icon-plus-label layout using the label's text measurement. Text
  discovery should stop at nested styled layout-item boundaries, and the label
  block should explicitly author `text_align("center")`.

Adding `text_align("center")` to the Paint label confirms a further wrapped-
text gap: single-line labels center, but wrapped labels remain left-aligned.
`apply_text_align()` currently measures the label with `wrap_at = 0` (the
unwrapped width). When that width exceeds the content box, its centering offset
is clamped to zero. The text system subsequently generates wrapped lines from
that left-edge origin. Measuring the resolved wrapped width would center the
multiline block as a whole; CSS-like centering of every line additionally
requires per-line offsets during glyph layout because one transform translation
cannot center lines of different widths independently.

These are consequences of previously relying on the accidental generic text
fallback, not reasons to restore that fallback. Bounds-late preview centering
belongs in the preview remeasurement lifecycle; Paint icon and label alignment
belongs to their respective styled slots.

## Current implementation shape (for diagnosis)

- Paint icons use a manually authored scale beneath the 4 GU icon slot.
- Asset previews use an `asset_preview_shell` with separately computed scale
  and offset beneath the fixed preview slot.
- The Grid delete X uses a manually authored scaled icon wrapper beneath the
  fixed button.

None of these production paths currently use `FitBounds`; they are three
instances of the same desired ownership rule implemented separately.

## Related trackers

- [Style-driven placement of bounded visual content](../task/style-driven-layout-visual-placement.md)
- [Layout-owned visual content is misaligned across editor UI](../bugs/layout-owned-visual-content-misaligned-in-editor-ui.md)
- [Paint-panel icons overlap labels before and after layout refresh](../bugs/paint-panel-icon-label-overlap-and-layout-refresh.md)
- [FitBounds layout-container targeting and presentational subtree split](../task/fit-bounds-layout-container-and-presentational-subtree.md)

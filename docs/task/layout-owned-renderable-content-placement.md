# Task: Layout-owned placement for bounded renderable content

## Goal

Let a renderable subtree with known bounds participate in block and inline
layout without requiring MMS authors to calculate translations from its width,
height, center, or origin.

Data-driven authoring should specify the geometry and its scale:

```mms
T {
    Style { display("inline-block") width(4.5) vertical_align("bottom") }
    Text { "12" Style { display("block") } }
    T.scale(1.5, value * 0.7, 1.5) {
        R.cube() {}
    }
}
```

It should not need expressions such as `width / 2.0`, `height / 2.0`, or a
hand-authored Z lift. Those offsets are consequences of layout coordinates and
the measured AABB, so Mittens should own them.

## Current state

Transform-aware intrinsic measurement now finds the scaled cube and gives its
styled ancestor the expected width and height. Inline `vertical_align("bottom")`
therefore aligns the three outer bar boxes correctly.

Placement is still missing. The measurement path reduces the visual AABB to
width and height, while the layout pass places only the outer styled transform.
The renderable subtree retains its graphics-space origin:

- a centered cube spans both sides of local X=0, while a layout content box
  spans X=0..width, so the cube overlaps the box's left edge;
- layout content advances down local -Y, while a centered cube spans both sides
  of Y=0, so much of the cube falls outside the box vertically; and
- the correct outer box can therefore be visible in `InspectLayout` even though
  the geometry supplying its intrinsic size is visibly misplaced.

Manually translating the cube by half its width and height merely duplicates
layout math in MMS. It is not an acceptable fix and remains fragile for
off-center meshes, imported assets, rotations, or unions of several renderables.

## Required behavior

- Preserve the complete root-relative intrinsic AABB through measurement and
  placement; do not discard `min`, `max`, or `center` after deriving extents.
- Treat the eligible bounded presentational subtree as atomic renderable
  content inside the styled item's content box.
- Normalize graphics-space bounds into layout coordinates automatically.
- Horizontally center the bar geometry in the wider authored content box.
- Vertically place its visual bottom on the content box's bottom edge, leaving
  any separately measured label space above it.
- Apply layout placement without destroying the subtree's authored scale,
  rotation, animation, or data-driven transform.
- Keep nested styled transforms as independent layout boundaries.

For a measured visual AABB `visual` and resolved content box `content`, the
placement conceptually needs the deltas:

```text
dx = center_x(content) - center_x(visual)
dy = bottom_y(content) - min_y(visual)
```

The exact Y expression must follow Mittens' local layout convention, where the
content box extends from Y=0 downward into negative Y. The important contract is
edge/center alignment based on measured bounds, not a hard-coded half extent.

## Proposed architecture

Attach a runtime-owned `LayoutVisualPlacementComponent` directly beneath the
existing authored transform that roots the eligible visual content:

```text
styled layout item
├── Style / Text / independent styled children
└── authored visual Transform       keeps authored scale/rotation/animation
    ├── __layout_visual_placement    layout-owned metadata component
    └── renderable subtree
```

The layout system should:

1. identify the same presentational content used by intrinsic visual
   measurement;
2. retain its complete pre-placement AABB and visual transform root in the
   current `VisualContentMeasurement`;
3. create or reuse one `LayoutVisualPlacementComponent` on that transform;
4. write the measured AABB and computed parent-local translation to the
   metadata component; and
5. leave the authored `TransformComponent` unchanged.

`TransformSystem`, not `LayoutSystem`, applies the metadata when resolving the
effective local matrix:

```text
effective_local = translation(layout_offset_parent_local) * authored_local
```

This preserves authored TRS while making rendering, BVH, raycasting, and
descendants observe one corrected world transform. Intrinsic measurement must
compose authored transforms while excluding the prior layout offset, so
repeated layout passes cannot feed back or drift.

The component contract is specified in
[`layout-visual-placement-component.md`](../spec/layout-visual-placement-component.md).

## Decisions to settle

- How presentational content is selected when a styled item contains several
  plain transform/renderable branches. V1 should prefer one common direct
  transform root; a later extension may place several roots from one measured
  union.
- Whether horizontal centering and bottom alignment are the default for atomic
  renderable content or are selected through existing/new Style alignment
  properties. Either choice must remain declarative and must not require AABB
  arithmetic in MMS.
- How labels and renderable content divide the content box. The chart requires
  label flow above a bottom-aligned visual region rather than both occupying the
  same origin.
- When cached bounds or transforms change, how the owning layout root is marked
  dirty so placement is recomputed.
- How layout-owned placement metadata is exposed by raw component inspection.
  It must be ignored by authored serialization and content/layout traversal.

## Acceptance coverage

- An origin-centered cube with only a data-driven scale is centered
  horizontally in a wider inline-block and touches its content bottom.
- Unequal scaled bars share one bottom baseline under
  `vertical_align("bottom")` with no bounds-derived translations in MMS.
- An off-center mesh is aligned from its actual AABB rather than an assumed
  half width or half height.
- Multiple eligible renderables use their transformed union as one atomic
  visual bound.
- Authored scale, rotation, animation, and explicit inner transforms survive
  repeated layout passes unchanged.
- No transform node is inserted and no authored subtree is reparented.
- Nested styled children remain independent layout items and receive no visual
  placement metadata from their ancestor.
- The JSON-file data-visualization example meets the visual result with only
  its bar scale driven by the parsed values.

## Relationship to existing tasks

`transform-aware-intrinsic-layout-bounds.md` solves measurement: which visual
bounds contribute to auto size. This task solves the second half: placing the
bounded content into the box that measurement produced.

`fit-bounds-layout-container-and-presentational-subtree.md` remains concerned
with uniformly fitting content into an explicit target. This task must not
implicitly rescale content; it preserves the authored scale and computes only
layout-owned placement.

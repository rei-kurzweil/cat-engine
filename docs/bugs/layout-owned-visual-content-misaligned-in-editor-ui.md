# Layout-owned visual content is misaligned across editor UI

## Status

Open cross-panel visual regression. No source changes made.

## Symptom

Editor UI icons and asset previews are broadly no longer placed in the center
of their intended containers. Depending on the panel and subtree, they appear
crammed into a lower-left or lower-right corner of the container, or of an
ancestor container. Paint-panel icons also overlap their labels, especially on
first materialization.

This has been observed in the desktop `vtuber-desktop` and
`bisket-desktop-demo` editor UIs. XR has not yet been checked.

## Context and likely seam

Recent transform-aware intrinsic layout work correctly allows scaled bounded
renderables to contribute a visual width and height to an auto-sized layout
item. That makes previously hidden ownership/placement errors visible:
measurement can know the renderable's AABB extents while placement still uses
the graphics-space origin instead of normalizing the full AABB into the
resolved layout content box.

The known placement contract says that the complete bounds (`min`, `max`, and
center), not only width and height, must be retained through layout. A
center-origin icon or preview otherwise naturally straddles the local origin
while its layout box runs from its content origin down/right, yielding the
observed corner bias. This is a leading hypothesis, not yet a confirmed common
root cause for every panel.

## Scope

This tracker is intentionally broader than the Paint tool-label defect:

- Paint tool icons and labels;
- Assets-panel previews;
- other editor panel icons/renderable thumbnails;
- initial materialization versus later dirty-layout passes; and
- containers with an explicit slot size as well as auto-sized visual content.

It should determine whether all reports share one regression in
`LayoutVisualPlacementComponent` eligibility/offset/invalidation, or whether
some panels are instead using `FitBounds` or a legacy wrapper incorrectly.

## Required measurements

For one representative icon and one representative asset preview, record
before and after the first layout refresh:

- resolved `LayoutBoundsComponent` content box for the styled item and icon
  slot;
- visual subtree's root-relative intrinsic AABB, including `min`, `max`, and
  center;
- authored local transform and runtime layout placement offset separately;
- the selected presentational root and whether a nested styled transform made
  it ineligible;
- effective world bounds after transform propagation; and
- the component/event that marks the owning LayoutRoot dirty.

Inspect the visual on screen and through `InspectLayout`; a correct outer box
with misplaced geometry is an important discriminating result.

## Required behavior

- A bounded icon or preview is visually aligned within its authored layout
  slot on the first completed layout pass.
- Placement preserves authored transform, scale, rotation, animation, and
  raycast behavior; layout supplies only the derived offset.
- Explicit layout dimensions remain owned by layout. Intrinsic visual bounds
  may size `auto` content, but must not redefine an explicitly sized icon slot
  or push a label into overlap.
- Later refreshes converge to the same placement unless authored data changes.
- Nested styled layout items remain independent placement/measurement
  boundaries.

## Repro matrix

| Surface | Scene | Initial layout | After dirty refresh |
|---|---|---|---|
| Paint tool tile | `vtuber-desktop` | inspect icon/label overlap and corner | inspect convergence |
| Paint tool tile | `bisket-desktop-demo` | inspect icon/label overlap and corner | inspect convergence |
| Asset preview tile | either desktop editor | inspect preview alignment | inspect convergence |
| one XR editor panel | XR | not yet tested | not yet tested |

## Related

- `docs/bugs/paint-panel-icon-label-overlap-and-layout-refresh.md`
- `docs/bugs/paint-panel-oversized-icons-and-incorrect-brush-selection.md`
- `docs/task/transform-aware-intrinsic-layout-bounds.md`
- `docs/task/layout-owned-renderable-content-placement.md`
- `docs/task/fit-bounds-layout-container-and-presentational-subtree.md`
- `docs/task/asset-preview-layout-resolution.md`
- `src/engine/ecs/system/layout/measure.rs`

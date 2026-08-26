# Task: Transform-aware visual bounds for intrinsic layout

## Goal

Make auto-sized styled transforms account for the visible bounds of eligible
descendant renderables after their nested transforms have been applied.  This
lets a layout item such as a data-visualization bar derive its height from a
scaled cube, and lets inline `vertical_align("bottom")` align unequal bars on
their visual baseline.

The feature is an engine/layout capability.  It is not an MMS, JSON, or
Mittens-host API concern: MMS already expresses the required component tree.

## Reproduction

`examples/data-viz-json-file.mms` builds one bar for each JSON value.  Each
bar has:

- an outer styled transform that participates in inline layout;
- a text label; and
- a translated/scaled cube below a nested transform.

Today the outer bar measures its direct text content, while
`find_renderable_local_bounds()` intentionally stops at nested transforms.
The scaled cube therefore contributes neither its height nor its transformed
position to the outer bar's intrinsic layout box.  All bar boxes consequently
look label-sized and `vertical_align("bottom")` has no differing heights to
align.

The existing cake/icon example is not evidence that this already works: its
icon wrappers have explicit `Style.width` and `Style.height`.

## Existing seams

- `BoundsSystem::measure_renderable_subtree_bounds()` already walks a full
  subtree and unions renderable AABBs in a requested root coordinate space,
  including intervening `TransformComponent`s.
- Layout uses a separate cached-bounds walker in `layout/measure.rs`.  It
  reads `BoundsComponent.local`, does not transform them, and stops at every
  nested transform.
- `BoundsComponent` is the appropriate layout-time source: it provides the
  cached local AABB without making layout depend on `RenderAssets`.
- `LayoutBoundsComponent` remains the output of resolved layout.  It is not a
  substitute for intrinsic visual input bounds.

Related work:

- `docs/draft/component-tree-bounds-measurement-v1.md` defines the broader
  renderable-versus-layout-aware bounds split.
- `docs/task/fit-bounds-layout-container-and-presentational-subtree.md`
  consumes resolved container bounds for fitting; it must not own intrinsic
  layout measurement.

## Proposed design

### 1. Share transform-aware cached-bounds projection

Extract the transform/AABB projection walk so both general bounds consumers
and layout can use one definition of root-relative visual bounds.  The layout
variant must:

- read cached `BoundsComponent.local` data;
- compose every eligible descendant transform into the styled root's local
  coordinate space;
- union the transformed AABBs; and
- remain a pure read-only query.

`BoundsSystem` keeps its render-asset-backed measurement entry point for
general geometry consumers.  The two paths may have different sources of mesh
bounds, but must share transform composition and AABB-union semantics.

### 2. Make visual intrinsic sizing automatic but bounded

For `Style.width(auto)` and `Style.height(auto)`, include eligible descendant
visual bounds by default.  No MMS style opt-in is required.

The traversal boundary is layout ownership:

- descend through presentational transforms and renderables;
- stop before a nested styled transform that is independently measured and
  placed as a layout item; and
- never use a descendant's already-resolved `LayoutBoundsComponent` as an
  implicit visual input to an ancestor.

This prevents independent layout branches from changing an ancestor's
intrinsic size through incidental presentation while still allowing a styled
wrapper to size around a transformed geometry subtree it owns.

### 3. Combine flow and visual contributions explicitly

Normal text/child-flow measurement and visual-bounds measurement are separate
inputs.  Text must no longer unconditionally short-circuit the visual path.
The implementation must define one root-local union/extent calculation for
both contributions so a label and its bar geometry are both represented.

Explicit `Style.width` and `Style.height` remain authoritative.  The visual
path applies only to auto dimensions and must not run a recursive layout pass
or create a parent/child measurement cycle.

### 4. Preserve a clear authored ownership shape

For the chart, the outer styled transform is the inline layout item.  The
scaled cube should be under a presentational transform subtree, not a nested
styled layout item.  A nested styled transform remains an intentional layout
boundary and owns its own size.

This distinction should be documented in the implementation and demonstrated
by the JSON example; it avoids treating every transformed descendant as an
unbounded layout contribution.

## Implementation checklist

- [ ] Add the shared cached-AABB transform projection helper and test its
  root-relative translation, scale, and union behavior.
- [ ] Replace layout's direct-renderable-only intrinsic query with the bounded
  visual query.
- [ ] Define and implement the normal-flow plus visual-bounds combination
  rule for auto width and height.
- [ ] Keep explicit dimensions and nested styled layout boundaries unchanged.
- [ ] Adjust `data-viz-json-file.mms` to use the supported presentational bar
  subtree shape.
- [ ] Retain `InspectLayout` as the manual diagnostic while developing the
  example.

## Acceptance coverage

- A styled auto-sized wrapper around a translated/scaled cube reports its
  transformed intrinsic width and height.
- Inline siblings with unequal visual heights and
  `vertical_align("bottom")` share a baseline.
- A nested styled/layout child does not contribute its renderable bounds to an
  ancestor's visual intrinsic measurement.
- A wrapper containing both text and geometry retains both contributions,
  without recursive layout or text masking the geometry.
- The JSON-file data-visualization demo displays distinct bar heights that are
  bottom-aligned in its chart container.

## Non-goals

- Measuring arbitrary third-party file/JSON data formats in MMS.
- Replacing `LayoutBoundsComponent` or changing its resolved-box contract.
- Making `FitBounds` perform layout measurement or mutate layout-owned
  transforms.
- Treating arbitrary nested layout subtrees as presentational bounds.

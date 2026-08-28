# Classify overlapping translucent layout backgrounds when layout changes

Status: proposed / investigation

Related:

- `docs/bugs/layout-background-transparency-order-varies-between-launches.md`
- `docs/task/layout-background-multilayer-transparency-correctness.md`
- `docs/task/single-layer-transparency-depth-write-contract.md`

## Goal

Determine, during layout reconciliation rather than every frame, which translucent
layout-generated background quads can safely use the single-layer path and which belong to an
overlapping multilayer group.

The result only needs to guarantee the intended front-facing appearance of a layout root. General
camera-dependent transparency classification for arbitrary 3D geometry is outside this task.

## Geometric model to investigate

For a layout root, derive its planar basis:

```text
x = layout horizontal basis
y = layout vertical basis
n = normalize(x cross y)
```

Treat generated background quads as polygons in the layout plane. Two translucent backgrounds are
potential multilayer participants when their projected polygons overlap in the layout `x/y` plane
and they occupy different stacking positions along `n`.

For ordinary axis-aligned layout rectangles, overlap can begin as a cheap 2D interval test. Nested
transforms, rotation, non-uniform scale, or future non-rectangular backgrounds may require projected
polygon bounds or a 2D separating-axis test. The investigation should choose the simplest model
that matches the actual layout contract.

The analysis can produce an overlap graph:

- each translucent generated background is a node;
- an edge means two backgrounds overlap when viewed along the layout normal;
- isolated nodes are candidates for the single-layer path;
- connected overlapping groups require ordered multilayer composition.

## Invalidation boundary

Prefer recomputation only when information affecting the classification changes:

- the layout root is marked dirty;
- a generated background is added or removed;
- background alpha changes between opaque and translucent;
- layout-owned bounds, stacking depth, or relative transforms change;
- content is attached, detached, or reparented within the layout scope.

If authored transforms can move a background relative to its layout root without dirtying layout,
either add the missing invalidation or conservatively classify that subtree as multilayer. A world
transform that moves or rotates the entire root should not require recomputing root-local overlap.

## Questions to resolve

- Which component owns the cached classification and overlap graph?
- Is layout-local projected overlap sufficient for the promised front-facing result?
- How are nested layout roots treated: one stacking scope, independent scopes, or a conservative
  multilayer boundary?
- Do translucent text, images, arbitrary authored renderables, and clipping helpers participate, or
  only generated `__bg` quads?
- How should nearly coplanar backgrounds and equal stacking priorities be ordered?
- Can the result supply a stable layout stacking key as well as a single/multilayer classification?
- What conservative fallback applies when the geometry no longer fits the planar model?

## Work tracker

- [ ] Specify the layout basis, normal direction, and front-facing convention.
- [ ] Inventory every transform path that can move a generated `__bg` relative to its layout root.
- [ ] Prototype root-local rectangular overlap detection during layout reconciliation.
- [ ] Build and cache overlap groups plus stable stacking order metadata.
- [ ] Wire all required dirty/invalidation events without introducing a per-frame scene scan.
- [ ] Add fixtures for isolated backgrounds, nested backgrounds, partial overlap, siblings, nested
      roots, rotated roots, and dynamic attach/detach.
- [ ] Compare classification cost and render savings against conservatively treating every
      translucent layout background as multilayer.
- [ ] Document fallback behavior for unsupported or ambiguous geometry.

## Complexity target

For the common rectangular case, prefer either:

- a simple pairwise `O(n^2)` test for small layout roots, with low constants; or
- a sweep/spatial-index approach if real layouts contain enough generated backgrounds to justify
  `O(n log n + k)`, where `k` is the number of overlaps.

The cache requires `O(n + k)` space for classifications and overlap relationships. This cost should
be paid when layout-relevant state changes, not once per camera or frame.

## Acceptance criteria

- Isolated translucent backgrounds remain eligible for the optimized single-layer path.
- Backgrounds that overlap along the layout normal are placed in correctly ordered multilayer
  groups.
- Repeated renders are deterministic regardless of component registration order.
- Moving or rotating an entire layout root does not trigger unnecessary local overlap analysis.
- Every relative-motion path either invalidates the cached result or uses a documented conservative
  fallback.

## Non-goals

- Exact visibility classification for every possible camera angle.
- Arbitrary intersecting or curved transparent world geometry.
- Replacing the immediate conservative correctness fix.


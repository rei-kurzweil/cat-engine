# Classify overlapping translucent layout backgrounds when layout changes

Status: proposed / investigation

Related:

- `docs/bugs/layout-background-transparency-order-varies-between-launches.md`
- `docs/task/layout-background-multilayer-transparency-correctness.md`
- `docs/task/single-layer-transparency-depth-write-contract.md`

Implementation tracker:

- `docs/task/visual-world-automatic-transparency-scope-transactions.md`

## Goal

Add an automatic transparency policy whose final draw-list classification is owned by
`VisualWorld`, while layout supplies the stable geometry and stacking facts needed to resolve that
policy when layout changes.

The immediate target is deciding which translucent layout-generated background quads can use the
single-layer path and which belong to an overlapping multilayer group. The result only needs to
guarantee the intended front-facing appearance within a layout transparency scope. General
camera-dependent transparency classification for arbitrary 3D geometry is outside this task.

## Feasibility conclusion

This is possible, but `VisualWorld` cannot infer the desired result from the current
`VisualInstance` alone.

`VisualWorld` already owns the relevant final decision:

- alpha and opacity decide whether an instance is opaque or transparent;
- `VisualInstance.multiple_layers` decides whether transparent content enters the cached
  single-layer stream or the view-sorted multilayer stream;
- draw-list caches are rebuilt when membership-affecting fields change.

That makes `VisualWorld` the natural owner of an `Auto` classification and its resolved queue
membership. However, a current `VisualInstance` only carries a world model matrix, material,
color/opacity, and a boolean `multiple_layers`. It does not know:

- which layout root or transparency scope owns the instance;
- the layout root's local `x/y` basis and normal;
- the generated background's root-local polygon or rectangle;
- its layout stacking order;
- whether moving the whole root should preserve the cached overlap result.

Trying to reconstruct those facts near the renderer from world matrices would either lose layout
semantics or require camera-dependent projected-overlap work whenever the camera or root moves.
The workable boundary is therefore:

> Layout publishes stable root-local overlap metadata; `VisualWorld` resolves automatic
> transparency and owns the draw lists.

## Proposed ownership boundary

### Layout owns geometry facts

When a dirty layout root reconciles its generated backgrounds, it produces or updates a compact
descriptor for each automatic transparent instance. A descriptor would conceptually contain:

```text
scope_id
polygon_or_rect_in_scope_xy
stacking_coordinate_or_order
front_normal_convention
fallback_policy
```

This is descriptive metadata, not a command saying “put this in the multilayer queue.” Layout does
not need to know renderer queue implementation details.

The descriptor may live in an internal component read by `RenderableSystem`, or be sent through a
targeted update intent. The exact transport is an implementation decision; `VisualWorld` should
not query ECS layout topology itself.

### VisualWorld owns policy resolution

Replace the current boolean-only choice conceptually with a policy such as:

```text
SingleLayer       explicit fast-path promise
MultiLayer        explicit correct ordered path
Auto(scope data)  unresolved engine-owned classification
```

The metadata itself can live in a side table keyed by `InstanceHandle` rather than enlarging every
GPU-facing instance record. `VisualWorld` would:

- group automatic candidates by transparency scope;
- recompute only scopes marked overlap-dirty;
- build overlap groups and stable stacking order;
- resolve isolated candidates to single-layer when the scope contract permits it;
- resolve overlapping groups to multilayer;
- mark the draw cache dirty only when resolved membership or order changes;
- default unresolved, incomplete, or unsupported automatic content to multilayer.

This creates the requested special registration/update path for transparent objects whose final
queue is not known when the renderable is first registered.

## Proposed data flow

```text
layout dirty
  -> layout computes root-local background rectangles and stacking facts
  -> RenderableSystem registers/updates VisualWorld Auto descriptors
  -> VisualWorld marks only the affected transparency scope dirty
  -> VisualWorld resolves overlap groups before rebuilding draw streams
  -> instances enter single-layer or multilayer lists
```

Registration order must not affect the result. An automatic instance that reaches rendering before
its descriptor group is complete uses the conservative multilayer fallback.

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

Because this metadata is expressed in scope-local coordinates, translating, rotating, or scaling
the whole layout root in the world does not change overlap membership and does not invalidate the
classification.

## Fundamental scope limitation

A layout-dirty-time analysis can prove that backgrounds do or do not overlap **inside that layout
scope**. It cannot prove the strict global single-layer condition against arbitrary transparent
world objects for every future camera pose.

For example, an isolated layout background may later move in front of transparent particles from a
different scene subtree without the layout becoming dirty. If the single-layer path writes depth,
that interaction can be incorrect even though the local layout classification was accurate.

The design must therefore choose and document one of these boundaries:

- layout scopes are isolated transparency compositions and are ordered as groups by the renderer;
- automatic single-layer means “single within this layout scope,” using a layout-specific path that
  does not claim the global depth-writing guarantee;
- cross-scope overlap is checked per view when needed, accepting camera-dependent work;
- or layout-auto content remains conservatively multilayer whenever global isolation is not known.

This is the main decision shared with
`docs/task/single-layer-transparency-depth-write-contract.md`. No root-local algorithm can remove
that global limitation by itself.

## Invalidation boundary

Prefer recomputation only when information affecting the classification changes:

- the layout root is marked dirty;
- a generated background is added or removed;
- background alpha changes between opaque and translucent;
- layout-owned bounds, stacking depth, or relative transforms change;
- content is attached, detached, or reparented within the layout scope.

`VisualWorld` also needs scope dirtiness when an automatic instance is registered, removed, changes
alpha classification, changes scope, or receives new overlap metadata.

If authored transforms can move a background relative to its layout root without dirtying layout,
either add the missing invalidation or conservatively classify that subtree as multilayer. A world
transform that moves or rotates the entire root should not require recomputing root-local overlap.

## Questions to resolve

- What is the exact `SingleLayer | MultiLayer | Auto` policy representation?
- Does automatic metadata live inline on `VisualInstance` or in a side table keyed by
  `InstanceHandle`?
- Which internal component or intent transports layout descriptors to `VisualWorld`?
- Does `VisualWorld` own the overlap graph directly, or only its resolved scope/order output?
- Is layout-local projected overlap sufficient for the promised front-facing result?
- How are nested layout roots treated: one stacking scope, independent scopes, or a conservative
  multilayer boundary?
- Do translucent text, images, arbitrary authored renderables, and clipping helpers participate, or
  only generated `__bg` quads?
- How should nearly coplanar backgrounds and equal stacking priorities be ordered?
- Can the result supply a stable layout stacking key as well as a single/multilayer classification?
- What isolation rule makes an automatically selected depth-writing single-layer path safe against
  transparent instances outside the layout scope?
- What conservative fallback applies when the geometry no longer fits the planar model?

## Work tracker

- [ ] Specify the automatic transparency policy and conservative unresolved state.
- [ ] Specify the layout-to-`VisualWorld` descriptor and lifecycle without adding ECS queries to
      `VisualWorld`.
- [ ] Specify the layout basis, normal direction, and front-facing convention.
- [ ] Inventory every transform path that can move a generated `__bg` relative to its layout root.
- [ ] Prototype root-local rectangular overlap resolution in a `VisualWorld` scope using metadata
      produced during layout reconciliation.
- [ ] Build and cache overlap groups, resolved policies, and stable stacking order metadata.
- [ ] Wire all required dirty/invalidation events without introducing a per-frame scene scan.
- [ ] Decide how layout scopes interact with transparent content outside their scope.
- [ ] Add fixtures for isolated backgrounds, nested backgrounds, partial overlap, siblings, nested
      roots, rotated roots, dynamic attach/detach, incomplete registration, and conservative
      fallback.
- [ ] Compare classification cost and render savings against conservatively treating every
      translucent layout background as multilayer.
- [ ] Document fallback behavior for unsupported or ambiguous geometry.

## Complexity target

For the common rectangular case, prefer either:

- a simple pairwise `O(n^2)` test for small layout roots, with low constants; or
- a sweep/spatial-index approach if real layouts contain enough generated backgrounds to justify
  `O(n log n + k)`, where `k` is the number of overlaps.

The cache requires `O(n + k)` space for descriptors, classifications, and overlap relationships.
This cost should be paid when layout-relevant state changes, not once per camera or frame.

The intended trade is a small amount of persistent CPU memory and dirty-time classification work in
`VisualWorld` in exchange for keeping isolated backgrounds on the fast path and avoiding a
camera-dependent scan of every layout background. Cross-scope correctness may still require a
separate per-view strategy depending on the chosen isolation contract.

## Acceptance criteria

- Isolated translucent backgrounds remain eligible for the optimized single-layer path.
- Backgrounds that overlap along the layout normal are placed in correctly ordered multilayer
  groups.
- `VisualWorld`, rather than layout authoring code, owns resolved transparent queue membership.
- Automatic registration is conservative until enough scope metadata exists to prove the faster
  classification.
- Repeated renders are deterministic regardless of component registration order.
- Moving or rotating an entire layout root does not trigger unnecessary local overlap analysis.
- Every relative-motion path either invalidates the cached result or uses a documented conservative
  fallback.

## Non-goals

- Exact visibility classification for every possible camera angle.
- Arbitrary intersecting or curved transparent world geometry.
- Teaching `VisualWorld` to traverse ECS layout topology.
- Replacing the immediate conservative correctness fix.



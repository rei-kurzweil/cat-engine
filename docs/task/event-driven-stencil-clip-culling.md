# Event-driven CPU culling for flat stencil clips

Status: deferred; GPU-cached deformation is the current renderer priority.

Epic: [Renderer optimisation](epic/renderer_optimisation.md)

## Problem

The existing `ClippingSystem` tracks active `StencilClipComponent`s, finds their source
renderables, assigns stencil depth, and resynchronizes affected subtrees. It does not retain which
renderables are governed by each clip, and `VisualWorld` has no clip-culled state.

Consequently, content that is completely outside a clip still enters phase orders and render
streams, is copied into instance buffers, and produces vertex/raster/stencil work before the GPU
rejects its fragments.

Testing every renderable against every clip every frame would replace GPU waste with avoidable CPU
work. Transform propagation is already event-driven, so clip visibility should be updated only
when a relevant transform, bound, clip registration, or topology relationship changes.

## Goal

Extend `ClippingSystem` into the owner of clip membership and conservative CPU visibility for
flat, 2D stencil shapes embedded in the 3D world.

Squares, stars, hearts, and other flat clip meshes remain exact GPU stencil masks. CPU culling is a
broad phase: omit a clipped renderable only when its bounds are fully outside the clip shape's
local 2D bounding box. Overlapping the bounding box remains visible and is resolved exactly by the
stencil test.

This approach is camera-independent for the intended coplanar UI hierarchy, so the same result can
be used for window, XR-eye, and mirror views viewed from different angles.

## Runtime indexes

Keep the following derived state inside `ClippingSystem`:

- A record per active clip containing its component, scope root, source renderable, source local
  bounds, and member renderables.
- A reverse map from renderable component to its ordered ancestor clips. Nested visibility passes
  only when the renderable overlaps every ancestor clip.
- The existing active-clip set remains the registration source of truth; all new indexes are
  rebuildable derived state.

The clip source participates as content of its ancestor clips, but never tests against itself.
Do not maintain a `TransformComponent -> affected renderables` index. Transform propagation
already discovers the renderables whose final world matrices changed, so duplicating that
dependency graph would add memory and topology-maintenance work.

## Membership lifecycle

- On clip registration, resolve its source renderable and scope root, enumerate renderables in the
  scope once, establish forward/reverse membership, and evaluate initial visibility.
- On renderable registration or deferred renderable flush, attach it to its ancestor clips and
  evaluate it once its `InstanceHandle` and bounds are available.
- On clip/renderable removal, clear both directions of membership before the ECS nodes or handles
  disappear.
- On attach, detach, reparent, or other topology refresh, rebuild membership only for the affected
  subtree and its old/new clip ancestry. Reuse the existing topology-triggered transform refresh
  path rather than adding a per-frame repair scan.
- On mesh or `BoundsComponent` changes, refresh the affected local bounds and reevaluate that
  renderable; changing a clip source's bounds reevaluates all members of that clip.

Missing source renderables, bounds, transforms, or non-invertible matrices must fail open: keep the
content drawable and retry when the relevant registration/update occurs.

## Event-driven visibility evaluation

Integrate culling directly with transform propagation rather than using `RxWorld`, emitted
signals, or clip-specific event handlers:

Here, “event-driven” means change-driven by the existing transform propagation call, not reactive
signal dispatch.

1. While `TransformSystem::propagate_subtree` computes final world matrices and calls
   `VisualWorld::update_model`, use that update's result to determine whether the instance is
   clipping-relevant.
2. Collect only clipping-relevant renderables whose effective world matrices were updated,
   including those reached through transform-stream outputs and transform-parent dependents.
3. Complete propagation first so clip sources and content both have settled world matrices.
4. Return or expose the changed-renderable collection to `SystemWorld::transform_changed`.
5. Pass that collection directly to `ClippingSystem` as a derived-state update stage.

Use reusable scratch storage and deduplicate component IDs before clip evaluation. Do not emit one
signal per renderable and do not route these internal updates through the reactive pipeline.

`VisualWorld::update_model` already resolves `InstanceHandle -> instance index`. Extend its return
value so the same lookup reports whether the updated instance has `stencil_ref > 0` or
`is_stencil_clip`. Do not follow `update_model` with another `VisualWorld::instance` lookup and do
not query a `ClippingSystem` hash set for every propagated renderable.

Use an explicit result shape:

```rust
pub enum ModelUpdateResult {
    Missing,
    Updated { clip_relevant: bool },
}
```

When `ClippingSystem` has no active clips, `SystemWorld` supplies no clipping-change sink:
propagation ignores `clip_relevant`, performs no scratch-vector writes or deduplication, and does
not call back into `ClippingSystem`.

The update result is a notification hint, not clipping ownership. It contains only the fact that
the updated instance is clipping-relevant; clip membership, bounds, and visibility decisions
remain private to `ClippingSystem`.

For each changed renderable:

- If it is a clip source, cache its new inverse world matrix once and nominate the members of that
  clip for reevaluation.
- If it is clipped content, nominate that renderable for reevaluation against its ordered ancestor
  clips.
- Deduplicate the nominated content so a moved clip and moved descendants do not evaluate the same
  renderable more than once in a propagation batch.

This direct path covers `UpdateTransform`, `UpdateTransformWorld`, transitions, layout movement,
transform streams, and transform-parent propagation because they converge on transform
propagation. Unrelated renderables are not added to the clipping change collection. Unchanged
frames perform no clipping work.

For each content/clip pair:

1. Read the content local `Aabb` and current world matrix.
2. Invert the clip source's world matrix.
3. Transform all eight content AABB corners by `clip_world_inverse * content_world`.
4. Project those conservative corners onto the clip-local XY plane.
5. Compare the resulting 2D AABB with the clip source's local XY AABB.
6. Mark the content clip-culled only when there is no overlap with at least one ancestor clip.

The first version deliberately does not perform polygon intersection against concave star/heart
silhouettes. Bounding-box false positives preserve correctness and leave exact rejection to the
GPU. Non-flat content or content outside the intended coplanar UI contract also fails open unless
the conservative test can prove it is outside.

## `VisualWorld` integration

- Add per-instance clip-culling state, separate from authored opacity, phase flags, and stencil
  depth.
- Provide an update method that dirties the draw cache only when the culling state changes.
  Transform changes still dirty instance data through the existing path.
- Change `update_model` from a boolean-only result to a small result value that reports whether
  the updated instance is clipping-relevant, reusing its existing handle-to-index lookup. Unknown
  handles still report that no update occurred.
- Exclude clip-culled instances from opaque, cutout, transparent, overlay, background, emissive,
  and other derived phase orders before batches or render streams are constructed.
- When a culled instance becomes visible again, it must re-enter its original phase with unchanged
  material, ordering, and stencil depth.
- Do not unregister instances or discard their transform/material state when culled.

The implementation must remain compatible with
[render streams as the single source for clip-capable phases](render-stream-single-source.md):
culling happens while phase membership is prepared, before stream construction.

## Out of scope

- Scissor rectangles or screen-space clip evaluation.
- Exact CPU polygon intersection for concave clip silhouettes.
- Occlusion culling between ordinary scene objects.
- Per-camera visibility differences.
- Replacing stencil rendering for partially overlapping content.
- `RxWorld` signals or event-handler dispatch for transform-to-culling synchronization.
- A persistent `TransformComponent -> affected clipped renderables` dependency index.
- A per-renderable clipping membership lookup from inside transform propagation.
- Moving clip bounds tests or clip membership ownership into `TransformSystem`.

## Test plan

- Square clip: content fully inside, partially overlapping, and fully outside in clip-local XY.
- Rotated/scaled 3D UI root: local culling results remain unchanged when the entire clip/content
  hierarchy is viewed or transformed from another angle.
- Star or heart clip: content outside the silhouette but inside its bounding box remains drawable
  for exact GPU stencil rejection; content outside the bounding box is culled.
- Nested clips: content is culled when outside either ancestor and restored only when it overlaps
  all ancestors.
- Transform events: moving content across a boundary toggles culling; moving a clip reevaluates
  its members; unrelated transforms trigger no clip visibility evaluations.
- Update-result filtering: `VisualWorld::update_model` reports ordinary, clipped-content,
  clip-source, and unknown-handle cases correctly without a second instance lookup.
- Propagation coverage: direct transforms, transform streams, transform-parent dependents, and
  topology-triggered world-transform refreshes all report their changed renderables after final
  matrices settle.
- Deduplication: moving a common ancestor of a clip source and its content evaluates each affected
  content renderable at most once in that propagation batch.
- Topology events: registering, attaching, reparenting, detaching, and removing clips/renderables
  update forward/reverse membership without stale entries.
- Missing bounds or singular clip transforms fail open without panics or disappearing content.
- Phase coverage: culled instances are absent from every applicable phase/order/stream and return
  with the same phase and stencil depth when visible.
- Verify window, XR, and mirror rendering for a rotated clipped UI.

## Performance acceptance

- An unchanged frame performs zero CPU clip-overlap tests.
- Transform propagation collects only changed renderables that `VisualWorld::update_model`
  identifies as clipped content or clip sources.
- A content transform tests only those collected renderables against their ancestor clips.
- A clip-source transform tests only members of that clip, with nested visibility resolved through
  the existing clip ancestry index.
- Transform-to-culling synchronization performs no reactive signal emission or handler dispatch.
- Transform-to-culling synchronization performs no additional handle-to-instance or clipping
  membership hash lookup per propagated renderable.
- A scroll-panel workload records before/after counts for overlap tests, emitted instances,
  instance-buffer bytes, draw instances, and CPU draw-cache preparation time.
- The optimized workload demonstrates that fully outside items are absent from renderer work
  while partially overlapping items retain exact stencil rendering.

## Completion criteria

- `ClippingSystem` owns forward membership and reverse clip ancestry; it does not duplicate
  transform dependencies.
- `VisualWorld::update_model` reuses its existing instance lookup to identify clipping relevance;
  transform propagation reports only those changed renderables to `ClippingSystem` after their
  final world matrices have settled.
- Clip visibility updates are event-driven, with no per-frame scan of all renderables.
- Conservative clip-local culling supports flat arbitrary stencil shapes through their local
  bounds and never changes their exact stencil silhouette.
- `VisualWorld` omits culled instances without losing their persistent state.
- Nested clips, topology changes, missing data, and visibility restoration are covered by tests.

# Task: Generalized mesh bounds visualization and `CombineMesh` aggregate AABBs

## Goal

Make the editor’s **Show bounds** control visualize bounds for all supported
mesh outputs, not just imported GLTF renderables.  As the first new consumer,
each `CombineMesh` output must own one aggregate bounding box, even when the
default mode has removed all of its source renderables.

This is an interim spatial layer.  An aggregate AABB supports visualization,
culling, and broad-phase ray candidates; it is not a precise mesh hit test.
The hierarchical mesh-BVH task remains responsible for triangle-accurate
intersection.

## Why now

`CombineMesh {}` currently registers its baked output directly in
`VisualWorld`.  Its sources are suppressed and, by default, removed from the
live World.  Consequently there is no remaining `RenderableComponent` with a
`BoundsComponent` for the existing GLTF-specific bounds visualization path to
find.  A truss therefore has neither a useful aggregate bounds overlay nor a
single broad-phase bounds representation.

The existing editor toggle is also named/implemented around GLTF even though
the useful user-facing concept is mesh bounds.  Static primitive meshes,
generated meshes, GLTF mesh primitives, and future combined meshes should be
able to report bounds through the same mechanism.

## Existing seams

- `BoundsComponent` stores a local `Aabb` alongside an ECS renderable.
  `RenderableSystem::cache_resolved_mesh_bounds(...)` creates it from a
  resolved CPU mesh.
- `BoundsSystem::measure_renderable_subtree_bounds(...)` can union descendant
  renderable bounds, transformed into a requested root coordinate space.
- `GltfBoundsVisualizationSystem` currently reads bounds only from spawned
  GLTF renderables and creates non-selectable wireframe-box marker subtrees.
- Editor Settings toggles `GLTFBoundsVisibility` by changing
  `GLTFComponent::bounds_visible`; it does not address arbitrary mesh output.
- `CombineMeshSystem::bake(...)` creates a root-local `CpuMesh`, then directly
  registers it in `VisualWorld`.  Its root transform is known at registration
  time, but no bounds record is retained.
- `BvhSystem` is presently a broad-phase BVH keyed only by ECS
  `RenderableComponent`.  It can later consume the same aggregate bounds, but
  this tracker does not require changing its topology or adding triangle BVHs.

## Scope and non-goals

In scope:

- a shared bounds-provider/output representation for mesh-backed visuals;
- generalized editor bounds overlays;
- authoritative aggregate AABBs for CombineMesh;
- an explicit temporary raycast-priority policy for AABB-only CombineMesh
  candidates.

Out of scope:

- per-triangle mesh BVHs, barycentric hit information, or exact mesh picking;
- skinned current-pose mesh bounds/intersection (except defining the interim
  exclusion/fallback); and
- multiple-material CombineMesh output.

## Design

### 1. Generalize from GLTF markers to mesh-output bounds

Introduce a runtime-owned bounds descriptor, for example:

```rust
MeshOutputBounds {
    owner: ComponentId,
    local: Aabb,
    model: TransformMatrix,
    kind: NormalRenderable | GltfPrimitive | CombineMesh,
}
```

The exact type/name can follow existing graphics/system ownership patterns.
The essential properties are:

- no requirement that the output be an authored `RenderableComponent`;
- local bounds and current model matrix are available together;
- `owner` is the stable component selected/reported by the editor;
- lifecycle removal is tied to the underlying output, not to an editor marker;
- the descriptor can later be promoted into the common spatial-instance API.

Normal renderables and GLTF primitives may initially adapt their existing
`BoundsComponent` data into this descriptor.  Do not duplicate or serialize
runtime overlay marker components into authored MMS.

Replace the GLTF-only visualization traversal with a generalized bounds
visualization system that consumes these descriptors.  It should create one
wireframe marker per eligible mesh output, retain the current non-selectable,
overlay, non-serialized behavior, and remove markers when the toggle is off
or the source output disappears.

The existing UI wording/row should become **Show mesh bounds** (migration of
the internal names can be incremental).  It must apply to all outputs beneath
effective editor roots, including CombineMesh roots.

### 2. Calculate CombineMesh bounds correctly

For every successful bake, calculate bounds in the CombineMesh root’s local
space and retain them with the registered output.

The authoritative calculation is:

```text
baked CpuMesh vertex positions -> local Aabb
```

This is preferable to treating a renderer instance as a mesh source: it is
exactly the geometry that was actually registered.  It naturally includes
each source’s transform baked relative to the CombineMesh root.

Before removal, also optionally measure source subtree bounds via
`BoundsSystem::measure_renderable_subtree_bounds(...)` in the same root space
for assertions/diagnostics.  For supported static meshes, the measured union
and baked-mesh AABB should agree within a small floating-point tolerance.
If they differ, the baked output AABB wins and the mismatch should be visible
in debug logging/tests; it indicates transform, mesh-override, or bounds
coverage drift.

The aggregate world AABB is `local.transformed(root_model)`.  On outer
transform changes, update that world representation without rebaking the
mesh.  On a `keep_transforms()` source edit/rebake, replace bounds and visual
output together so no stale marker or spatial candidate persists.

Skinned descendants remain excluded until current-pose snapshot semantics are
implemented.  Their absence must be explicit in the aggregate bounds result,
not silently represented as bind-pose geometry.

### 3. Temporary raycast policy

Once CombineMesh aggregate AABBs enter broad phase, an AABB-only combined
candidate can produce a false-positive hit in gaps between bars.  Until a
per-mesh triangle BVH exists:

- classify it as `AabbApproximate` rather than a normal precise mesh hit;
- give it lower raycast/selection priority than exact UI or primitive hits at
  comparable distance;
- preserve normal nearest-distance ordering within the same hit-quality tier;
- expose the hit quality in debug output so approximate selection is obvious;
- allow a deliberate fallback: if it is the only candidate, selecting the
  CombineMesh outer owner is acceptable.

This priority must be a policy at ray-hit resolution, not a magic material,
VisualWorld ordering trick, or a permanent exception in UI selection code.

## Implementation phases

### Phase 1: bounds registry and CombineMesh producer

- Define the runtime descriptor/registry and lifecycle API.
- Register/update/remove a CombineMesh aggregate bounds descriptor alongside
  its direct VisualWorld output.
- Compute the local AABB from the baked `CpuMesh`; add source-union comparison
  coverage for static meshes.
- Keep the record across default source collapse; update only its world bounds
  when the outer transform changes.

### Phase 2: generalized editor overlay

- Refactor or replace `GltfBoundsVisualizationSystem` to consume the common
  registry, retaining GLTF overlay behavior.
- Change the Settings toggle to control all mesh bounds below effective editor
  roots.
- Ensure markers neither appear in authored World-panel rows nor influence
  picking, CombineMesh source discovery, or serialization.

### Phase 3: approximate broad-phase integration

- Admit CombineMesh bounds to the broad-phase as a non-ECS runtime entry.
- Implement `AabbApproximate` ray-hit quality and lower-priority resolution.
- Do not claim exact surface intersection; defer that replacement to the
  hierarchical mesh-BVH task.

## Acceptance criteria

- Enabling Show mesh bounds displays one box around each collapsed truss, with
  no boxes for its deleted bars.
- The truss box matches the baked output geometry in CombineMesh-root local
  space and follows only the outer transform without forcing a rebake.
- GLTF bounds overlays continue to work, now through the generalized path.
- Bounds markers are runtime-only, non-selectable, non-serialized, and absent
  from authored World-panel rows.
- A rebuilt/removed CombineMesh output cannot leave an orphan marker or bounds
  descriptor.
- Before triangle BVHs, any CombineMesh AABB ray hit is labeled approximate
  and loses priority to comparable exact hits; it can still select the outer
  CombineMesh when it is the only viable hit.

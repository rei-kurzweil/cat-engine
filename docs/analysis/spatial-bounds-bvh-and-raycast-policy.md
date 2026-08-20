# Analysis: bounds, BVH, and raycast policy for aggregate meshes

## Prompt

`CombineMesh` and `GLTF` can represent complex geometry behind one logical
scene object.  We need a spatial policy that keeps broad-phase work cheap,
provides useful editor bounds, and supports accurate narrow-phase ray hits
without forcing every object through an analytic primitive test.

This is an analysis/epic seed, not a source-change task.

## What exists today

### Bounds representation

- `BoundsComponent` and mesh-output bounds use `Aabb`: an axis-aligned box in
  the object's local coordinate system.
- `Aabb::transformed(model)` transforms its corners and returns a new
  **world-space AABB**.  Rotation therefore enlarges the world-aligned broad
  bound; it does not create an OBB data type.
- `CombineMeshSystem` calculates an AABB over the baked mesh in
  CombineMesh-root-local space and records it in `MeshBoundsSystem` with the
  root's current model matrix.
- `BoundsVisualizationSystem` draws the local box as a child of a
  transform-parented marker.  The marker inherits the target model, so a
  rotated truss displays as a rotated wireframe box.  This is visually an
  oriented box and is useful, but it does **not** mean the BVH stores or tests
  an OBB.

### Broad phase

- `BvhSystem` stores one `RenderableAabb` per raycastable renderable using the
  `bvh` crate's world-space `AABB`.
- Its AABB is produced from mesh/local bounds transformed by the renderable's
  world model.  Rotation is conservatively folded into a world-aligned AABB.
- Transform changes queue refits; adds/removals rebuild as needed.
- The current BVH leaf identity is a renderable component ID.  A phase-1
  CombineMesh output is registered directly in `VisualWorld`, rather than as
  an authored `RenderableComponent`; its `MeshBoundsSystem` entry currently
  serves editor visualization but is not itself a BvhSystem leaf.

### Raycasting / narrow phase

- The current `raycast_renderables` path traverses broad-phase AABBs and then
  returns a ray/AABB distance.  It is not triangle-accurate.
- There is a sorted candidate API intended for future narrow-phase rejection
  and fall-through.
- Analytic primitive tests can remain a narrow-phase fast path for primitives,
  but imported and combined triangle meshes need a geometry-based path.

## Desired layered model

```text
scene/object BVH (world AABBs)
  -> logical object / output leaf
       -> optional local mesh BVH (local AABBs, triangles in leaves)
            -> narrow hit: analytic primitive or triangle intersection
```

The outer BVH should answer “which logical outputs might the ray hit?” and
remain cheap to refit under object transforms.  A local mesh BVH should answer
“which triangles in this GLTF/CombineMesh output does the ray hit?” after the
ray is transformed into object-local space.  This lets a complex truss use one
outer leaf while retaining accurate holes/openings in narrow phase.

## OBBs: likely role and limits

An OBB can be a better object-level rejection test for rotated elongated
objects such as trusses.  It should not replace the outer world-AABB BVH:
standard BVH traversal benefits from world AABBs, while an OBB is a useful
secondary narrow/between-phase test on a selected candidate.

Potential candidate flow:

1. Traverse the world-AABB BVH.
2. For a candidate with an OBB, transform the ray into the object's local
   frame and intersect the local AABB.  This is equivalent to an OBB test in
   world space and avoids adding a separate OBB math representation initially.
3. Traverse the output's local mesh BVH or run an analytic primitive test.
4. Return the nearest accepted hit, with the logical owner and optional source
   provenance.

This can improve false-positive rejection for rotated objects, but triangle
narrow phase is still required for holes and non-convex geometry.

## Ownership and hit policy to decide

- A collapsed `CombineMesh {}` should normally select/hit the CombineMesh
  owner/root, because its source nodes no longer exist in the live world.
- `CombineMesh.keep_transforms()` needs an explicit policy: hit the group by
  default, or preserve a baked mapping from triangle range back to source
  transform/renderable for editor selection and diagnostics.
- GLTF needs comparable output/scene-node provenance policy rather than a
  different ad-hoc result shape.
- Bounds visualization is editor affordance; collision, picking, selection,
  and culling may each need different accuracy/cost policies.

## Investigation slices, later

1. Audit all uses of `Aabb::transformed` and the bounds marker so terminology
   clearly distinguishes local AABB, displayed oriented box, and world AABB.
2. Add a ray-to-local-space helper and test it against rotated/scaled boxes,
   including non-uniform scale and singular-transform rejection policy.
3. Define a shared `SpatialOutput` / hit-result abstraction capable of ECS
   renderables, GLTF outputs, and CombineMesh outputs.
4. Prototype a local CPU mesh BVH and triangle-hit payload for GLTF, then
   CombineMesh.  Decide CPU retention/upload lifecycle and rebuild timing.
5. Add outer-BVH leaves for non-ECS `MeshBoundsSystem` outputs, or make the
   output representation first-class in BvhSystem.
6. Benchmark world-AABB only versus world-AABB plus local-AABB/OBB rejection
   versus local mesh BVH on long rotated trusses and dense GLTF scenes.

## Acceptance direction for a future task

- A rotated truss's visible bounds remain oriented with its root transform.
- Broad phase remains conservative and fast.
- Rays through a truss opening do not select its enclosing broad bound once
  triangle narrow phase is enabled.
- GLTF and CombineMesh produce the same structured hit contract, including
  logical owner and optional source provenance.
- No system infers accurate picking from a bounds-visualization marker.

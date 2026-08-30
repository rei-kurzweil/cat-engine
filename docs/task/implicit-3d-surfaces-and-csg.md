# Task: Implicit 3D surfaces and CSG for terrain

Date: 2026-08-29

Status: investigation / MVP design required; depends on
[mesh CSG operations](mesh-csg-operations.md)

## Motivation

The anime VN staircase background needs smooth rolling terrain where a stair
and path interrupt the hillside.  Flat and rotated planes can establish the
ground, but cannot provide the intended soft transitions beside the staircase.

The desired workflow is:

1. combine several overlapping implicit spheres into a smooth terrain-like
   surface;
2. bake that field into a normal mesh; then
3. use constructive solid geometry (CSG) to cut it cleanly against the
   staircase, landings, path, and world boundary.

This task defines the implicit-field half of the workflow.  The general
post-bake CSG operation is intentionally designed and implemented first in
[mesh CSG operations](mesh-csg-operations.md).  It is linked from
[the anime VN staircase-background task](anime-vn-staircase-background-example.md).

## Questions to resolve before implementation

- Which Rust crate(s), if any, can generate an isosurface from scalar-field
  samples with acceptable licensing, maintained dependencies, and a `CpuMesh`
  compatible output path?
- Is marching cubes sufficient for the MVP, or is dual contouring required for
  sharp CSG-adjacent features?  The expected answer is marching cubes first.
- What field blend is needed for overlapping spheres: hard union, metaball
  addition, or smooth minimum?  The MVP needs an explicit, controllable
  smooth-union/blend-radius value; it must not hide the blend behavior.
- How does the completed CSG operand contract accept an `ImplicitSurface`
  baked mesh without leaking CSG concepts into scalar-field evaluation?

Record the selected crates, versions, licenses, numerical limitations, and a
small benchmark in this document before committing to public MMS syntax.

## Proposed conceptual model

`ImplicitSurface` is both an operation and a nesting component, analogous
to `CombineMesh`: primitives nested inside it describe sources, and the
component owns one baked output mesh.  Unlike `CombineMesh`, its nested
primitives describe scalar fields rather than direct visible renderables.

`ImplicitSphere` is a child-only field primitive that contributes a sphere
to the nearest owning `ImplicitSurface` ancestor.  A standalone implicit
sphere is invalid and should produce an actionable authoring error.

The unqualified names are deliberate: this first operation generates a 3D
triangle mesh, so a `3D` suffix adds no useful distinction.  A future 2D field
feature would need a separately designed output contract (contours, filled
planar mesh, or extrusion) rather than overloading this component.

Illustrative syntax only:

```mms
ImplicitSurface {
    voxel_size(0.25)
    bounds([-12.0, -2.0, -12.0], [12.0, 8.0, 12.0])
    iso_level(0.0)
    smooth_min_radius(0.8)

    ImplicitSphere.center(-4.0, 0.0, 0.0).radius(5.5) {}
    ImplicitSphere.center( 1.0, 0.4, 2.0).radius(6.0) {}
    ImplicitSphere.center( 6.0, 0.0,-2.0).radius(4.0) {}
}
```

Names are deliberately provisional.  The public API must describe the field,
its sampling bounds, world-space voxel size, isovalue, and smoothing
semantics—not expose only an opaque pre-baked mesh.  The implementation may
enforce a maximum grid dimension and reject an overly fine voxel size for the
chosen bounds.

## MVP scope

1. `ImplicitSurface` component:
   - owns bounded field evaluation and the generated renderable/mesh;
   - supports explicit axis-aligned local bounds;
   - takes a world-space `voxel_size`, while enforcing safe per-axis and total
     sampling limits internally;
   - supports an explicit isolevel and world-space smooth-min radius;
   - invalidates and rebakes when its field children or relevant properties
     change; and
   - has clear ownership, visibility, and editor-selection behavior comparable
     to `CombineMesh`.
2. `ImplicitSphere` component:
   - has local center/transform and radius;
   - contributes to the nearest implicit-surface ancestor; and
   - supports overlapping spheres with documented smoothing behavior.
3. Mesh generation:
   - start with a CPU marching-cubes-style generator;
   - generate positions, normals, indices, and stable enough winding for
     normal materials/lights; and
   - register the output through the existing render-assets pathway.
4. CSG integration after the prerequisite task:
   - make a baked `ImplicitSurface` a supported operand of the completed CSG
     operation;
   - demonstrate a difference with closed stair/path clearance cutters; and
   - preserve CSG's documented tolerances and failure behavior.
5. Validation scene:
   - a small MMS example with 2–4 overlapping spheres, visible smoothing
     changes, and a simple cutter; and
   - later, replacement of the staircase task's temporary hillside helper.

## Explicit non-goals

- unbounded/infinite fields;
- arbitrary user-authored scalar functions in MMS;
- runtime sculpting or per-frame field remeshing;
- material/texture-painting redesign;
- a full general-purpose CSG language; or
- promising topologically perfect results from arbitrary hostile meshes.

## Component and lifecycle requirements

### Nesting and ownership

- An implicit-surface root discovers only its owned nested field primitives,
  not spheres belonging to a nested/adjacent implicit surface.
- The generated mesh is owned by the root and appears to rendering, bounds,
  raycast, editor, serialization, and removal systems as one aggregate.
- Source components remain represented enough for editor inspection and MMS
  round-trip.  Do not lose authored field parameters merely because the mesh
  was baked.
- Decide whether the sources collapse by default or follow a
  `CombineMesh.keep_transforms()`-like policy; document the choice and test it.

### Field semantics

- Define the sphere field as the actual signed distance function
  `length(p - center) - radius`: negative is inside, zero is the surface, and
  positive is outside.  With that convention, `iso_level(0.0)` is the normal
  and unsurprising default.
- Combine spheres using a documented smooth-min function.  Its
  `smooth_min_radius` parameter is measured in world units and controls the
  width of the blend region; a radius of zero (or an explicit hard-union mode)
  gives ordinary SDF union.
- In the MVP, `ImplicitSphere` supports translation plus a radius or uniform
  scale only.  Reject non-uniform scaling with an authoring error; ellipsoids
  are a later, intentional primitive rather than an approximate sphere SDF.
- Validate finite numeric inputs, positive radii, nonempty bounds, and bounded
  resolution before allocating a sampling volume.

### CSG seam

CSG operates only after `ImplicitSurface` produces a normal triangle mesh.
The initial useful operation is mesh **difference** defined by the linked
[mesh CSG operations task](mesh-csg-operations.md):

```text
smooth sphere field -> baked terrain mesh -> subtract stair/path cutters
```

This lets large overlapping spheres make curvature while explicit staircase
geometry retains crisp edges.  `CSG` must not be nested inside
`ImplicitSurface` in this slice: the field system evaluates only field
primitives, and the CSG system consumes its baked result as an operand.  Keep
cutter geometry and operation order visible in MMS/serialization.  Never
silently apply a failed boolean result; emit a useful diagnostic and preserve
the last known-good mesh if possible.

## Investigation plan

1. Inventory existing `CpuMesh`, `Renderable`, `CombineMesh`, mesh bounds,
   asset registration, and serialization seams.
2. Complete the CSG crate, lifecycle, tolerance, and operand-contract decision
   in [mesh CSG operations](mesh-csg-operations.md).
3. Research candidate isosurface-generation crates using their current
   upstream documentation, licenses, and small local proof-of-concept
   branches.
4. Benchmark representative volumes (for example 32³, 48³, and 64³ samples)
   and record mesh count, bake time, memory, and main-thread impact.
5. Prototype two overlapping spheres with hard union and smooth union; inspect
   normals, seams, and repeatable output.
6. Feed that baked mesh to the CSG operation and prototype stair/path cutter
   differences without holes, inverted normals, or catastrophic failure.
7. Decide the public MMS names only after these experiments establish what can
   be made reliable.
8. Add focused unit tests plus an MMS integration scene before connecting the
   terrain to the anime VN example.

## Acceptance criteria

- An MMS-authored bounded implicit surface built from overlapping spheres
  bakes into one lit, normal-correct mesh.
- Changing the documented smoothing control visibly and predictably changes
  the join between spheres.
- Invalid nesting and unsafe sampling parameters report authoring errors.
- Authored source parameters survive save/load or have an explicit documented
  round-trip limitation approved before release.
- The generated aggregate behaves correctly for removal, bounds, and editor
  selection.
- The CSG prerequisite has a documented, tested difference operation with
  known tolerances, and a baked `ImplicitSurface` succeeds as one of its
  supported operands.
- The staircase background can replace its temporary hill stand-in without
  changing its stair/path coordinate contract.

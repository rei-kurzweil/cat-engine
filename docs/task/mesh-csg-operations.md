# Task: Mesh CSG operations

Date: 2026-08-29

Status: proposed; prerequisite for implicit-surface terrain

## Chosen MVP backend: `csgrs`

Use [`csgrs`](https://crates.io/crates/csgrs), initially pinned to `0.20.1`,
as the CSG boolean backend.  It provides polygon-mesh BSP booleans—union,
difference, intersection, and xor—while leaving Mittens responsible for ECS,
MMS authoring, material policy, render-asset registration, and the derived
mesh lifecycle.

The crate has a deliberately broad CAD-oriented surface: 2D sketches,
import/export, text, metaballs, SDF meshing, physics conversion, and other
utilities.  None of that becomes part of the Mittens dependency contract for
this slice.  We need only its 3D `Mesh<()>`, polygon construction, triangle
conversion, and `difference` operation.

Start with an explicit minimal feature selection rather than the crate's
large default feature set:

```toml
# Cargo.toml; exact version until the adapter/regression suite is accepted.
csgrs = { version = "=0.20.1", default-features = false, features = ["f64", "delaunay"] }
```

`csgrs` requires one numeric feature and one triangulation feature.  Choose
`f64` for the offline boolean calculation even though Mittens render vertices
are `f32`: convert positions to `f64` at the adapter boundary, then validate
and convert the finished result back to `f32`.  This avoids making CSG
tolerance behavior unnecessarily fragile.  `delaunay` is the required
triangulation selection for the first adapter.  Do not enable the crate's
`sdf` or `metaballs` features yet; implicit surfaces remain a separate,
following task with its own evaluation and world-space voxel-size contract.

Before merging the dependency, run `cargo tree -i csgrs` and record the
resolved dependency impact.  The `f64` selection currently brings optional
physics-conversion dependencies through `csgrs`; they are tolerated only if
that impact is acceptable in the actual lockfile.  If it is not, reassess the
backend or upstream feature split before exposing public MMS CSG.

## Goal

Implement constructive solid geometry (CSG) as a general post-bake mesh
operation before adding implicit surfaces.  It must combine ordinary authored
mesh operands, including future baked `ImplicitSurface` output, without
requiring field primitives or isosurface generation.

The first motivating composition is:

```text
ImplicitSurface -> baked triangle mesh -> CSG difference with stair/path cutters
```

This task deliberately does **not** put CSG inside `ImplicitSurface`.
`ImplicitSurface` evaluates scalar fields and bakes them; CSG consumes baked
mesh operands afterward.  Keeping those operation domains separate is the
first-slice boundary.

## Engine adapter boundary

Keep `csgrs` behind a small internal adapter, rather than passing its types
through ECS components or the scripting registry:

```text
CpuMesh + operand local-to-CSG-root transform
    -> mittens_to_csgrs_mesh(...) -> csgrs::mesh::Mesh<()>
    -> .difference(...)
    -> csgrs_to_mittens_mesh(...) -> CpuMesh
    -> RenderAssets registration / derived CSG output
```

The input adapter must:

- expand/triangulate Mittens indexed triangles into `csgrs` polygons;
- apply each operand's transform into CSG-root local space before the boolean;
- convert positions and normals to `f64` without changing handedness;
- reject non-finite positions, non-triangle source data, and unsupported
  topology before calling the backend; and
- verify input winding/orientation with focused cube fixtures.

The output adapter must:

- triangulate the returned `csgrs` mesh deterministically;
- rebuild a Mittens `CpuMesh` with `f32` positions and normals, rejecting
  non-finite or out-of-range values;
- recompute/validate normals and winding for the renderer rather than trusting
  imported per-polygon normals blindly;
- assign the CSG root's chosen material policy and a deliberate initial UV
  policy (UVs are not preserved through arbitrary booleans in this slice); and
- avoid every import/export path such as STL, DXF, Bevy mesh, Parry, or Rapier.

`Mesh<()>` is intentional for the MVP.  Per-polygon metadata cannot be made a
material/UV preservation promise across splits and joins, so CSG output gets
one explicit material at its root.

## Why CSG comes first

CSG answers the general questions that implicit terrain would otherwise
silently inherit:

- how operands are collected and transformed;
- how a derived mesh is owned, rebaked, selected, bounded, and serialized;
- what invalid or numerically unstable mesh inputs do; and
- whether an available Rust backend is robust enough to make a public MMS
  authoring promise.

Once that contract exists, an `ImplicitSurface` only needs to become another
producer of a normal triangle-mesh operand.

## MVP authoring and operation model

Introduce a nesting root—provisional name `CSG`—that owns one derived mesh.
Its direct/nested operand children describe source geometry and an operation
role.  The exact syntax follows crate validation, but the data model must
express:

- one base operand;
- one or more ordered operand operations;
- at least `difference` for the MVP;
- local transforms for every operand; and
- a retained authored operand tree for editor inspection and MMS round-trip.

Illustrative shape only:

```mms
CSG {
    CSGOperand.base() {
        // Any supported baked/static mesh source.
    }
    CSGOperand.difference() {
        // A closed cutter mesh, such as a stair or path clearance volume.
    }
}
```

Do not expose union or intersection merely because a backend offers them.
Implement and validate `difference` first, then expand the public operation
set only with specific scenarios and regression coverage.

## Operand contract

CSG operates on finite, static, closed triangle meshes after their source
operation has baked.  Initially valid sources should be limited to:

- normal `Renderable` primitive or imported static meshes for which CPU mesh
  data is available;
- `CombineMesh` outputs; and
- later, `ImplicitSurface` outputs.

Skinned/deformed meshes, dynamic per-frame meshes, open surfaces, and
non-manifold inputs are out of scope for the MVP unless the chosen backend can
prove a defined, safe behavior.  Reject unsupported operands with an explicit
diagnostic rather than producing broken geometry.

The initial implicit-surface integration is directional:

```text
ImplicitSphere children -> ImplicitSurface bake -> CSG operand
```

Neither a `CSG` root nor a `CSGOperand` is valid inside an `ImplicitSurface` in
slice one.  There is no need for the field system to sample arbitrary triangle
mesh booleans, and no implicit-surface-specific CSG API should be introduced.

## Implementation phases

1. Inventory the existing CPU-mesh, render-asset, `CombineMesh`, bounds,
   editor-selection, serialization, and removal seams.
2. Add the minimal `csgrs` dependency configuration above and build the
   internal `CpuMesh` ↔ `Mesh<()>` adapter.  Do not enable the crate's
   sketch/SDF/metaball/import-export integrations.
3. Run adapter-only fixtures: cube round trip, transformed cube round trip,
   winding/normal verification, and explicit rejection of malformed meshes.
4. Implement the component lifecycle and derived-mesh ownership independently
   of implicit surfaces.
5. Implement a box-minus-box / cube-minus-cube difference regression suite,
   including transformed operands and near-coplanar tolerance cases.
6. Add an MMS validation example with a visibly cut static mesh and clear
   diagnostics for unsupported operands.
7. Establish whether successful CSG roots collapse source transforms by
   default or adopt an explicit retain-sources mode analogous to
   `CombineMesh.keep_transforms()`.
8. Only then add `ImplicitSurface` as an operand producer and validate the
   staircase-hillside use case.

## Acceptance criteria

- A CSG root produces one correct, normal-oriented derived triangle mesh from
  a supported base mesh and closed difference cutter.
- Operand ordering and local transforms produce deterministic results.
- Bounds, selection, removal/rebake, and serialization have explicit tested
  behavior for the derived mesh and authored sources.
- Unsupported, open, non-manifold, and failed-boolean cases fail visibly with
  actionable errors; they do not silently replace good geometry.
- A documented crate/tolerance decision and focused regression suite support
  the public MVP, with `csgrs 0.20.1` confined to the internal adapter.
- The result can accept a baked `ImplicitSurface` mesh later without adding
  CSG logic to the implicit-field evaluator.

## Related work

- [Implicit 3D surfaces and CSG for terrain](implicit-3d-surfaces-and-csg.md)
- [Anime VN staircase background MMS examples](anime-vn-staircase-background-example.md)
- [Hierarchical BVH for mesh intersection, GLTF, and `CombineMesh`](hierarchical-bvh-mesh-intersection.md)

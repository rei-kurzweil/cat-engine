# Task: Implicit surface system first slice

Date: 2026-08-29

Status: implemented; accepted first slice of
[Implicit 3D surfaces and CSG for terrain](implicit-3d-surfaces-and-csg.md)

## Outcome and stop condition

Deliver one end-to-end authored `ImplicitSurface` expression that samples one
or more ordered `ImplicitSphere` signed-distance fields inside explicit bounds,
extracts their zero set as a static triangle mesh, and registers one
system-owned visual and aggregate bound at the `ImplicitSurface` root.

The authored root, sphere primitives, and transforms remain editable and
serializable. The generated samples, mesh, GPU instance, bounds, fingerprint,
and failure state are system-owned and never appear in MMS serialization.

Stop this slice when the overlapping-spheres example renders as one smooth,
lit, closed surface and the focused adapter, system, serialization, and cleanup
tests below pass. Do not add CSG integration, additional field primitives,
arbitrary scalar functions, or per-frame remeshing.

## Public MMS model

The first public form is:

```mms
ImplicitSurface
    .bounds(-3.0, -2.0, -2.5, 3.0, 2.0, 2.5)
    .voxel_size(0.12)
    .iso_level(0.0)
    .smooth_min_radius(0.45) {
    T.position(-0.8, 0.0, 0.0) {
        ImplicitSphere.radius(1.55) {}
    }
    T.position(0.9, 0.2, 0.0) {
        ImplicitSphere.radius(1.35) {}
    }
}
```

The authored structure has these rules:

- `ImplicitSurface` is the derived-mesh owner and root.
- It contains one or more `ImplicitSphere` descendants in authored child
  order. An empty surface is an authoring error.
- Transform containers between the root and a sphere are allowed. Their
  effective transform is resolved into surface-root-local space.
- The first slice accepts translation, rotation, and uniform scale. Rotation
  has no visible effect on a sphere but remains valid authored state.
- A non-uniform, sheared, non-invertible, or non-finite effective sphere
  transform is rejected. Ellipsoids are not approximated as spheres.
- A nested `ImplicitSurface` and an `ImplicitSphere` without an owning
  `ImplicitSurface` are invalid. A nested surface does not contribute fields
  to its ancestor.
- Field combination follows authored sphere order. Hard union is `min`; smooth
  union is the ordered polynomial smooth-min fold defined below.

Register only `ImplicitSurface` and `ImplicitSphere`. Names for future boxes,
capsules, noise fields, intersections, and subtraction are not public in this
slice.

## Parameter contract

`ImplicitSurface` stores:

- `bounds_min` and `bounds_max`, set together by
  `bounds(min_x, min_y, min_z, max_x, max_y, max_z)`;
- `voxel_size`, a positive world-space maximum cell width;
- `iso_level`, finite and `0.0` by default; and
- `smooth_min_radius`, finite, non-negative, and `0.0` by default.

Defaults may make a newly authored component valid, but the example and tests
must set bounds and voxel size explicitly. Reject an axis where `min >= max`.

`ImplicitSphere` stores a positive finite `radius`, configured by
`ImplicitSphere.radius(value)`. Its center comes from its effective transform;
do not add a second center property in this slice.

`voxel_size` is measured in world units. The first slice requires the
`ImplicitSurface` root itself to have a rigid transform plus uniform scale.
Translation or rotation changes only the derived instance model and world
bounds. Changing root scale rebakes so the maximum world-space cell width is
preserved. Non-uniform or sheared root transforms are rejected with an
actionable diagnostic.

For each axis, compute:

```text
world_extent = local_extent * root_uniform_scale
cell_count = ceil(world_extent / voxel_size)
sample_count = cell_count + 1
local_cell_width = local_extent / cell_count
```

Use checked integer arithmetic before allocating. The initial safety limits
are 128 cells per axis and 2,200,000 total sample points. Exceeding either is
an authoring error that reports the requested dimensions, limit, bounds, and
voxel size.

## Field semantics

Evaluate in `ImplicitSurface` local space. For a sphere with resolved local
center `c` and resolved local radius `r`:

```text
sphere(p) = length(p - c) - r
```

Negative values are inside and positive values are outside. With
`smooth_min_radius == 0`, combine fields with ordinary `min`.

For positive radius `k`, fold authored spheres with:

```text
h = clamp(0.5 + 0.5 * (b - a) / k, 0, 1)
smooth_min(a, b, k) = mix(b, a, h) - k * h * (1 - h)
```

This polynomial blend is intentional and is not an exact signed-distance
field in the blend region. Child order is retained in the fingerprint and
evaluation even though representative sphere fixtures should be symmetric.
The extracted surface is where the combined field equals `iso_level`.

Require at least one sample layer strictly outside the requested surface on
every side for a closed result. If the field is at or below `iso_level` on any
sampling-boundary point, reject the bake with a diagnostic asking the author
to enlarge bounds. Do not silently cap the mesh at the sampling box.

## Meshing backend gate

Start with this exact, private dependency:

```toml
mcubes = "=0.1.7"
```

`mcubes 0.1.7` is MIT-licensed and currently has one direct dependency,
`lin_alg`. It consumes a bounded regular `f32` scalar grid and produces
triangle vertices, normals, and indices. Keep all `mcubes` types and its
coordinate/packing conventions behind an engine-private adapter such as
`src/engine/graphics/implicit_mesh.rs`.

Before registering public MMS components:

1. Run `cargo tree -i mcubes` and `cargo tree -e features -i mcubes` and record
   their output or a concise exact summary in this document.
2. Confirm the lockfile contains only the expected dependency expansion.
3. Verify the crate's actual scalar packing and world-coordinate mapping with
   an asymmetric fixture; do not rely on prose documentation alone.
4. Verify outward winding and closed indexed topology. The backend emits
   triangle-local vertices, so the adapter must deterministically weld shared
   output positions or the dependency must be rejected.
5. Recompute geometric normals after final winding. Do not trust backend
   normals as the renderer contract.

If deterministic closed output cannot be obtained without a tolerance that
merges distinct nearby features, stop before public registration and replace
the backend or implement the narrow extractor internally.

## Internal adapter contract

Use a boundary equivalent to:

```rust
struct ImplicitGridSpec {
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    cells: [usize; 3],
    iso_level: f32,
}

fn extract_implicit_mesh(
    spec: &ImplicitGridSpec,
    samples: Vec<f32>,
) -> Result<CpuMesh, ImplicitMeshError>;
```

The system owns field discovery and sampling; the adapter owns backend packing,
coordinate mapping, deterministic welding, winding, normal construction, and
conversion to `CpuMesh`.

The adapter must:

- accept samples in one documented Mittens order and explicitly repack them
  into the backend's observed order;
- reject shape mismatches and all non-finite samples before extraction;
- use only the outward side of the zero set;
- validate all backend indices and triangle grouping;
- discard degenerate triangles only when their area is zero at renderer
  precision, then reject the result if that breaks closedness;
- orient every connected closed result so signed volume and normals point
  outward from the negative field region;
- deterministically weld positions shared by adjacent cells, canonicalize
  vertex and triangle ordering, and produce identical bytes for identical
  inputs;
- assign placeholder UVs `[0.0, 0.0]`; and
- reject non-finite output, `usize` to `u32` overflow, and values not
  representable as finite `f32`.

An empty result is valid only when every sample is strictly on the same side
of `iso_level`. It is a successful cached bake with no visual and no aggregate
bound. A boundary-crossing field that produces no triangles is a backend
error.

Use typed errors that distinguish parameter validation, sampling limits,
field topology, backend conversion, and output validation. Diagnostics should
identify the surface root and, for sphere errors, the authored sphere ID and
index.

## Authored and derived ownership

`ImplicitSurfaceComponent`, `ImplicitSphereComponent`, and their transform
containers are authored ECS state. Their normal `Component::to_mms_ast(...)`
implementations round-trip all public parameters and authored child order.

The sampled grid, generated `CpuMeshHandle`, GPU `InstanceHandle`, successful
fingerprint, root model, local bounds, and last failed fingerprint belong to
`ImplicitSurfaceSystem`. They must never serialize.

The first slice registers the derived visual against the `ImplicitSurface`
root with `MaterialHandle::TOON_MESH`, placeholder UVs, and normal lighting. A
direct `Color` child supplies its tint and can update without a remesh; white is
the fallback. Custom material, texture, emissive, transparency, inherited
style, and UV policy are follow-up work. Selection identity resolves to the
authored root rather than to a hidden runtime component.

Add `MeshOutputKind::ImplicitSurface` and register one aggregate local bound
at the root. Field primitives have no normal visual, bound, BVH, or raycast
representation and therefore require no source suppression.

## `ImplicitSurfaceSystem` evaluation and cache

Add `ImplicitSurfaceSystem` beside `CombineMeshSystem`. Reconcile it from
`SystemWorld::prepare_render(...)` after transforms are current and before
ordinary pending renderables are flushed.

For every root, one reconciliation transaction:

1. Validate root parameters and root transform.
2. Discover ordered owned `ImplicitSphere` descendants, rejecting malformed
   nesting and unsupported field descendants.
3. Resolve each sphere center and uniform radius into root-local space.
4. Compute safe grid dimensions from bounds, root scale, and world-space
   voxel size.
5. Build a fingerprint from root parameters, ordered sphere IDs and radii,
   sphere-to-root transforms, topology/order, and the root uniform scale.
6. If only root translation or rotation changed, update the existing output
   model and aggregate world bounds without resampling or remeshing.
7. Otherwise sample the ordered field, call the private adapter, upload the
   mesh if nonempty, and prepare the replacement visual and local bound.
8. Atomically replace the previous output only after every required step
   succeeds.

Cache at least the successful fingerprint, root model, derived instance and
CPU mesh handles, optional local bound, and last failed fingerprint. Live child
attachment, removal, reparenting, and order changes must be discovered even
when no transform signal fires.

### Failure policy

- On a failed first bake, emit one actionable diagnostic and create no output.
- On a failed rebake, retain the last known-good output while the same authored
  root still owns the field primitives represented by it.
- If topology changes remove or reparent a represented sphere, remove stale
  output rather than presenting geometry that claims unrelated authored state.
- Do not repeat a diagnostic every frame for an unchanged failing fingerprint.
- Root removal immediately removes its visual, aggregate bound, and cache.

## Rebuild matrix

| Change | Resample/remesh | Model/bounds update |
| --- | --- | --- |
| Sphere radius or sphere-relative transform | yes | yes |
| Sphere add/remove/reparent/order | yes | yes |
| Bounds, voxel size, iso level, smooth radius | yes | yes |
| Root uniform scale | yes | yes |
| Root translation or rotation only | no | yes |
| Root removal | no | remove output |
| Unrelated descendant | no | no |

## Component and registry work

Add serializable components under `src/engine/ecs/component/`, export them from
`component/mod.rs`, and register only the two public names through:

- `src/scripting/component_registry.rs`;
- `src/scripting/runtime_config.rs`; and
- the existing component-expression serialization path.

Add the private graphics adapter, system module/export, `SystemWorld` field,
render-preparation reconciliation, transform/update notification as needed,
and cleanup hooks. Reuse the derived-output and aggregate-bounds seams already
established by `CombineMeshSystem`; do not create a second render-asset store
or serialize a generated renderable.

## Verification plan

### Adapter and field tests

- Extract one centered sphere and prove finite output, indexed triangle
  grouping, closed edges, outward winding/normals, and approximate bounds.
- Use an asymmetric scalar fixture to prove axis order, offset, and cell width.
- Re-run identical extraction and compare canonical vertices/indices exactly.
- Prove a fully outside field yields the intentional empty result.
- Reject non-finite samples, shape mismatch, boundary contact, invalid indices,
  degenerate output, and unsafe grid dimensions.
- Compare two overlapping spheres with hard union and positive smooth radius;
  representative samples and the extracted waist must change predictably.
- Characterize 32-cubed, 48-cubed, and 64-cubed cell grids in release mode,
  recording sample count, triangle count, bake time, and peak grid bytes.

### System tests

- Discover spheres in authored order and exclude spheres owned by another root.
- Resolve nested transforms and uniform scales into root-local centers/radii.
- Reject no spheres, standalone spheres, nested surfaces, and non-uniform or
  non-finite transforms with actionable diagnostics.
- Register exactly one derived visual and one
  `MeshOutputKind::ImplicitSurface` aggregate bound for a nonempty result.
- Change only root translation/rotation and prove adapter invocation count does
  not increase while the model and world bounds update.
- Mutate every rebuild-matrix input and prove only the specified work occurs.

### Lifecycle tests

- Serialize a successfully baked expression and prove only authored surface,
  sphere, and transform components appear.
- Remove or reparent a sphere and prove stale output is replaced or removed.
- Remove a root and prove complete visual, bounds, and cache cleanup.
- Force a failed first bake and prove no output appears.
- Force a failed rebake and prove last-known-good behavior plus diagnostic
  deduplication.

## Example

Add `examples/implicit-surface.mms`, runnable with:

```text
cargo run --release -- load examples/implicit-surface.mms
```

The example is the first composition study for `anime-vn-background.mms`: a
three-sphere hillside descending toward camera-right and a camera-left tree
with a plain brown cube trunk and a five-sphere smooth deciduous canopy. Both
implicit surfaces have direct color children and explicit empty-margin bounds.
It remains free of CSG, imported models, and `CombineMesh`.

## Implementation order

1. Run and record the `mcubes` dependency, feature, license, and lockfile gate.
2. Implement grid validation, sphere SDF, smooth-min sampling, and benchmark
   fixtures without public MMS registration.
3. Implement and pass the private adapter's packing, winding, welding,
   determinism, empty-output, and malformed-output tests.
4. Add the two authored components and MMS round-trip coverage.
5. Add `ImplicitSurfaceSystem`, transactional output replacement, aggregate
   bounds, cleanup, and rebuild-matrix tests.
6. Add and visually validate the overlapping-spheres example.
7. Run the focused test set, record results below, and stop the slice.

## Explicit non-goals and follow-up triggers

- CSG integration waits for the mesh-CSG backend gate to be resolved.
- `ImplicitBox`, capsules, noise/displacement, field intersection/subtraction,
  and arbitrary authored scalar functions require their own scenarios.
- Non-uniformly transformed spheres become an explicit ellipsoid primitive,
  not relaxed validation.
- Runtime sculpting, asynchronous/chunked baking, level of detail, and
  per-frame remeshing require demonstrated latency or memory pressure.
- Materials, colors, textures, UV generation, and multiple material regions
  require a separate derived-material contract.
- Dual contouring requires a sharp-feature use case that marching cubes cannot
  satisfy.
- Making the baked output a CSG operand is a later integration slice and must
  reuse the normal `CpuMesh` output rather than adding CSG concepts here.

## Implementation record

- Pinned `mcubes = "=0.1.7"`. `cargo tree -i mcubes` resolves only
  `mittens-engine -> mcubes 0.1.7`; its default feature is enabled. The lockfile
  expansion is `mcubes -> lin_alg -> num-traits` plus `paste`, and the backend
  is MIT licensed.
- The private adapter confirmed `x + y * nx + z * nx * ny` sample packing,
  offsets backend coordinates into authored bounds, validates indices and
  closed edges, orients by signed volume, recomputes smooth normals, and emits
  deterministic indexed output. Neighbor-aware spatial welding uses a
  tolerance of `1e-4` of the smallest cell width; this fixed backend drift in
  the asymmetric demo fixtures without approaching adjacent grid features.
- Centered-sphere release characterization on 2026-08-29:

  | Cells | Samples | Triangles | Grid bytes | Bake time |
  | --- | ---: | ---: | ---: | ---: |
  | 32³ | 35,937 | 4,280 | 143,748 | 11.024 ms |
  | 48³ | 117,649 | 9,512 | 470,596 | 24.664 ms |
  | 64³ | 274,625 | 17,192 | 1,098,500 | 47.622 ms |

- Final example: `examples/implicit-surface.mms`. The release loader completed
  all eight startup stages with both derived meshes accepted and no
  `ImplicitSurfaceSystem` diagnostics.
- Acceptance commands:
  `cargo check --locked`, `cargo test --locked implicit_surface --lib`,
  `cargo test --locked implicit_mesh --lib`, the ignored release
  characterization test, and
  `cargo run --release -- load examples/implicit-surface.mms`.

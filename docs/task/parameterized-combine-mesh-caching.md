# Task: Parameterized MMS factory and `CombineMesh` caching

Date: 2026-09-06

Status: proposed

## Motivation

Parameterized MMS factories can generate large static component trees that are
immediately consolidated by `CombineMesh`. The motivating example is:

```mms
import { truss } from "../assets/components/truss.mms"

truss(8)
truss(8)
truss(32)
```

Two calls to `truss(8)` should still create two distinct scene instances, but
they should be able to share the same baked CPU and GPU geometry. `truss(32)`
must use a different geometry variant.

Today those three calls create three separate baked CPU meshes and three GPU
mesh uploads.

## Current behavior

There are several different caches in this path, but none deduplicates a
`CombineMesh` result across factory instances.

### MMS module caching is not factory-result caching

The MMS evaluator caches a loaded module by `SourceId`. `AssetSystem` similarly
holds one `LoadedMmsModule` and its exported function values. This avoids
re-reading and re-evaluating the module for every placement.

Calling an exported factory is intentionally different. Every ordinary
`factory(args...)` invocation executes the function and produces a fresh live
component identity and component subtree. Factory call results are not cached.
This behavior is required for scene identity, mutable state, event handlers,
queries, and editor operations.

For `truss(8)`, each call therefore creates a new `CombineMesh` root and a new
set of source transforms and cube renderables.

### Built-in source meshes are shared

Every source bar in a truss uses the stable built-in cube
`CpuMeshHandle::CUBE`. The source bars do not each allocate cube vertex data.
They share the built-in CPU mesh and its GPU upload while they are ordinary
renderables.

That sharing does not automatically carry through to the combined result,
because the source transforms are baked into a new vertex/index buffer.

### Every `CombineMesh` root currently bakes independently

During `SystemWorld::prepare_render`, `CombineMeshSystem::reconcile_and_build`:

1. finds the renderable descendants owned by one `CombineMesh` root;
2. transforms their vertices into combine-root-local space;
3. concatenates the vertices and indices into a new `CpuMesh`;
4. calls `RenderAssets::register_mesh(mesh)`;
5. asks `RenderAssets::gpu_mesh_handle` to upload that new handle; and
6. registers one `VisualWorld` instance for that combine root.

`RenderAssets::register_mesh` always appends and always returns a fresh
`CpuMeshHandle`. It performs no content deduplication. GPU upload caching is
keyed by `CpuMeshHandle`, so two byte-identical combined meshes with different
CPU handles are uploaded separately.

The renderer batches/instances by GPU mesh and material (plus texture and
other render state). Distinct combined `MeshHandle`s consequently prevent two
otherwise identical trusses from sharing an instanced draw batch.

### The current fingerprint is only per-root dirty detection

`CombineMeshSystem::fingerprint` is not a reusable mesh cache key. It includes:

- the combine root `ComponentId`;
- each source `ComponentId`;
- source mesh and material handles; and
- source transforms in `keep_transforms` mode.

Including component identities guarantees that two separate truss instances
have different fingerprints. `CombinedOutput` stores that fingerprint to avoid
rebaking one unchanged root; it does not share output with another root.

In default collapsed mode, the source subtrees are removed after the first
successful bake. The `CombinedOutput` retains the visual instance, but
`RenderAssets` retains the separately registered CPU and GPU mesh resources for
the lifetime of the asset registry.

### Adjacent limitation: combined output styling is not propagated

The current `CombineMeshSystem` registers the consolidated `VisualWorld`
instance with a hard-coded white color (`[1.0; 4]`) and other fixed instance
state. It does not transfer an inherited `ColorComponent` or the first source's
effective color to the output. Consequently, a color authored around a
`CombineMesh` tree, such as the dark-gray color around the shading-model truss,
does not currently affect the consolidated visual.

This is separate from geometry caching, but the cache design must preserve the
distinction: geometry should be shared, while color and other uniform-only
render state should remain per instance. Fixing style propagation should not
create another baked `CpuMeshHandle` for each color.

## Desired behavior

For equal static geometry recipes:

```text
truss(8) instance A ─┐
                     ├─ shared CpuMeshHandle ─ shared MeshHandle
truss(8) instance B ─┘                         ├─ InstanceHandle A
                                               └─ InstanceHandle B

truss(32) instance C ─ distinct CpuMeshHandle ─ distinct MeshHandle
```

Scene identity remains per invocation:

- separate component identities;
- separate combine roots;
- separate transforms, colors (once output style propagation is implemented),
  selection state, and instance handles.

Only immutable geometry resources are shared.

## Recommended first implementation: cache the bake recipe

Add a cache at the `CombineMeshSystem`/`RenderAssets` boundary that maps a
canonical combine recipe to a shared CPU mesh and local bounds:

```rust
struct CombinedMeshCacheValue {
    mesh: CpuMeshHandle,
    local_bounds: Aabb,
}

HashMap<CombinedMeshRecipeKey, CombinedMeshCacheValue>
```

The recipe key should describe geometry, not scene identity. It must exclude:

- combine-root `ComponentId`;
- source `ComponentId`s;
- the combine root's world transform;
- per-instance color and other uniform-only state.

It should include, in deterministic source order:

- the bake algorithm/vertex-format version;
- each source `CpuMeshHandle` plus an immutable mesh revision if handles can
  ever be modified in place;
- each source-to-combine-root local transform;
- geometry-affecting UV or vertex attribute variants; and
- mode/options that change emitted geometry.

Use a typed equality key or a collision-checked content digest rather than the
current bare `u64` fingerprint. Float fields need a canonical policy. Exact
`to_bits()` identity is a reasonable initial rule if `-0.0` is normalized and
non-finite transform values are rejected.

On a cache hit, `CombineMeshSystem` should skip vertex concatenation and
`register_mesh`, reuse the cached `CpuMeshHandle`, and call
`gpu_mesh_handle`. The existing GPU cache will then return the shared
`MeshHandle`. Each root still registers its own `VisualWorld` instance, allowing
the renderer to instance equal trusses together.

Geometry and render state should remain separate. The current output material
comes from the first source and `GpuRenderable` stores material separately from
mesh. A material change should normally update/re-register instance state while
retaining the same cached geometry, not force another CPU bake.

This layer is the safest starting point because it caches what was actually
resolved and baked. It also deduplicates identical output made by different
factory functions or by directly authored component trees.

## Optional second implementation: factory invocation keys

A factory-level key can avoid more work than a bake-recipe cache:

```text
(module source revision, export name, canonical arguments, factory ABI version)
```

For `truss(32)`, such a key could avoid repeatedly interpreting its loop and
constructing 132 source renderables merely to discover a known combined mesh.
However, arguments alone are not a generally valid geometry identity.

An MMS factory may also depend on:

- captured mutable module values;
- tables or component references passed as arguments;
- queries and current world state;
- external data or host calls;
- conditionals that produce non-renderable behavior;
- engine or asset revisions; or
- nested factories whose implementation changed.

Ordinary factory calls must also keep returning fresh live component identities.
Returning the same cached component tree would be a semantic bug.

Therefore factory-level caching should be an explicit, constrained optimization,
not automatic memoization of every MMS function. Possible requirements are:

1. The export is declared or inferred to be pure and deterministic.
2. Its arguments have canonical, immutable value representations.
3. The result has a supported static `CombineMesh` boundary.
4. The cache key includes the transitive source/module revision.
5. A hit reuses only compiled template or baked geometry data, never live ECS
   component identities.

One implementation could attach an internal `FactoryInvocationProvenance` value
to returned combine roots. `CombineMeshSystem` could use it as a fast cache hint,
then fall back to the resolved recipe key when the factory is not cacheable.
Factories that return multiple combine roots, dynamic components, or behavioral
subtrees need an explicit policy before source-tree construction can be skipped.

## Suggested phases

### Phase 1: shared baked geometry

- Add a global-per-`RenderAssets` or global-per-`CombineMeshSystem` recipe cache.
- Remove component IDs and material from the reusable geometry identity.
- Store the resolved `CpuMeshHandle` on `CombinedOutput` for diagnostics.
- Reuse cached local bounds with the geometry.
- Add cache hit/miss, avoided CPU bytes, and avoided GPU upload counters.
- Preserve one distinct `InstanceHandle` per combine root.

This phase still expands every MMS factory tree and computes its recipe, but it
eliminates duplicate combined CPU allocations, GPU uploads, and mesh-identity
fragmentation in draw batching.

### Phase 2: invalidation and lifetime

- Define immutable mesh handles or add explicit mesh revision numbers.
- Include algorithm and vertex-format revisions in keys.
- Invalidate naturally across MMS/module hot reload by source revision.
- Decide cache retention, reference counting, and an eviction budget.

`RenderAssets` currently stores CPU meshes in an append-only `Vec` and has no
resource removal. A first cache may follow that lifetime, but the retained-byte
cost must be observable before adding unbounded content-derived variants.

### Phase 3: opt-in pure factory acceleration

- Define purity/cacheability metadata for MMS asset factories.
- Canonicalize supported scalar, string, array, and table arguments.
- Reject or bypass caching for live component handles and host-dependent values.
- Carry invocation provenance to eligible `CombineMesh` roots.
- Cache a reusable template and/or directly reusable baked-output descriptor.
- Preserve fresh ECS identities and normal authored serialization semantics.

## Correctness and acceptance tests

- Two `truss(8)` calls create different combine roots and instance handles but
  resolve to the same `CpuMeshHandle` and GPU `MeshHandle`.
- `truss(8)` and `truss(32)` resolve to different mesh handles.
- Different outer/world transforms still share root-local geometry.
- Different internal source transforms produce different recipe keys.
- A material-only change reuses geometry while changing output render state.
- A geometry-affecting UV/vertex variant produces a cache miss.
- `CombineMesh.keep_transforms()` reuses an existing variant after returning to
  a previously seen recipe and does not reuse stale geometry after an edit.
- Two calls to a cached factory still produce distinct live component IDs.
- A mutable or world-dependent factory is not reused solely because its explicit
  arguments match.
- Module/source revision changes cannot return geometry from an incompatible old
  factory implementation.
- Repeated creation/removal does not leak `VisualWorld` instances; retained CPU
  and GPU cache memory follows the documented lifetime/eviction policy.

## Related code and documents

- `src/engine/ecs/system/combine_mesh_system.rs`
- `src/engine/graphics/render_assets.rs`
- `src/engine/graphics/visual_world.rs`
- `src/engine/ecs/system/asset_system.rs`
- `crates/meow-meow-script/src/evaluator.rs`
- `docs/draft/combine_mesh.md`
- `docs/review/mesh_component.md`
- `docs/task/mms-procedural-renderables-and-parametric-meshes.md`
- `docs/task/partial-annulus.md`

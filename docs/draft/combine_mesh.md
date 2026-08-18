# `CombineMesh`: subtree mesh consolidation

## Status

Design draft.  This describes a new ECS component and a proposed
`src/engine/ecs/system/combine_mesh_system.rs`; it is not implemented.

## Goal

`CombineMesh` makes its component subtree render as one consolidated visual
instead of registering every descendant `RenderableComponent` as a separate
`VisualWorld` instance.

```mms
CombineMesh {} {
  Transform.translate(-1, 0, 0) { Renderable.cube() {} }
  Transform.translate( 1, 0, 0) { Renderable.cube() {} }
}

```

The v1/default form is `CombineMesh {}` / `CombineMesh.single_material()`.
It creates one mesh, one material binding, and one `VisualWorld` instance.  It
uses the material from the first eligible descendant renderable; later source
materials are deliberately ignored.  v1 does not need to validate that source
materials match.

`CombineMesh.multi_material()` is a v2 proposal, not a v1 constructor.  It
needs a renderer format beyond the current `GpuRenderable { mesh, material }`
record.

## Current lifecycle and the implication for suppression

Today, `RenderableComponent::init` emits `RegisterRenderable`.  `SystemWorld`
passes that intent to `RenderableSystem::register_renderable`; the latter puts
the component in its `pending` map.  GPU upload and `VisualWorld::register`
then happen later in `SystemWorld::prepare_render` via
`RenderableSystem::flush_pending`.

Tree initialization is pre-order, so a `CombineMeshComponent` init intent
would be emitted before its descendants' `RegisterRenderable` intents.  That
ordering is useful, but must not be the correctness mechanism: trees can be
edited, components can be attached later, and a renderable can already have a
`VisualWorld` handle when a CombineMesh ancestor is added.

Therefore, **suppression is derived from the current component graph, not
stored as a one-time flag on every descendant**.  The combine system owns the
mapping below and answers it whenever renderables are registered or flushed.

```text
descendant Renderable ── nearest CombineMesh ancestor ──► owning combine root
                         (unless another CombineMesh is nearer)
```

The nearest ancestor rule makes nested groups unambiguous: an inner
`CombineMesh` owns its descendants and appears to the outer group as no
renderable geometry.  Treating the inner result as input to the outer group is
an optional future flattening pass, not MVP behavior.

## Proposed ECS/API shape

```rust
pub enum CombineMeshMode {
    FirstSourceMaterial,
    // v2:
    MultiMaterial,
}

pub struct CombineMeshComponent {
    pub mode: CombineMeshMode,
    pub dirty: bool,
    // Runtime-only: output resource/VisualWorld handles and diagnostics.
}
```

The component is a topology/grouping node; it does not need its own
`TransformComponent`.  Its output uses the nearest ancestor transform, just
as an ordinary descendant renderable does.  Descendant transform matrices are
baked into the combined mesh in the combine root's local space:

```text
vertex_in_combined_local = inverse(world(root basis)) * world(descendant) * vertex
```

This preserves the initial scene appearance and lets moving an ancestor of the
combine root move the output normally.  Phase 1 treats internal transforms as
bake-time authoring data: after the first bake, moving a transform *inside* the
group does not move an individual part or trigger a rebuild.  A future explicit
rebuild/edit workflow (or per-part transforms) can make such edits live.

Suggested constructors/serialization:

```mms
CombineMesh {}
CombineMesh.single_material() {}
```

`CombineMesh {}` serializes as the default first-source-material form.  The
`multi_material()` spelling is reserved for v2.

## `CombineMeshSystem` responsibilities

Add `src/engine/ecs/system/combine_mesh_system.rs`, register/export it from
`ecs/system/mod.rs`, and add `pub combine_mesh: CombineMeshSystem` to
`SystemWorld`.  It should run in the render-preparation lifecycle, after
imports have made source meshes resolvable and before ordinary pending
renderables are flushed.

The system maintains a derived membership index:

```rust
owner_by_renderable: HashMap<ComponentId, ComponentId> // renderable -> combine root
groups: HashMap<ComponentId, CombinedGroupState>
```

On initialization, topology changes, removal, source mesh resolution, or a
source transform/mesh/UV change, it marks the affected nearest group dirty.
The first eligible source's material or effective renderer-style change also
dirties the group; later sources' material/style changes do not in v1.
Rebuild does the following:

1. Walk the group subtree, stopping at nested `CombineMesh` roots.
2. Collect descendant `RenderableComponent`s and resolve their `MeshComponent`,
   UV, effective color/opacity/cutout/background/overlay, and world transform.
3. Select the first eligible source as the output material/style authority;
   v1 does not compare materials on later sources.
4. Remove any source handles that may already be in `VisualWorld`; remove their
   BVH/raycast entries as well.
5. Bake source positions and normals into a new CPU combined resource.
6. Upload it, then register/update exactly one output visual.
7. Retain source-to-output metadata for diagnostics and future picking.

The output should be system-owned, rather than synthesizing a visible
`RenderableComponent` in the authored tree.  A synthetic component would
participate in serialization, inheritance walks, editor trees, and user
queries unless every one of those systems learned to hide it.

## How to suppress individual `VisualWorld` registration

There are two complementary hooks; both are required.

1. `SystemWorld::register_renderable` asks `combine_mesh.owns_renderable(world,
   component)`.  If true, it must not call `RenderableSystem::register_renderable`,
   attach normal renderable bounds, add BVH/raycast eligibility, or notify
   clipping.  It instead marks the owning group dirty.
2. Immediately before `RenderableSystem::flush_pending`, the combine system
   re-evaluates ownership for pending and already-registered source
   renderables.  It removes stale pending records and existing source handles.
   This closes the attach-late/reparent/CombineMesh-added-after-renderable gap.

The `RenderableSystem` needs small explicit APIs rather than exposing its
maps, for example:

```rust
fn suppress_renderable(&mut self, world: &mut World, visuals: &mut VisualWorld,
                       component: ComponentId);
fn is_pending_or_registered(&self, component: ComponentId) -> bool;
```

`suppress_renderable` removes the component from `renderables`, `pending`, and
all pending style maps, and removes its `VisualWorld` handle if present.  The
combine system should call the normal SystemWorld removal path for the BVH and
raycast side effects, or that path should be factored into a shared
`deactivate_source_renderable` helper.

Do not encode this as `RenderableComponent { suppressed: bool }`: it becomes
stale across reparenting and makes ownership depend on mutation ordering.

## Proposed combined formats

### First-source-material (v1 / MVP, optimized)

`CombinedCpuMesh::Single` is just the existing tightly packed `CpuMesh`:

```rust
Single {
    mesh: CpuMesh,                // positions/normals transformed and concatenated
    material: MaterialHandle, // selected from the first eligible source
    render_class: RenderClass,    // opaque/cutout/transparent/background/overlay
}
```

The first eligible source supplies the output material.  Every later source's
geometry is baked using that material, even if it originally named another
material.  This is intentional v1 behavior, not a validation failure.

The same one-output limitation applies to per-instance renderer state:
`TransparentCutout`, background/overlay routing, opacity layering, emissive,
light quantization, texture/filtering, and future descriptors need one defined
output value.  For v1, take the first eligible source's effective renderer
state too.  Source UVs remain mesh data and are preserved.  Per-source color
cannot remain a `VisualWorld` per-instance color; v1 takes the first source's
color until the vertex format gains vertex colors.

### Multi-material (v2, separate optimized resource)

Do not expose this in v1.  The v2 multi-material format must not pretend it is
one existing `GpuRenderable`.  Propose a distinct resource with concatenated
geometry and material sections:

```rust
Multi {
    mesh: CpuMesh, // one vertex/index allocation; indices grouped by section
    sections: Vec<CombinedMaterialSection> {
        material: MaterialHandle,
        first_index: u32,
        index_count: u32,
        render_class: RenderClass,
        descriptor_key: CombinedDescriptorKey,
    },
}
```

The renderer gains `GpuCombinedMesh`/`VisualCombinedInstance` and emits one
draw per compatible section.  This still eliminates descendant `VisualWorld`
instances, transform updates, per-instance bookkeeping, and repeated mesh
uploads; it does **not** promise one draw call for arbitrary materials.

A later, genuinely one-draw `MultiMaterialPacked` variant may add a per-vertex
material slot plus a material palette and use texture arrays/bindless
descriptors.  It is only valid when all slots share one pipeline, render class,
descriptor layout, and compatible texture representation.  Keep that as a
separate capability-gated format, not the semantics of
`CombineMesh.multi_material()`.

Sections must never cross render classes: opaque, cutout, transparent,
background, and overlay have different ordering/pipeline rules.  A group with
such a mix either produces multiple combined outputs (one per render class) or
is rejected in MVP.  The latter is simpler and preserves the stated
"one combined output" contract.

## Constraints and non-goals for phase 1

- Phase 1 does not yet support skinned meshes or morph/animation-driven vertex
  deformation.  Phase 2 adds an explicit *current-pose snapshot* path for
  skinned meshes; it bakes their deformed vertices at rebuild time and does not
  keep the combined output live-skinned.
- No independent descendant transform animation after build; phase 1 leaves
  the baked output unchanged rather than moving an individual part.
- No source-level `StencilClip`, render-to-texture, custom pipeline, or other
  per-instance renderer state unless the group format supports it explicitly.
- Phase 1 disables source BVH/raycast registrations and makes the output
  non-raycastable unless the group supplies one aggregate/proxy shape.  Phase 2
  establishes the full aggregate-BVH and intersection policy below.
- Bounds should be one aggregate AABB attached to/output for the group; do not
  retain source bounds as live render bounds.
- `Renderable` descendants remain authored, inspectable components; they are
  merely render-suppressed while owned by a group.

## Scheduling and invalidation

Recommended `prepare_render` sequence:

```text
GLTF imports resolve
  -> CombineMeshSystem::reconcile_and_build
  -> RenderableSystem::flush_pending (only unsuppressed renderables)
  -> clipping resync / texture flush
```

The source list is rederived for every dirty group.  Cheap topology events can
mark a root dirty by walking ancestors to the nearest `CombineMesh`.  Before
the event plumbing is complete, a correctness-first implementation may scan
all CombineMesh subtrees each render preparation and only rebuild when a
content fingerprint changes (source IDs, mesh handles/revisions, transforms,
and the first eligible source's effective material/style state).

For failed validation or not-yet-resolved imported meshes, keep the group dirty
and suppress sources only once the group has a valid replacement output.  This
avoids a one-frame disappearance.  If the group was previously valid, retain
its old output until the replacement is ready; if it becomes permanently
invalid, report the error and choose an explicit policy (recommended: fall back
to independently registered sources after removing the old output).

## Implementation phases

### Phase 1: static mesh consolidation

Add the component, v1 MMS constructors/registry/serialization, ownership
lookup, registration suppression, first-source-material CPU baking, and one
system-owned output.  Include reparent/attach-late tests and test that mixed
source materials yield one output using the first source's material.  Include
invalidation, aggregate bounds, cleanup, and explicit unsupported-state
diagnostics/fallback.

### Phase 2: skinned snapshots and ray intersection policy

Allow a skinned descendant by taking a **current-pose snapshot** of its mesh.
`SkinnedMeshSystem` (or a focused skinning helper it exposes) supplies the
current joint matrices and source vertex skin weights; the combine builder
applies skinning on CPU, then applies the descendant-to-combine-root transform
and appends the resulting static vertices.  The output is a snapshot: later
animation/pose changes mark the group dirty and require another snapshot
rebuild.  It must never accidentally retain stale joint indices/weights in the
combined mesh.

Phase 2 also replaces the phase-1 non-raycastable default with an aggregate
acceleration structure and a specified hit result:

```text
ray -> one CombineMesh aggregate BVH -> baked triangle intersection
    -> { combine_root, source_renderable, triangle, position, normal, distance }
```

The combined output owns one aggregate bounds/BVH entry; its suppressed sources
have none.  While baking, retain a triangle-range table that maps every output
triangle range to its authored source renderable (and, where useful, its source
triangle).  On a hit, the raycast system reports the original descendant as
the target for existing selection/handler semantics, while retaining
`combine_root` for diagnostics.  If a source has an explicit proxy raycast
shape, either transform/bake that shape into the aggregate in this phase or
reject it with a clear diagnostic; do not mix proxy and triangle results
implicitly.  Rebuild/refit the aggregate whenever geometry, transforms, or a
skinned snapshot changes.

### Phase 3: multi-material

Add `CombineMesh.multi_material()`, a sectioned multi-material resource, and
renderer support.  Measure VisualWorld/CPU savings separately from draw-call
savings.

### Later: packed one-draw multi-material

Consider `MultiMaterialPacked` only after descriptor and pipeline compatibility
rules are designed.

## Acceptance tests

- A group containing N compatible static renderables produces one source-free
  `VisualWorld` output and keeps its rendered world-space geometry unchanged.
- A renderable added below an already-live group never receives a source handle.
- Adding/reparenting a group over already-live renderables removes their handles
  before the next render.
- Removing/reparenting the group unregisters its output and restores eligible
  descendants as ordinary renderables.
- Nested groups do not double-own geometry.
- V1 combines mixed-material inputs using the first eligible source's material.
- Phase 2 skinned snapshots match the source's current posed geometry and
  rebuild after pose changes.
- Phase 2 aggregate ray hits select the authored descendant represented by the
  intersected baked triangle.
- Phase 3 multi-material sections preserve material assignment and pass routing.

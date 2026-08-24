# Morph target lifecycle review

This reference traces the current morph-target path from glTF import to the
shared Vulkano deformation cache.

## Mental model

Morph targets have two ownership domains:

| Domain | Owner | Role |
| --- | --- | --- |
| Immutable mesh data | `CpuMesh::morph_targets` | Dense, target-major POSITION/NORMAL deltas shared by mesh users. |
| Instance expression state | `GLTFComponent::morph_targets` and `morph_factors` | Imported weights plus temporary drivers for one GLTF ECS instance. |

```text
glTF target accessors -> CpuMesh dense deltas -> Vulkano delta arena
glTF weights/labels   -> GLTF factor map    -> sparse visual input
                                             -> shared compute cache
                                             -> window / mirrors / XR eyes
```

Labels are semantic lookup metadata. Structural keys route a factor to the
correct rendered primitive.

## Types and roles

| Type | Layer | Role | Trace |
| --- | --- | --- | --- |
| `MorphTargetKey` | ECS identity | Stable `{ node_index, primitive_index, target_index }`; independent of labels. | [`morph_target.rs:13`](../../src/engine/ecs/component/morph_target.rs#L13) |
| `MorphTargetInfo` | ECS metadata | Key, optional label, imported base factor. | [`morph_target.rs:20`](../../src/engine/ecs/component/morph_target.rs#L20) |
| `CpuMorphTarget` | shared CPU mesh | One dense position/normal delta array per target. | [`mesh.rs:275`](../../src/engine/graphics/mesh.rs#L275) |
| `MorphFactorState` | GLTF instance | `base` plus optional `driver`; effective is `driver.unwrap_or(base)`. | [`morph_target.rs:51`](../../src/engine/ecs/component/morph_target.rs#L51) |
| `MorphTargetMapComponent` | GLTF child | Explicit semantic channel → target label mapping. | [`morph_target.rs:75`](../../src/engine/ecs/component/morph_target.rs#L75) |
| `MorphTargetBindingComponent` | Renderable child | Connects a renderable to owning GLTF instance, node, and primitive. | [`morph_target.rs:29`](../../src/engine/ecs/component/morph_target.rs#L29) |
| `VisualWorld::morph_inputs` | visual world | Sparse `(target_index, weight)` input per visual instance. | [`visual_world.rs:169`](../../src/engine/graphics/visual_world.rs#L169) |
| `GpuMorphDelta` | GPU | Dense target/vertex delta record. | [`deformation.rs:31`](../../src/engine/graphics/deformation.rs#L31) |
| `GpuActiveMorph` | GPU | Dense-delta base plus active weight. | [`deformation.rs:38`](../../src/engine/graphics/deformation.rs#L38) |
| `GpuDeformationJob` | GPU | Deformation ranges and active-morph subrange for an instance. | [`deformation.rs:45`](../../src/engine/graphics/deformation.rs#L45) |

## Import and instance setup

`GLTFSystem::load_gltf_resources` reads `reader.read_morph_targets()` for every
triangle primitive. POSITION is required and must have one delta per base vertex.
NORMAL is optional; missing normal data becomes dense zero deltas, and present
normal data must have the same vertex count. The importer creates a target-major
`Vec<CpuMorphTarget>` and attaches it with `CpuMesh::with_morph_targets`.

Trace: [`gltf_system.rs:708`](../../src/engine/ecs/system/gltf_system.rs#L708)
through [`gltf_system.rs:840`](../../src/engine/ecs/system/gltf_system.rs#L840).

The importer separately reads `mesh.extras.targetNames` as optional VRM/VRoid
metadata. Those names provide labels only, not structural identity. Trace:
[`gltf_system.rs:655`](../../src/engine/ecs/system/gltf_system.rs#L655).

After a GLTF instance is spawned, the system records each target's key, label,
and base factor on that `GLTFComponent`. Node weights take precedence over mesh
weights; missing weights use zero. Each factor begins as
`MorphFactorState { base, driver: None }`. Trace:
[`gltf_system.rs:485`](../../src/engine/ecs/system/gltf_system.rs#L485).

## Primitive binding

For a target-bearing imported primitive, the spawned ECS shape is:

```text
GLTFComponent instance
└── imported node TransformComponent
    └── RenderableComponent
        ├── MeshComponent
        └── MorphTargetBindingComponent { gltf, node_index, primitive_index }
```

The binding lets the renderer select factors from the right GLTF instance even
when the mesh is shared. It is created at
[`gltf_system.rs:1050`](../../src/engine/ecs/system/gltf_system.rs#L1050)
through [`gltf_system.rs:1083`](../../src/engine/ecs/system/gltf_system.rs#L1083).

## Factor activation and driver release

The precedence rule is:

```text
effective factor = driver when present; otherwise imported base
```

`active_factors` excludes only values with `abs(value) <= 1e-4`. Negative
weights are valid; map ordering is deterministic because factors use a
`BTreeMap`. Assigning `driver = None` restores the imported base. The unit tests
cover signed epsilon filtering and driver release at
[`morph_target.rs:108`](../../src/engine/ecs/component/morph_target.rs#L108).

`MorphTargetMapComponent` is GLTF-owned semantic configuration. It currently
recognizes only `left_eye_blink` and `right_eye_blink`; both require explicit
target labels. There is no automatic conventional-label or VRM-expression map.

### AVC + OSC blink path

`XREyeTrackingSystem` accepts `/avatar/parameters/EyesClosedAmount` and
`/tracking/eye/EyesClosedAmount`, interpreting them as direct closure
(`0 = open`, `1 = closed`). Trace:
[`xr_eye_tracking_system.rs:240`](../../src/engine/ecs/system/xr_eye_tracking_system.rs#L240).

Each tracker tick clears closure first. A packet supplies a live driver value;
the absence of a packet releases it. `AvatarControlSystem` resolves its GLTF and
GLTF-child `MorphTargetMap`, then writes the closure or `None` to every imported
target with a matching mapped label. Trace:
[`avatar_control_system.rs:187`](../../src/engine/ecs/system/avatar_control_system.rs#L187).

The Bisket example declaration is at
[`vtuber-eye-tracking-mirror.mms:109`](../../examples/vtuber-eye-tracking-mirror.mms#L109).

## ECS-to-renderer synchronization and invalidation

`RenderableSystem::tick` is the factor bridge. For every registered renderable
with `MorphTargetBindingComponent`, it reads the binding's `GLTFComponent`, calls
`active_factors`, retains keys for that node/primitive, converts them to
`(target_index, weight)`, and calls `VisualWorld::set_active_morphs`.

Trace: [`renderable_system.rs:1386`](../../src/engine/ecs/system/renderable_system.rs#L1386).

`set_active_morphs` compares the complete sparse vector to its previous value.
Only a difference replaces `morph_inputs` and sets that individual
`VisualInstance::deformation_dirty` flag. Unchanged factor values therefore do
not invalidate the deformation cache.

Trace: [`visual_world.rs:394`](../../src/engine/graphics/visual_world.rs#L394).

Skin palette changes use the same dirty flag: allocation changes and changed
matrices dirty their instance. `sync_deformation_ranges` maintains persistent
output ranges and dirties after a range/size change.

Traces: [`visual_world.rs:560`](../../src/engine/graphics/visual_world.rs#L560)
and [`visual_world.rs:2430`](../../src/engine/graphics/visual_world.rs#L2430).

## Vulkano compute path

On first upload of a skinned `CpuMesh`, `VulkanoState::upload_mesh` adds base
vertices and skin data to renderer-wide arenas. When morph targets exist, it
appends their target-major dense deltas to the morph arena, uploads/replaces the
device-local `GpuMorphDelta` buffer, and records `morph_delta_base` in the mesh's
GPU metadata.

Traces: [`vulkano_renderer.rs:4802`](../../src/engine/graphics/vulkano_renderer.rs#L4802),
[`vulkano_renderer.rs:5118`](../../src/engine/graphics/vulkano_renderer.rs#L5118),
and [`vulkano_renderer.rs:343`](../../src/engine/graphics/vulkano_renderer.rs#L343).

For every dirty visual instance, `record_dirty_deformations` creates one
`GpuActiveMorph` per sparse factor:

```text
delta_base = mesh.morph_delta_base + target_index * mesh.vertex_count
weight     = effective factor
```

`GpuDeformationJob.active_morph_base` and `active_morph_count` select its active
records. Traces: [`vulkano_renderer.rs:3125`](../../src/engine/graphics/vulkano_renderer.rs#L3125)
and [`vulkano_renderer.rs:3230`](../../src/engine/graphics/vulkano_renderer.rs#L3230).

The compute shader adds all active position and normal deltas for a vertex, then
applies its skin matrix. It deliberately skips a skin-buffer read for
`bones_count == 0`. Trace:
[`mesh-deformation.comp:30`](../../assets/shaders/mesh-deformation.comp#L30).
The CPU reference test proves morph-before-skin ordering at
[`deformation_reference.rs:330`](../../src/engine/graphics/deformation_reference.rs#L330).

## Cache and render consumers

Deformation output is renderer-wide. The first command buffer that consumes dirty
instances records compute work into the shared deformed-vertex cache; later views
use that same result. This includes the Winit/window view, mirror captures, and
OpenXR left/right eyes.

Traces: [`vulkano_renderer.rs:3500`](../../src/engine/graphics/vulkano_renderer.rs#L3500)
and [`vulkano_renderer.rs:3928`](../../src/engine/graphics/vulkano_renderer.rs#L3928).

At command-buffer completion, `mark_deformations_clean` clears dirty flags, so
unchanged frames do not dispatch again. Traces:
[`vulkano_renderer.rs:4589`](../../src/engine/graphics/vulkano_renderer.rs#L4589)
and [`visual_world.rs:2470`](../../src/engine/graphics/visual_world.rs#L2470).

## Current constraints and follow-ups

| Topic | Current state | Follow-up |
| --- | --- | --- |
| Semantic automapping | Explicit map only. | Add conventional-label/VRM expression inference if desired. |
| Morph-only meshes | Shader is guarded, but output ranges currently require nonzero `bones_count`. | Route morph-only meshes through cache allocation/scheduling. |
| Active GPU palette | Sparse factors persist in `VisualWorld`, but active GPU records are made for dirty compute work. | Give each instance a stable palette range and upload only changed ranges. |
| Delta arena | Rebuilt/replaced when another target-bearing skinned mesh uploads. | Accept rare-load cost or make arena growth incremental. |
| Target bounds | Renderer derives a global bound rather than holding per-mesh target count. | Store `morph_target_count` in `VulkanoGpuMesh`. |
| Dirty clearing | All instances are marked clean after command-buffer construction. | Add mixed-view and failure-path regression tests. |
| Test coverage | Factor-policy and CPU reference tests exist. | Add GLTF → visual input → GPU/cache and Bisket desktop/XR coverage. |

## Practical Bisket debug order

1. Confirm direct-closure OSC reaches `XREyeTrackingSystem`.
2. Confirm GLTF-child `MorphTargetMap` resolves the intended labels.
3. Inspect `GLTFComponent::morph_factors` for matching driver values.
4. Confirm expected primitives have `MorphTargetBindingComponent`.
5. Confirm changed factors create sparse visual input and dirty only those instances.
6. Confirm the compute job has active records pointing at target-major deltas.
7. Compare desktop, mirror, and both XR eyes; all should use the shared cache.

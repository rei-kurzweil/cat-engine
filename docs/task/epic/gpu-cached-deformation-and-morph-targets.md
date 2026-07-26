# GPU-cached deformation and morph targets

Status: active epic.

## Purpose

Make event-driven, compute-cached mesh deformation the current renderer optimization priority.
Skinning and morph evaluation should run once when deformation inputs change, rather than once per
vertex for every graphics pass and view.

The cached mesh-local result must be shared by ordinary window rendering, mirrors,
emissive/bloom extraction, and both XR eyes while preserving the renderer's existing instanced
draw organization.

## Execution order

### Phase 1: cache skinned deformation

Implement [compute-cached mesh deformation](../compute-cached-mesh-deformation.md):

1. Keep base vertices, joint attributes, and the persistent bones palette as compute inputs.
2. Allocate stable per-renderable output ranges containing mesh-local positions and normals.
3. Dispatch deformation only for dirty renderables and coalesce changed palette and job uploads.
4. Insert the compute-write-to-vertex-read dependency before the first consumer.
5. Read cached output from a lightweight graphics vertex shader in every dependent pass and view.
6. Remove graphics-stage skinning after visual and numerical parity is established.

Static meshes retain their current graphics path.

### Phase 2: add morph targets and editor controls

After Phase 1 passes its validation gate, implement
[morph targets and the editor panel](../morph-targets-and-editor-panel.md):

1. Normalize glTF position and normal target accessors into one immutable dense representation.
2. Upload target-major device-local morph data.
3. Extend dirty deformation jobs with compact nonzero target weights and apply morphs before
   skinning.
4. Add runtime morph state and stable primitive/target update intents.
5. Add a selected-glTF editor panel with grouped, draggable weight controls.

## Requirements and constraints

- A compute-capable Vulkan queue and sufficient storage-buffer limits are renderer requirements.
  Devices that cannot support the deformation path must fail renderer initialization with a clear
  capability error.
- Do not retain graphics-stage vertex skinning as a compatibility path.
- Compute capability is checked from Vulkan queue flags, features, and limits, not from a GPU-name
  allowlist. Vulkan queues advertise compute support with `VK_QUEUE_COMPUTE_BIT`; NVIDIA supports
  Vulkan on the targeted Pascal hardware. See the
  [Khronos queue model](https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html) and
  [NVIDIA Vulkan support](https://developer.nvidia.com/Vulkan).
- Deformation output remains mesh-local. Model, camera, view, and projection changes do not dirty
  it.
- One deformation generation is reused across desktop, mirrors, extraction passes, and both XR
  eyes.
- Stable ranges, resizing, and synchronization must account for every frame in flight.
- Preserve instanced batching: instance data selects cached output through `deformed_base`.

## Measurement record

Each phase records before/after evidence for a representative skinned workload:

- GPU time spent on deformation and affected graphics passes
- compute dispatch count and vertices dispatched
- deformation input/output bytes and bytes uploaded
- draw and instance counts
- number of window, mirror, extraction, and XR-eye consumers
- visual parity evidence
- hardware, runtime, build profile, and date

Validate on a GTX 1050 Ti Mobile for desktop rendering and a GTX 1080 for desktop and XR
rendering. Other hardware may be used during development, but does not replace these acceptance
targets.

## Validation gates

Phase 1 is complete only when:

- CPU-reference tests cover identity, single-joint, four-weight, normal deformation, and multiple
  independently posed instances.
- Unchanged frames perform zero deformation dispatches and zero deformation uploads.
- Dirtying one rig updates only its allocated ranges.
- Window, mirror, emissive/bloom, XR-left, and XR-right consumers produce identical deformation.
- Before/after GPU timings demonstrate that deformation is evaluated once rather than once per
  graphics pass or view.
- Stable allocation reuse/freeing, resize/reallocation, frames-in-flight hazards, and unsupported
  devices are covered.
- The old graphics-stage skinning path is removed.

Phase 2 is complete only when:

- Dense and sparse glTF accessors normalize to identical target-major arrays.
- Position-only and position-plus-normal targets work; malformed counts fail before partial GPU
  state is created.
- Morph-before-skin ordering and multiple active targets match a CPU reference.
- Defaults, naming/grouping, slider behavior, selection changes, and reload reset are covered.
- A real model's expression targets can be changed live from the editor panel.
- The required GTX 1050 Ti Mobile and GTX 1080 validation is recorded.

## Deferred follow-ups

The following are deliberately outside the two phases:

- content/version-keyed disk caching of normalized morph data
- morph animation channels
- VRM expression presets
- serialized named morph poses
- retargeting
- morph-aware LOD
- tangent morph deltas

A future serializable `MorphPoseComponent` may represent named `(target key, value)` collections
with a pose-like `apply(gltf)` API. It is distinct from a raw morph target and is not part of v1.


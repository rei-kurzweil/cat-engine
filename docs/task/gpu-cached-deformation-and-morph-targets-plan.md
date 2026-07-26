# GPU-Cached Deformation and Morph Targets Documentation

Status: complete.

## Summary

Create a new epic that makes compute-cached mesh deformation the current optimization priority:

1. Move skinning from repeated graphics vertex-shader evaluation into an event-driven compute pass.
2. Extend that deformation pass with dense morph targets and a selected-glTF editor panel.
3. Preserve instanced drawing and reuse one deformation result across desktop, mirrors, emissive
   passes, and both XR eyes.

Pause the unfinished clipping phases in the existing renderer epic and mark the old dense-vs-sparse
blend-shape proposal as superseded.

## Documentation Changes

- Add `docs/task/epic/gpu-cached-deformation-and-morph-targets.md`:
  - Link the compute-caching task as Phase 1 and morph-target/editor support as Phase 2.
  - Require before/after GPU timings, dispatch counts, deformation bytes, draw counts, and visual
    parity.
  - Target GTX 1080 and GTX 1050 Ti Mobile validation.
  - Require a compute-capable Vulkan queue and sufficient SSBO limits; do not retain the old
    vertex-skinning compatibility path. Vulkan exposes compute through queues carrying
    `VK_QUEUE_COMPUTE_BIT`, and NVIDIA supports Vulkan across Pascal-era hardware.
    [Khronos queue model](https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html),
    [NVIDIA Vulkan support](https://developer.nvidia.com/Vulkan).

- Add `docs/task/compute-cached-mesh-deformation.md`:
  - Keep base vertices, joint attributes, and the persistent bones palette as compute inputs.
  - Allocate stable per-renderable ranges in a device-local deformation SSBO containing mesh-local
    position and normal.
  - Add `deformed_base` to instance data. A lightweight cached-deformation graphics vertex shader
    reads `deformed_base + gl_VertexIndex`, preserving existing instanced batches.
  - Dispatch one thread per vertex only for dirty deformation jobs. Coalesce jobs and changed
    palette ranges before uploading.
  - Reuse the existing event-driven `SkinnedMeshSystem` binding invalidation: joint,
    mesh-transform, mesh-resource, or allocation changes dirty only affected renderables.
  - Record compute before the first dependent view and insert an explicit
    compute-write-to-vertex-read barrier. Every desktop, mirror, extraction, and XR-eye pass in that
    deformation generation consumes the same cached output.
  - Keep output in mesh-local space so ordinary model/camera changes do not trigger deformation.
  - Replace graphics-stage skinning after parity validation; static meshes retain their current
    path.
  - Cover stable-range reuse/freeing, multiple instances of one mesh with different rigs,
    frames-in-flight hazards, resize/reallocation, no-dirty-work frames, and devices lacking the
    required capability producing a clear renderer initialization error.

- Add `docs/task/morph-targets-and-editor-panel.md`:
  - Import glTF `POSITION` and `NORMAL` target deltas and imported mesh/node default weights;
    tangents, animation channels, VRM presets, retargeting, LOD, and persistence remain out of v1.
  - Use the glTF reader's sparse-accessor expansion once, validate each target against vertex
    count, and store immutable target-major dense arrays. No dense/sparse analyzer or storage
    variants.
  - Retain normalized arrays in the existing URI/mesh CPU cache and upload immutable device-local
    morph buffers. Track a content/version-keyed disk cache explicitly as a later optimization.
  - Keep host weights dense and use a compact nonzero `(target_index, weight)` list for compute.
    Apply morph deltas in bind-pose space before skinning.
  - Add runtime morph components and weight-update intents with stable primitive/target identity.
    Weight changes dirty only affected deformation ranges and upload only changed weight data.
  - Add `EditorPanel::MorphTargets`, its panel kind/asset, selection synchronization, and a
    scrollable panel for the selected glTF instance.
  - Resolve selection from the selected `GLTFComponent` or its nearest glTF ancestor. Group
    equal-name targets across primitives; unnamed targets use primitive-qualified fallback labels.
    If same-name members have different current weights, split them into primitive-qualified rows
    for v1 and record mixed-value grouping as v2.
  - Each row contains a label, numeric value, track, and `Draggable` thumb. On drag start, capture
    the thumb's local X and weight; drag movement converts into track-local X, clamps position and
    weight to `[0,1]`, updates the thumb, and emits one grouped weight update. Do not depend on
    handler ordering by rereading a transform mutated by `DraggableSystem`.
  - Slider changes are runtime preview state and reset to imported defaults after reload.
  - Reserve a future serializable `MorphPoseComponent` concept—distinct from a raw morph
    target—for named presets containing `(target key, value)` entries and a pose-like
    `apply(gltf)` API.

- Update existing trackers:
  - Mark `docs/spec/blend-shapes.md` superseded, link the new epic/tasks, and identify reusable
    background material while explicitly rejecting its runtime dense/sparse analyzer, multiple
    morph graphics pipelines, and compute deferral.
  - Update `docs/task/epic/renderer_optimisation.md` so completed draw-cache work remains recorded,
    CPU clip culling becomes deferred, and the new deformation epic is the current optimization
    direction.

## Test and Acceptance Coverage

- Compute task:
  - CPU reference versus compute output for identity, single-joint, four-weight, normal
    deformation, and multiple independently posed instances.
  - Zero dispatches and zero deformation uploads on unchanged frames.
  - One dirty rig updates only its ranges while other cached instances remain untouched.
  - Identical output across window, mirror, emissive/bloom, XR-left, and XR-right consumers.
  - Before/after GPU timings demonstrating deformation is evaluated once rather than once per
    graphics pass/view.

- Morph task:
  - Dense and sparse glTF accessors normalize to identical target-major arrays.
  - Position-only and position-plus-normal targets; malformed counts fail clearly without partial
    GPU state.
  - Morph-before-skin ordering and multiple active targets match a CPU reference.
  - Named target grouping, unnamed fallback names, duplicate-name split behavior, default weights,
    slider clamping, selection changes, and reload reset.
  - A real model's expression targets such as happy/sad/angry can be changed live from the panel.
  - Desktop validation on GTX 1050 Ti Mobile and desktop/XR validation on GTX 1080.

## Assumptions

- This change creates and updates documentation only; implementation follows through the two child
  tasks.
- Compute capability is a renderer requirement, checked through Vulkan features/limits rather than
  GPU-name allowlists.
- Deformed vertices remain mesh-local and cached per rendered instance.
- Dense morph normalization is the only v1 storage representation; sparse active weights remain an
  execution optimization.
- Disk caching, morph animation, VRM expression presets, serialized presets, retargeting, and LOD
  remain explicitly tracked follow-ups.

## Implementation record

Completed on 2026-07-26:

- Added the active
  [GPU-cached deformation and morph-target epic](epic/gpu-cached-deformation-and-morph-targets.md).
- Added the Phase 1 [compute-cached mesh deformation](compute-cached-mesh-deformation.md) task.
- Added the Phase 2 [morph targets and editor panel](morph-targets-and-editor-panel.md) task.
- Marked the old [blend-shape draft](../spec/blend-shapes.md) as superseded while retaining useful
  background research.
- Updated the [renderer optimization tracker](epic/renderer_optimisation.md) to preserve completed
  render-stream evidence, defer clipping work, and make cached deformation the current priority.

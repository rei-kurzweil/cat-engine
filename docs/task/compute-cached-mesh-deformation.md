# Compute-cached mesh deformation

Status: complete; accepted on same-hardware VR performance validation.

Epic: [GPU-cached deformation and morph targets](epic/gpu-cached-deformation-and-morph-targets.md)

## Problem

Before commit `ef592dc`, skinned vertices were evaluated in graphics vertex shaders. The same
bones and base mesh were therefore processed again for each graphics pass and view, including
ordinary desktop, mirrors, emissive/bloom extraction, and both XR eyes.

Skin matrices are already maintained as a shared palette with stable per-instance ranges, and
`SkinnedMeshSystem` updates bindings when joints, mesh transforms, mesh resources, or allocations
change. The source now contains a persistent deformed-vertex cache driven by those changes. The
task was accepted complete on 2026-08-09 after VR with mirrors and skinning improved from below
30 FPS to a consistent 60 FPS on the same hardware.

## Goal

Run skinning in a compute pass only for dirty renderables. Store mesh-local position and normal in
a device-local deformation buffer, then let every graphics consumer read the same cached result.

Preserve existing instanced batches by adding `deformed_base` to instance data. A lightweight
cached-deformation graphics vertex shader uses:

```text
deformation_buffer[deformed_base + gl_VertexIndex]
```

Static meshes retain their existing path.

## Data ownership and layout

### Compute inputs

Keep these as deformation inputs:

- immutable base positions and normals
- immutable joint indices and weights
- the persistent bones palette
- one job record per dirty renderable

Do not bake model or camera transforms into the cache. Output remains in mesh-local space so
ordinary transform and view changes do not trigger deformation.

### Persistent output

Maintain a device-local storage buffer containing position and normal for every allocated
deformed vertex. Each skinned renderable owns a stable range for as long as its vertex count and
allocation remain valid.

The allocator must:

- reuse and coalesce freed ranges
- support multiple instances of one mesh with different rigs
- preserve an instance's base when its required size is unchanged
- reallocate safely when vertex count or backing-buffer capacity changes
- update instance `deformed_base` only when allocation changes
- avoid exposing a recycled range while an older frame can still read it

Choose and document the concrete packed/aligned GPU representation before implementation. Record
the bytes per vertex and total allocated/live deformation bytes in profiling output.

### Dirty jobs and uploads

Reuse `SkinnedMeshSystem`'s event-driven binding invalidation. Joint, mesh-transform,
mesh-resource, or allocation changes dirty only affected renderables.

Coalesce:

- adjacent changed bones-palette ranges before upload
- deformation jobs before dispatch
- output-range initialization or relocation work where safe

An unchanged frame must upload no deformation inputs and issue no deformation dispatch.

## Compute and graphics execution

- Dispatch one invocation per vertex for each dirty job.
- Record compute before the first dependent view or extraction pass for that deformation
  generation.
- Insert an explicit dependency from compute shader storage writes to graphics vertex-stage
  storage reads.
- Ensure all window, mirror, emissive/bloom, and XR-eye consumers in that generation observe the
  same completed output.
- Do not introduce a separate deformation dispatch per view or pass.
- Keep dispatch and resource ownership correct across every frame in flight.

After parity validation, replace skinned graphics vertex shaders with a cached-deformation vertex
shader that performs only the remaining model/view/projection and per-pass work. Remove the old
graphics-stage skinning shader and pipeline path rather than retaining a fallback.

## Capability handling

Renderer initialization must verify:

- a selected queue supports `VK_QUEUE_COMPUTE_BIT`
- required storage-buffer descriptor counts and ranges are available
- required shader/storage limits cover the chosen layouts

Failure must produce a clear initialization error naming the missing capability or limit. Do not
use GPU-name allowlists and do not silently fall back to graphics-stage skinning.

## Instrumentation

Capture before and after, using the same scene, cameras, passes, and build profile:

- GPU deformation time and affected graphics-pass GPU time
- compute dispatch count
- dirty jobs and vertices dispatched
- bones-palette bytes uploaded
- job bytes uploaded
- live, allocated, and resized deformation-buffer bytes
- draw and instance counts per consumer
- count of window, mirror, extraction, and XR-eye consumers

The comparison must demonstrate that adding consumers no longer repeats skinning evaluation.

## Test plan

### Numerical reference tests

Compare compute output with a CPU reference for:

- identity matrices
- a single joint
- four weighted joints
- position and normal deformation
- non-uniform and independently posed instances of the same mesh
- multiple dirty ranges in one dispatch batch

Use tolerances appropriate to the selected GPU representation and document them.

### Dirty-work tests

- An unchanged frame records zero dispatches and zero deformation uploads.
- Updating one rig dirties only its renderable ranges.
- Updating a model or camera transform alone does not dirty deformation.
- Joint, mesh-resource, vertex-count, and allocation changes do dirty the expected ranges.
- Freed ranges are reused without corrupting live instances.

### Lifetime and synchronization tests

- Grow and replace the backing buffer while frames are in flight.
- Free and recycle ranges only after older readers are safe.
- Validate barriers and queue ownership when compute and graphics use the same queue family and,
  if supported by the renderer, distinct queue families.
- Verify descriptor and instance-data refresh after reallocation.
- Verify a no-dirty-work frame remains valid after prior dirty work.

### Consumer parity

For one deformation generation, compare cached position and normal results used by:

- ordinary window rendering
- mirror rendering
- emissive/bloom extraction
- XR left eye
- XR right eye

Draw and instance counts must remain unchanged except where measurement exposes an existing bug.

## Completion criteria

- Deformation runs once per dirty renderable generation, not once per graphics pass or view.
- Every dependent consumer reads the shared cached output.
- Instanced drawing is preserved through `deformed_base`.
- Unchanged frames perform no deformation upload or dispatch.
- Allocation, resizing, frames-in-flight, and unsupported-device behavior are tested.
- CPU-reference and visual parity checks pass.
- Before/after evidence is recorded on the hardware required by the epic.
- Graphics-stage skinning and its compatibility path are removed.

## Implementation record

### Slice 1: CPU deformation reference

Completed on 2026-07-26:

- Added a test-only CPU reference that mirrors the existing
  `skinned-toon-mesh.vert` conventions: column-major matrices, direct four-weight matrix blending,
  identity fallback, mesh-local position output, and normalized 3x3-transformed normals.
- Covered identity, single-joint, four-weight, normal deformation, and two independently posed
  instances sharing one base mesh.
- Added clear failures for mismatched vertex-attribute counts and out-of-range joints.
- `cargo test deformation_reference --lib`: 6 passed.

The reference remains independent of the compute implementation so GPU readback tests can use it
as an oracle.

### Slice 2: compute-cache source cutover

Implemented in commit `ef592dc` on 2026-07-26:

- Added `mesh-deformation.comp` and a cached-deformation graphics vertex shader.
- Added explicit CPU/GPU layouts for base vertices, skin attributes, jobs, workgroups, active
  morph records, and packed 16-byte deformed vertices.
- Added a stable, coalescing deformation-range allocator and stored `deformed_base` per visual
  instance.
- Added dirty-generation tracking, changed-palette uploads, compute dispatch construction, and
  deformation counters.
- Added renderer capability checks for compute workgroup and storage-buffer limits.
- Replaced graphics-stage skinning with reads from the persistent deformation output; the old
  graphics-stage skinning shader was removed.
- Added CPU coverage for morph-before-skin ordering, range allocation/reuse, octahedral normal
  packing, and dirty-state behavior.

Focused validation on 2026-07-28:

- `cargo test deformation --lib`: 17 passed.

Performance acceptance on 2026-08-09:

- On the same VR hardware and representative workload with mirrors and skinning, observed frame
  rate improved from subjectively poor performance below 30 FPS to a consistent 60 FPS.
- This result accepts the compute-cached skinning cutover as complete and unblocks morph-target
  work in the shared deformation pass.

The following remain worthwhile non-blocking hardening and measurement follow-ups:

- add GPU readback/parity coverage against the CPU reference;
- prove unchanged frames issue zero deformation uploads and dispatches;
- validate one-generation reuse across window, mirror, extraction, and both XR eyes;
- cover backing-buffer growth, range retirement, descriptor refresh, and frames-in-flight hazards;
- remove or reuse per-dirty-frame job, workgroup, placeholder morph, and descriptor allocations;
- record before/after GPU timings and visual results on the required GTX 1050 Ti Mobile and
  GTX 1080 targets.

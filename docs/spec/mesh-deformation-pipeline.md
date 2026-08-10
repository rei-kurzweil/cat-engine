# Mesh deformation pipeline

This document is the authoritative Phase 1 contract for compute-cached skinning. Static meshes
continue through the ordinary vertex path. A skinned instance is deformed in mesh-local space by
compute, cached once, and reused by every window, mirror, emissive-extraction, and XR draw that
observes that deformation generation.

Phase 1 includes operational low-level morph buffers and morph-before-skin compute logic. glTF
morph import, ECS morph blend factors, animation, intents, persistence, and editor controls are
Phase 2.

## GPU data contract

Rust declarations live in `engine::graphics::deformation`; GLSL declarations live in
`mesh-deformation.comp`. All offsets below are bytes and all shader buffers use `std430`.
Compile-time Rust assertions enforce size, alignment, and the nonzero field offsets.

| Type | Fields and offsets | Size / alignment |
|---|---|---|
| `GpuBaseDeformationVertex` | `vec4 position` 0, `vec4 normal` 16 | 32 / 16 |
| `GpuDeformationSkinVertex` | `uvec4 joints` 0, `vec4 weights` 16 (skin joint weights) | 32 / 16 |
| `GpuMorphDelta` | `vec4 position_delta` 0, `vec4 normal_delta` 16 | 32 / 16 |
| `GpuActiveMorph` | `uint delta_base` 0, `float weight` 4 (morph blend factor) | 8 / 4 |
| `GpuDeformationJob` | eight `uint`s: base, skin, output, vertex count, bones base/count, active-morph base/count | 32 / 4 |
| `GpuDeformationWorkgroup` | `uint job_index` 0, `uint first_vertex` 4 | 8 / 4 |
| `GpuDeformedVertex` | `vec3 position` 0, `uint packed_normal` 12 | 16 / 4 |
| `DeformationRange` | CPU-only `uint base` 0, `uint vertex_count` 4 | 8 / 4 |

The permanent v1 output format is 16 bytes. Its normal is octahedrally encoded as two signed
16-bit values using the `[-32767, 32767]` quantizer. `0x80008000` is reserved for a zero normal.
Non-finite normals are rejected before dispatch; the encoder never emits the sentinel for a
nonzero normal. The graphics shader decodes the value and normalizes again after `mat3(model)`.
The CPU/GPU angular error contract is at most `0.0001` radians.

## Compute descriptors and indexing

Compute set 0 has exactly eight storage-buffer bindings:

| Binding | Buffer | Access |
|---:|---|---|
| 0 | base deformation vertices | read |
| 1 | skin vertices | read |
| 2 | bone matrices | read |
| 3 | morph deltas | read |
| 4 | active morphs | read |
| 5 | deformation jobs | read |
| 6 | workgroup records | read |
| 7 | deformed output cache | write |

Base and skin arenas are global and immutable after each mesh upload. Morph deltas are
target-major. For local vertex `v`, active morph `a` reads:

```text
morph_deltas[a.delta_base + v]
```

Workgroups contain 64 invocations. A job with `N` vertices produces `ceil(N / 64)` records whose
`first_vertex` values are `0, 64, ...`. Invocation indexing is:

```text
record = workgroups[push.workgroup_base + gl_WorkGroupID.x]
job = jobs[record.job_index]
v = record.first_vertex + gl_LocalInvocationID.x
base = base_vertices[job.base_vertex + v]
skin = skin_vertices[job.skin_vertex + v]
output = output_vertices[job.output_vertex + v]
```

One dispatch covers all accumulated records. It is split only at
`maxComputeWorkGroupCount[0]`; the push constant advances `workgroup_base`.

## Deformation algorithm

For each in-range invocation:

1. Load the mesh-local base position and normal.
2. Add every active position and normal delta. Do not normalize between targets.
3. Sum the four skin joint weights. Select identity if `bones_count == 0` or the sum is not greater
   than zero.
4. Otherwise blend all four matrices directly. Do not renormalize the skin joint weights.
5. Transform position by the 4x4 matrix and normal by `mat3(skin)`.
6. Normalize once in the octahedral packer and write one `GpuDeformedVertex`.

Attribute-count mismatches, incomplete skin attributes, non-finite inputs, out-of-range joints,
and invalid morph ranges fail before dispatch. There is no partial job submission.

## Allocation, dirtiness, and lifetime

Each `VisualInstance` has CPU-only `deformed_base` and `deformed_count`. A stable first-fit range
allocator preserves the range while vertex count is unchanged, coalesces adjacent frees, reuses
removed ranges, and grows to the next power of two. The cache is device-local. On replacement,
the old live prefix is copied before descriptors switch; resource references keep the old buffer
alive until commands that mention it complete.

Bone palette writes are compared with the existing range. Identical matrices create no dirty
interval and do not dirty deformation. Changed and freed ranges are coalesced. A mesh allocation,
output allocation, changed palette, or active morph-blend-factor change dirties its affected
instances.
Model, camera, material, visibility, and viewport changes do not. An unchanged generation records
no bone/job/morph-blend-factor upload and no dispatch.

Immutable base/skin arenas and the output cache are renderer-global, not swapchain- or eye-local.
The first command buffer observing dirtiness records uploads and compute before
`begin_rendering()`. Vulkan resource dependencies order transfer writes before compute reads,
prior vertex reads before output rewrites, and compute writes before vertex reads. The graphics
queue must support compute; no asynchronous compute queue is used.

All command buffers that touch the shared cache must remain on the renderer-wide submission
ordering chain. A saved command buffer owns every buffer and descriptor it references, so arena
replacement cannot invalidate an older submission.

## Graphics consumption

`InstanceData` exposes `i_deformed_base` at vertex location 10. The cached skinned vertex shader
loads:

```text
deformed_vertices[i_deformed_base + gl_VertexIndex]
```

UVs and indexed topology still come from the ordinary mesh vertex/index buffers. The shader then
applies model, view, and projection transforms and emits the existing varyings. All skinned opaque,
cutout, transparent, emissive, extraction, mirror, and XR pipelines use this shader. Graphics-stage
skinning is not a fallback.

## Device requirements and telemetry

Initialization rejects a graphics queue without compute, local size or invocation limits below
64, fewer than eight compute-stage storage buffers, or a zero X dispatch limit, and reports the
failed capability.

The renderer accumulates dispatches, jobs, workgroups, dirty vertices,
bone/job/morph-blend-factor upload bytes, live/allocated cache bytes, and resizes. GPU timestamp
collection is enabled where the
renderer's profiling path provides timestamp queries. Performance acceptance records before/after
timings and bandwidth on GTX 1080 and retains the epic's GTX 1050 Ti desktop validation.

The 64-thread group follows NVIDIA's warp-multiple guidance without introducing a dedicated
asynchronous compute queue:

- <https://developer.nvidia.com/blog/?p=71300>
- <https://developer.nvidia.com/blog/vulkan-dos-donts/>

Synchronization follows the Khronos compute-to-graphics model:

- <https://github.khronos.org/Vulkan-Site/guide/latest/synchronization_examples.html>

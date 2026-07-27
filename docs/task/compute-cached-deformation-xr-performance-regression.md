# Compute-cached deformation XR performance regression

Status: open regression.

Related:

- [Compute-cached mesh deformation](compute-cached-mesh-deformation.md)
- [Mesh deformation pipeline](../spec/mesh-deformation-pipeline.md)
- [GPU-cached deformation and morph targets](epic/gpu-cached-deformation-and-morph-targets.md)
- [Opt-in System, MMS, Vulkano, and XR Profiling](opt-in-system-mms-vulkano-xr-profiling.md)

## Observed regression

After the compute-cached skinning cutover, VR testing in:

- `vtuber-secondary-motion`
- `vtuber-mirror-example`

is substantially choppier and runs at a lower frame rate than before. Enabling spring-bone
visualization can make the application repeatedly miss presentation deadlines: the rendered
Mittens scene flickers out and the SteamVR environment / aurora becomes visible between engine
frames.

Before the cutover, these VR-only examples, including mirror use, were approximately in the
30–60 FPS range on the test system. The current result is materially lower.

Treat SteamVR-environment flicker as a missed-XR-deadline failure, not as an acceptable gradual
performance reduction.

## Current diagnosis

The leading cause is renderer submission serialization rather than the arithmetic cost of the
compute shader itself.

`render_xr_eye_offscreen` currently:

1. calls `render_mirror_captures`
2. submits the eye command buffer through the renderer-wide future chain
3. signals a fence and calls `wait(None)`

OpenXR calls `render_xr_eye_offscreen` once for each eye. Therefore mirror captures are requested
again for the second eye, and every mirror capture also submits and calls `wait(None)`.

The effective shape can be:

```text
prior window/shared-cache work
  -> mirror capture 0 -> CPU fence wait
  -> ...additional captures, each with a CPU fence wait
  -> XR eye 0 -> CPU fence wait
  -> mirror captures again, each with a CPU fence wait
  -> XR eye 1 -> CPU fence wait
  -> XR swapchain copy
```

The renderer-wide ordering chain is required to protect the shared deformation cache, but that
does not require draining the queue on the CPU between every consumer. The current implementation
turns cache lifetime safety into globally synchronous rendering.

Continuously changing bones expose additional per-frame costs in `record_dirty_deformations`:

- allocate a host staging buffer for each dirty bone interval
- allocate and upload device-local job and workgroup buffers
- allocate and upload dummy morph and active-morph buffers even though Phase 1 has zero active
  morphs
- create a new deformation descriptor set
- rescan all skin vertices of every dirty instance to validate immutable joint indices

Secondary motion makes affected palettes dirty continuously. Avatar control, IK, and headset or
controller tracking can also make a rig dirty even when secondary motion is disabled.

## Required test matrix

Test in a release build on the same headset, runtime, render resolution, MSAA setting, avatar, and
scene. Do not use only “secondary motion on” versus “off”; use the following controls.

| Case | Avatar / rig activity | Secondary motion | Visualization |
|---|---|---|---|
| A | No avatar, XR environment only | Off | Off |
| B | Avatar loaded; normal XR avatar control / IK | Off | Off |
| C | Same avatar and pose source as B | On | Off |
| D | Same as C | On | On |

Run each case:

- without mirrors
- with the normal mirror configuration

Where practical, also repeat with the desktop companion/window consumer disabled and enabled.
This separates:

- the base OpenXR cost
- continuously dirty cached skinning from ordinary avatar tracking
- secondary-motion simulation and additional deformation dirtiness
- visualization draw cost
- mirror duplication and synchronization
- cross-consumer serialization involving the desktop window

Use `vtuber-secondary-motion` as the secondary-motion and visualization surface.
Use `vtuber-mirror-example` as the mirror/render-view surface. If the examples do not expose
runtime toggles for every matrix cell, add temporary or documented launch-time switches rather
than maintaining divergent copied scenes.

## Measurements

For every matrix cell, record:

- headset/runtime target refresh rate
- delivered application FPS and missed/reprojected frame count
- XR `wait_frame`, eye-render, copy, and frame-submit CPU durations
- mirror capture count and total mirror GPU time per XR frame
- XR eye count and GPU time per eye
- number of Vulkan queue submissions and CPU fence waits per XR frame
- deformation dispatch count, jobs, workgroups, and dirty vertices
- bone, job, workgroup, and morph upload bytes
- deformation buffer/descriptor allocations and resizes
- window, mirror, extraction, and XR draw/instance counts

Add a temporary submission trace if the profiler cannot yet expose queue submissions and waits.
The trace must make it possible to verify that a logical mirror capture is not accidentally
rendered multiple times for the same deformation generation and viewer family.

## Fix requirements

### 1. Remove per-consumer CPU queue drains

- Do not call `wait(None)` between mirror captures or between XR eyes.
- Preserve cache hazards through GPU-side ordering in the renderer-wide submission chain.
- Batch compatible command buffers into one submission where practical.
- Perform only the synchronization required for the raw XR swapchain copy/handoff.
- Do not restore unsafe independent `sync::now` submissions as a performance workaround.

### 2. Schedule deformation and mirrors once

- Record dirty deformation once before the first consumer of the generation.
- Reuse that output for mirrors, both XR eyes, extraction, and the window.
- Schedule each required mirror capture once per logical frame/viewer-family combination, not once
  because each eye-render entry point independently asks for all captures.
- Keep stereo mirror captures distinct where their camera inputs genuinely differ.

### 3. Make dirty-frame data persistent

- Use bounded, reusable frame-slot staging for bones, jobs, workgroups, and active morph weights.
- Keep persistent dummy/empty morph resources for Phase 1 rather than allocating them per dispatch.
- Reuse descriptor sets until one of their backing buffers is replaced.
- Grow buffers geometrically and retire old resources only after submissions referencing them have
  completed.

### 4. Move immutable validation out of the frame loop

- Validate joint indices and immutable mesh skin data during mesh upload.
- Validate instance palette bounds when the mesh-to-rig binding changes.
- Do not scan every vertex merely because bone matrices changed.

### 5. Preserve the deformation contract

- Do not reintroduce graphics-stage skinning.
- An unchanged deformation generation must still perform no deformation upload or dispatch.
- Model/camera-only changes must not dirty deformation.
- Shared output lifetime and transfer-to-compute-to-vertex hazards must remain validation-clean.

## Acceptance criteria

- SteamVR environment/aurora does not flicker through during Cases A–D.
- There are no CPU fence waits between mirror captures or XR eyes.
- Both XR eyes reuse one completed deformation generation.
- Mirror work is not duplicated by the per-eye entry point.
- Cases B–D show no per-frame device-buffer or descriptor allocation after warm-up unless capacity
  actually grows.
- Case B quantifies the cost of ordinary continuously changing avatar bones without secondary
  motion.
- The incremental costs from B → C and C → D are measured separately.
- Mirror-off versus mirror-on results identify the mirror cost without changing deformation job
  counts.
- Vulkan synchronization validation reports no transfer/compute/vertex or buffer-retirement
  hazards.
- Draw and instance counts remain correct for the window, mirror, extraction, and both XR eyes.
- Release-build VR performance returns to the pre-cutover range or any remaining difference is
  explained with captured CPU/GPU measurements.

## Hardware validation

Run the main XR regression matrix on the hardware/runtime where the regression was observed.
Retain the deformation epic's GTX 1080 and GTX 1050 Ti desktop validation, but do not treat
desktop-only results as sufficient evidence for this task.

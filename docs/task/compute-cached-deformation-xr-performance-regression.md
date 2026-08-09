# Compute-cached deformation XR performance regression

Status: complete; the regression is accepted resolved by same-hardware headset validation.

Related:

- [Compute-cached mesh deformation](compute-cached-mesh-deformation.md)
- [Mesh deformation pipeline](../spec/mesh-deformation-pipeline.md)
- [GPU-cached deformation and morph targets](epic/gpu-cached-deformation-and-morph-targets.md)
- [OpenXR render submission pipelining](openxr-render-submission-pipelining.md)
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
30–60 FPS range on the test system. An intermediate post-cutover result was materially lower.
After the mirror scheduling and submission-pipelining work, same-hardware VR testing with mirrors
and skinning improved from below 30 FPS to a consistent 60 FPS.

Treat SteamVR-environment flicker as a missed-XR-deadline failure, not as an acceptable gradual
performance reduction.

## Current implementation status

The original diagnosis below led to two source changes:

- commit `069c4a6` moved mirror scheduling out of the per-eye entry point, so a logical
  stereoscopic mirror is captured twice per XR frame rather than being requested again for the
  second headset eye;
- commit `c036271` implemented
  [OpenXR render submission pipelining](openxr-render-submission-pipelining.md) through Phase 4.
  Mirrors and both eyes now extend one GPU ordering chain without intermediate CPU waits. The raw
  XR copy signals one fence, and the CPU waits once before releasing the OpenXR swapchain image.

This removes the known repeated-mirror and per-consumer-wait structure in source. Same-hardware
headset testing accepted the regression as resolved on 2026-08-09: the representative mirrors and
skinning workload rose from below 30 FPS to a consistent 60 FPS. Persistent dirty-frame staging,
descriptor reuse, moving immutable validation out of the frame loop, and detailed captured timing
reports remain useful non-blocking follow-ups.

Profiling also separated a different bottleneck:
[spring-bone visualization command flushing](spring-bone-visualization-command-flush-performance.md)
adds about `66 ms` on the measured visualization-on workload without changing deformation jobs or
upload bytes. That issue is tracked independently.

## Original diagnosis

The leading diagnosis was renderer submission serialization rather than the arithmetic cost of
the compute shader itself.

Before commits `069c4a6` and `c036271`, `render_xr_eye_offscreen`:

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
does not require draining the queue on the CPU between every consumer. The old implementation
turned cache lifetime safety into globally synchronous rendering.

Continuously changing bones still expose additional per-frame costs in
`record_dirty_deformations`:

- allocate a host staging buffer for each dirty bone interval
- allocate and upload device-local job and workgroup buffers
- allocate and upload dummy morph and active-morph buffers even though Phase 1 has zero active
  morphs
- create a new deformation descriptor set
- rescan all skin vertices of every dirty instance to validate immutable joint indices

Secondary motion makes affected palettes dirty continuously. Avatar control, IK, and headset or
controller tracking can also make a rig dirty even when secondary motion is disabled.

## One example with four stable presets

Use one example executable for the primary comparison. Extend `vtuber-secondary-motion` so its
Rust harness accepts one named `--vr-perf-case` and configures the same base scene before timed
measurement begins.

Do not automatically switch cases in one running OpenXR session. A case remains stable for the
whole process. Separate process launches prevent resource creation, shader warm-up, visualization
spawn/removal, secondary-motion settling, and transition frames from leaking into the next
average.

The canonical four-run sequence is:

| Preset | Avatar / XR control | Mirror | Secondary motion | Visualization |
|---|---|---|---|---|
| `avatar_no_spring_no_mirror` | On | Off | Off | Off |
| `avatar_no_spring_mirror` | On | On | Off | Off |
| `avatar_spring_no_viz_mirror` | On | On | On | Off |
| `avatar_spring_viz_mirror` | On | On | On | On |

These four differences are intentional:

- run 1 → 2 isolates mirror scheduling
- run 2 → 3 isolates secondary-motion simulation plus its deformation dirtiness
- run 3 → 4 isolates spring-bone visualization

“Secondary motion off” does not mean “rig is static.” Normal XR avatar control, head tracking,
controller tracking, and IK remain enabled, so the first two runs still exercise continuously
changing cached skinning.

Support an optional `xr_empty` preset later if a no-avatar OpenXR floor is needed, but it is not
part of the first four-run regression gate.

Suggested commands:

```bash
cargo run --release --example vtuber-secondary-motion -- \
  --vr-perf-case avatar_no_spring_no_mirror

cargo run --release --example vtuber-secondary-motion -- \
  --vr-perf-case avatar_no_spring_mirror

cargo run --release --example vtuber-secondary-motion -- \
  --vr-perf-case avatar_spring_no_viz_mirror

cargo run --release --example vtuber-secondary-motion -- \
  --vr-perf-case avatar_spring_viz_mirror
```

The preset must be printed at startup and included in every report. An unknown or missing preset
in VR performance mode must fail with usage text instead of silently selecting a default.

The ordinary example invocation without `--vr-perf-case` should retain its current interactive
demo behavior.

## Sampling and report files

Add optional timing controls:

```text
--vr-perf-warmup-seconds <seconds>   default: 5
--vr-perf-sample-seconds <seconds>   default: 10
```

Start warm-up only after the OpenXR session is running, the avatar and mesh resources are loaded,
the selected preset has been applied, and headset frames are being presented. Reset all sampled
counters after warm-up. Measure the stable preset for the requested sample duration, write the
report, then exit normally.

Write Markdown reports under:

```text
docs/.debug/vr_perf/
```

Use descriptive, non-overwriting names rather than numeric `0.md`–`3.md`:

```text
<timestamp>__avatar_no_spring_no_mirror.md
<timestamp>__avatar_no_spring_mirror.md
<timestamp>__avatar_spring_no_viz_mirror.md
<timestamp>__avatar_spring_viz_mirror.md
```

For example:

```text
docs/.debug/vr_perf/20260726-231530__avatar_spring_no_viz_mirror.md
```

Create the directory if it is absent. Print the completed report path to the terminal. Report-file
failure must be visible but must not panic inside the OpenXR frame loop.

Average FPS is required, but it must be computed from headset frames presented during the sample,
not desktop window redraws. Also include:

- sampled frame count and elapsed duration
- arithmetic average FPS
- mean, median, p95, p99, minimum, and maximum headset frame time
- count and percentage of frames exceeding the runtime display interval
- selected preset and all resolved booleans
- build profile, GPU/device name, OpenXR runtime, headset target refresh rate, render extent, and
  MSAA setting

If the runtime exposes dropped, missed, or reprojected frame counters, record those too. Do not
invent zero values when a counter is unavailable; write `unavailable`.

The report should contain the more detailed renderer/deformation measurements below when their
instrumentation is available.

## Measurements

For every preset run, record:

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

Source status: implemented through OpenXR submission-pipelining Phase 4; same-hardware headset
verification accepted on 2026-08-09.

- Do not call `wait(None)` between mirror captures or between XR eyes.
- Preserve cache hazards through GPU-side ordering in the renderer-wide submission chain.
- Batch compatible command buffers into one submission where practical.
- Perform only the synchronization required for the raw XR swapchain copy/handoff.
- Do not restore unsafe independent `sync::now` submissions as a performance workaround.

### 2. Schedule deformation and mirrors once

Source status: mirror scheduling is once per XR viewer family, both eyes consume one ordered
deformation generation, and same-hardware performance verification is accepted.

- Record dirty deformation once before the first consumer of the generation.
- Reuse that output for mirrors, both XR eyes, extraction, and the window.
- Schedule each required mirror capture once per logical frame/viewer-family combination, not once
  because each eye-render entry point independently asks for all captures.
- Keep stereo mirror captures distinct where their camera inputs genuinely differ.

### 3. Make dirty-frame data persistent

Status: non-blocking follow-up.

- Use bounded, reusable frame-slot staging for bones, jobs, workgroups, and active morph weights.
- Keep persistent dummy/empty morph resources for Phase 1 rather than allocating them per dispatch.
- Reuse descriptor sets until one of their backing buffers is replaced.
- Grow buffers geometrically and retire old resources only after submissions referencing them have
  completed.

### 4. Move immutable validation out of the frame loop

Status: non-blocking follow-up.

- Validate joint indices and immutable mesh skin data during mesh upload.
- Validate instance palette bounds when the mesh-to-rig binding changes.
- Do not scan every vertex merely because bone matrices changed.

### 5. Preserve the deformation contract

- Do not reintroduce graphics-stage skinning.
- An unchanged deformation generation must still perform no deformation upload or dispatch.
- Model/camera-only changes must not dirty deformation.
- Shared output lifetime and transfer-to-compute-to-vertex hazards must remain validation-clean.

## Acceptance criteria

The primary performance criterion was accepted on 2026-08-09 from the same-hardware headset
result: VR with mirrors and skinning now runs consistently at 60 FPS instead of below 30 FPS. The
more detailed instrumentation and validation items below remain useful follow-up evidence rather
than blockers for the resolved regression.

- SteamVR environment/aurora does not flicker through during any of the four canonical presets.
- There are no CPU fence waits between mirror captures or XR eyes.
- Both XR eyes reuse one completed deformation generation.
- Mirror work is not duplicated by the per-eye entry point.
- All presets show no per-frame device-buffer or descriptor allocation after warm-up unless
  capacity actually grows.
- The first preset quantifies ordinary continuously changing avatar bones without secondary
  motion or mirror work.
- The reported deltas isolate mirror scheduling, secondary motion, and visualization in that
  order.
- Mirror-off versus mirror-on results identify the mirror cost without changing deformation job
  counts.
- Vulkan synchronization validation reports no transfer/compute/vertex or buffer-retirement
  hazards.
- Draw and instance counts remain correct for the window, mirror, extraction, and both XR eyes.
- Release-build VR performance returns to the pre-cutover range or any remaining difference is
  explained with captured CPU/GPU measurements.

## Hardware validation

Run the four canonical presets on the hardware/runtime where the regression was observed.
Retain the deformation epic's GTX 1080 and GTX 1050 Ti desktop validation, but do not treat
desktop-only results as sufficient evidence for this task.

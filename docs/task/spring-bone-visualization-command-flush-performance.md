# Spring-bone visualization command-flush performance

Status: open measured regression.

Related:

- [Renderer optimisation](epic/renderer_optimisation.md)
- [Compute-cached deformation XR performance regression](compute-cached-deformation-xr-performance-regression.md)
- [Opt-in System, MMS, Vulkano, and XR profiling](opt-in-system-mms-vulkano-xr-profiling.md)

## Problem

Spring-bone visualization makes `vtuber-secondary-motion` extremely slow in XR even after mirror
captures were corrected from six to two per headset frame.

The spring solver and visualization geometry calculation are both inexpensive. The dominant cost
appears when the visualization's per-marker transform intents are executed through the generic
signal and transform machinery.

This is separate from compute-cached deformation:

- deformation jobs, workgroups, dirty vertices, and upload bytes are unchanged between the
  visualization-off and visualization-on cases
- spring simulation itself takes less than `0.1 ms`
- visualization snapshot and transform calculation takes about `0.08 ms`
- the following command flush takes about `66 ms`

## Corrected XR measurements

Measured on 2026-07-27 with:

- release build
- NVIDIA GeForce GTX 1080
- SteamVR/OpenXR 2.12.14
- `1868 × 1868` per-eye render extent
- 4x MSAA
- one mirror
- corrected XR scheduling: two stereoscopic mirror captures and two headset-eye renders
- historical pre-pipelining submission behavior: five Vulkan submissions, four CPU fence waits,
  and one XR queue-idle wait per headset frame

SteamVR selected different effective presentation intervals between runs, so delivered FPS and
frame-submit time include compositor pacing. The scoped application CPU timings and renderer
counters are the primary comparison.

| Preset | FPS | Mean frame | Update before XR | Spring simulation | Spring propagation | Visualization calculation | Post-pose command flush | XR eye render |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `avatar_no_spring_mirror` | 45.439 | 22.007 ms | 5.214 ms | 0.000 ms | 0.000 ms | 0.002 ms | 0.001 ms | 9.567 ms |
| `avatar_spring_no_viz_mirror` | 40.976 | 24.404 ms | 10.297 ms | 0.041 ms | 5.003 ms | 0.022 ms | 0.001 ms | 9.486 ms |
| `avatar_spring_viz_mirror` | 8.389 | 119.204 ms | 77.110 ms | 0.082 ms | 5.311 ms | 0.082 ms | 66.345 ms | 11.958 ms |

Reports:

- [No spring, mirror](../.debug/vr_perf/20260727-190245Z__avatar_no_spring_mirror.md)
- [Spring without visualization, mirror](../.debug/vr_perf/20260727-190432Z__avatar_spring_no_viz_mirror.md)
- [Spring visualization, mirror](../.debug/vr_perf/20260727-190610Z__avatar_spring_viz_mirror.md)

The visualization-on run adds `66.344 ms` to the measured post-pose command flush and
approximately `66.8 ms` to total update time relative to spring without visualization. It does
not increase the per-frame deformation workload:

```text
deformation dispatches       1
deformation jobs            16
deformation workgroups     548
dirty deformation vertices 34,566
bone upload bytes          132,096
job upload bytes             4,896
morph-weight upload bytes        0
```

## Current execution path

`SpringBoneVisualizationSystem` retains marker entities across frames. During each tick it:

1. obtains the bound secondary-motion snapshot
2. reconciles collider, segment, and endpoint markers
3. calculates the current marker transforms
4. emits one `UpdateTransform` intent per collider and two per spring segment

The calculation and intent emission complete in about `0.08 ms`. The intents are then executed by
the post-pose `CommandQueue::flush`:

```text
SpringBoneVisualizationSystem::tick_with_queue
  -> UpdateTransform intent per marker
  -> CommandQueue::flush
  -> SystemWorld::process_signals
  -> RxMutationExecutor
  -> SystemWorld::update_transform
  -> SystemWorld::transform_changed
```

Each generic transform change may propagate a subtree, search for `TransformParent` dependents,
mark affected skinning state, and queue BVH work. Repeating that machinery independently for every
debug marker is the leading scaling explanation, but the individual sub-costs inside the
`66 ms` flush have not yet been measured.

## Measurement required before implementation

Add temporary or opt-in counters for the visualization-on run:

- visible spring chains, colliders, segments, and endpoints
- retained markers and newly spawned or removed markers
- `UpdateTransform` intents emitted by visualization per frame
- total signals processed by the post-pose flush
- time in intent dispatch/mutation, transform propagation, `TransformParent` dependent discovery,
  skinning invalidation, and BVH queueing
- number of component records scanned while resolving transform-parent dependents

This should establish whether the dominant cost is the repeated world scan, repeated subsystem
notifications, signal-envelope overhead, or a combination.

## Implementation direction

Preserve retained visualization markers, but update their transforms through a batched
visualization-owned path.

The preferred first approach is:

1. collect all distinct marker transform writes for the frame
2. apply the local transforms as one batch
3. propagate the independent marker roots without running unrelated rig/skinning invalidation
4. update their `VisualWorld` instances in the same batch
5. notify or refit the BVH once for the completed marker batch, if disabled raycasting still
   requires any BVH work

If a reusable batch-transform facility is introduced into the signal machinery, it must support
different transform values per component and perform shared dependency work once. Merely wrapping
the existing per-marker `UpdateTransform` loop in a new intent will not change the complexity.

A renderer-owned debug primitive buffer is a possible later design if marker entities provide no
required selection, serialization, layout, or scripting semantics. Do not require that larger
redesign for the first fix.

## Non-goals

- Do not rewrite the secondary-motion solver; its measured simulation cost is below `0.1 ms`.
- Do not change compute-cached deformation or reintroduce graphics-stage skinning.
- Do not weaken general transform dependency semantics for ordinary authored components.
- Do not fold the separate approximately `5 ms` spring-root propagation cost into this task.
- Do not undo stereoscopic mirror captures or the once-per-XR-frame mirror scheduling fix.

## Acceptance criteria

- The visualization retains correct collider, segment, and endpoint positions, rotations, scales,
  colors, overlay behavior, and mirror/XR visibility.
- Enabling visualization does not create or remove marker entities every steady-state frame.
- Steady-state visualization does not execute one full generic signal/transform dependency pass
  per marker.
- On the reference workload, visualization adds no more than `2 ms` to update CPU time relative to
  `avatar_spring_no_viz_mirror`, down from approximately `66.8 ms`.
- The post-pose command flush remains below `2 ms` on the reference workload.
- Deformation dispatches, jobs, workgroups, dirty vertices, and upload bytes remain equal between
  visualization-off and visualization-on runs.
- XR still records two mirror captures and two eye renders. Under the Phase 4 submission pipeline,
  steady-state rendering performs no intermediate mirror/eye CPU waits, one final raw-copy fence
  wait before OpenXR image release, and zero queue-idle waits.
- Relevant visualization, transform, signal, BVH, and removal/reconciliation tests pass.
- A before/after pair of release XR reports is linked here after implementation.

## Reproduction

Primary comparison:

```bash
cargo run --release --example vtuber-secondary-motion -- \
  --vr-perf-case avatar_spring_no_viz_mirror

cargo run --release --example vtuber-secondary-motion -- \
  --vr-perf-case avatar_spring_viz_mirror
```

Use separate process launches so marker creation, warm-up, compositor pacing, and secondary-motion
settling do not leak between cases.

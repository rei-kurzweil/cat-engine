# Renderer futures and frame timing

## Status

Draft design. The ownership model described here is not implemented yet.

Related:

- [Vulkano frame-future resource-use regression](../task/vulkano-frame-future-resource-use-regression.md)
- [Compute-cached deformation XR performance regression](../task/compute-cached-deformation-xr-performance-regression.md)
- [Mirror viewer-family captures](../task/mirror-viewer-family-captures.md)

## Problem

The renderer currently asks one future field to cover several different responsibilities:

- order submissions that access renderer-wide shared GPU resources;
- keep a presented swapchain image and its paired attachments alive;
- prevent a per-image depth, MSAA, or post-processing target from being reused too early;
- provide a place to retire Vulkano's resource-use tracking.

Those responsibilities overlap, but they are not identical.

Commit `ef592dc` introduced `submission_future` and changed a successful window submission from
being stored in `images_in_flight[image_i]` to being stored only in `submission_future`. The
per-image cleanup loop remained, but every image slot remained `None`.

The minimal `vulkano-frame-future-regression` case demonstrates the consequence. A built-in cube
with bloom, mirrors, GLTF, skinning, and MSAA disabled fails when a window depth attachment is
used again:

```text
access to a resource has been denied
resource use: begin_rendering / DepthStencilAttachment
error: the resource is already in use, and there is no tracking of concurrent usages
```

The larger avatar scene can report the color attachment instead. Both failures are consistent
with broken frame-attachment lifetime tracking rather than a stencil algorithm or deformation
bug.

## Goals

- Track completion of every successful window submission by swapchain image index.
- Maintain one authoritative ordering chain for submissions that can share GPU resources.
- Retire completed Vulkano future nodes so resource locks and retained allocations remain bounded.
- Keep the CPU and GPU pipelined during normal window rendering.
- Make swapchain recreation and exceptional recovery explicit.
- Give future XR and mirror timing work a clear ownership model.

## Non-goals

- Introduce an unconditional `device.wait_idle()` in steady-state rendering.
- Wait on a CPU fence every time a window image is reused.
- Treat the combined depth/stencil attachment as the root cause merely because validation named
  `DepthStencilAttachment`.
- Redesign all XR and mirror synchronization in the first regression fix.
- Create independent submission branches for work that accesses shared resources.

## Terms

### Submission chain

The latest future in a linear sequence of renderer submissions. Extending this future tells
Vulkano and Vulkan that the next command buffer happens after all earlier work represented by the
chain.

### Image completion

The fence-signalled future produced by the last successful present submission for one swapchain
image index. It owns that frame's use of:

- the swapchain image;
- the depth attachment at the same index;
- the MSAA attachment at the same index, when enabled;
- the post-processing targets at the same index, when enabled;
- per-window-slot buffers selected using that image index.

### Frame slot

A group of resources that may be written together and cannot be overwritten until their previous
GPU consumer has completed. Window frame slots currently use the swapchain image index. XR eyes
and mirror captures have different slot domains and must not be assumed to share window timing.

## Required invariants

### 1. Every shared-resource submission extends one chain

Any submission that reads or writes renderer-wide resources must consume the current
`submission_future` and replace it with a descendant future.

This includes, until proven otherwise:

- deformation-cache dispatch;
- window drawing;
- monoscopic and stereoscopic mirror captures;
- XR-eye drawing;
- runtime-texture production and publication.

This invariant prevents two apparently independent Vulkano future branches from accessing the
same resource without a declared dependency.

### 2. Every successful window submission populates its image slot

After a successful flush for swapchain image `i`:

```text
images_in_flight[i] = completion of this window submission
```

The slot must remain populated until that completion is retired or replaced by a later successful
submission for the same image.

### 3. The ordering and completion references describe the same event

The successful window submission must not create two unrelated futures that merely happen to
refer to the same Vulkan fence. Both fields should refer to the same Vulkano future object.

Vulkano 0.35 supports this shape because `Arc<FenceSignalFuture<F>>` implements `GpuFuture`.
After `then_signal_fence_and_flush()` succeeds, the renderer can wrap the returned fence future in
an `Arc` and box two cloned `Arc` values:

```rust
let completion = Arc::new(future);
submission_future = Some(completion.clone().boxed());
images_in_flight[image_i] = Some(completion.boxed());
```

The boxes are separate trait objects, but their inner `Arc` values share one fence-future state.

### 4. Do not join the same window completion twice

The renderer-wide chain is the authoritative GPU ordering dependency. If it already contains the
previous use of image `i`, the next window submission should extend that chain and join only the
new acquire future:

```text
submission_future
        +
acquire_future(image i)
        |
        v
window command buffer
        |
      present
        |
shared fence completion
```

The per-image clone is a completion/lifetime reference, not a second independent branch to join
back into the chain. Joining both the global clone and an ancestor per-image clone would duplicate
lineage and risks presenting Vulkano with an artificial branch relationship.

This rule depends on invariant 1: no submission may bypass the global chain and later access a
window-slot resource.

### 5. Completed nodes are cleaned incrementally

At the beginning of a render cycle, call `cleanup_finished()` on:

- `submission_future`, when present;
- every populated `images_in_flight` entry.

Cleanup must be non-blocking. Its purpose is to let Vulkano release completed resource-use locks
and old future graph nodes. It is not a substitute for ordering.

Because the window entries and global entry can contain cloned `Arc` references to the same fence
future, cleanup must rely on Vulkano's shared `FenceSignalFuture` state being idempotent. Vulkano's
`signal_finished` contract also permits repeated calls on the same future.

## Proposed window submission flow

For a normal frame:

1. Recreate the swapchain first if requested.
2. Clean completed global and per-image futures without waiting.
3. Acquire the next swapchain image and record `image_i`.
4. Select all window resources from the same `image_i` frame slot.
5. Build mirror work through the shared submission chain, if required.
6. Take `submission_future`, falling back to `sync::now` only at initialization or after an
   explicit reset.
7. Join that future with `acquire_future`.
8. Execute the window command buffer, present, and signal a fence.
9. On success, wrap the fence future in `Arc` and store clones in:
   - `submission_future`;
   - `images_in_flight[image_i]`.
10. On failure, follow the exceptional recovery rules below. Do not leave either field claiming
    ownership of a submission that was never established.

Conceptually:

```text
renderer-wide prior work ───────────────┐
                                       ├─> window submit -> present -> fence
acquire swapchain image i ──────────────┘                         │
                                                                 ├─> global chain
                                                                 └─> image slot i
```

## Why per-image tracking is part of the solution

The image index selects more than the presentable color image. It selects paired engine-owned
attachments and buffers that the swapchain acquire operation does not independently own.

Per-image completion tracking:

- makes that ownership visible;
- keeps the exact frame future alive;
- gives cleanup and swapchain teardown a complete set of window completions;
- provides a place for diagnostics such as submission generation and image handle;
- prevents comments and implementation from disagreeing about whether an image slot is in flight.

Per-image tracking alone is not sufficient if shared-resource submissions are allowed to form
independent branches. The per-image and renderer-wide invariants are therefore complementary.

## Cleanup versus waiting

These operations have different meanings:

| Operation | Meaning | Steady state |
|---|---|---|
| `cleanup_finished()` | Retire work that has already completed | Yes |
| Extend `submission_future` | Establish GPU ordering | Yes |
| Join `acquire_future` | Wait on presentation-engine image availability | Yes |
| `future.wait(None)` | Block the CPU until a fence signals | No for window frames |
| `device.wait_idle()` | Stop until all device work completes | Exceptional recovery only |

A populated image slot does not imply that the CPU should wait on it. Normal ordering stays on the
GPU through futures, queue order, semaphores, and fences.

## Swapchain recreation

Swapchain-dependent targets include swapchain views, depth views, MSAA views, post-processing
targets, and window frame-slot buffers. They must not be destroyed while referenced by an
in-flight future.

The first implementation may retain the existing exceptional recreation sequence:

1. wait for the device to become idle;
2. mark and clean all global and per-image future references;
3. drop old swapchain-dependent resources;
4. recreate the swapchain and targets;
5. resize `images_in_flight` to the new image count, initialized to `None`;
6. reset `submission_future` to `sync::now`.

This wait is acceptable during resize or `OutOfDate`; it must not enter the ordinary frame path.
A later design may replace it with generation-owned retired swapchains.

## Submission and flush failures

Failure handling must distinguish:

- `OutOfDate` before submission;
- `OutOfDate` while flushing/presenting;
- a recoverable Vulkan runtime error;
- a Vulkano validation error.

Validation errors are programming errors and must remain visible to the caller/test harness. They
must not be converted into a log message followed by continued rendering.

If a flush may have partially submitted work and no reliable completion future is returned, the
exceptional fallback is:

1. stop issuing new work;
2. wait for the device or queue to become idle;
3. signal and clean tracked futures;
4. clear all per-image slots;
5. reset the global chain;
6. request swapchain recreation when applicable.

This path is recovery, not normal timing.

## XR and mirror timing

Current XR-eye and mirror-capture paths extend `submission_future`, flush, and then wait on the
CPU. That keeps their resource use linear but serializes CPU and GPU work.

The window regression fix should preserve that behavior initially while making it explicit in
traces. A later timing pass should assign independent completion slots to:

- each XR swapchain image acquired from OpenXR;
- each reusable XR offscreen eye target;
- each mirror target generation or capture slot;
- each runtime-texture publication target.

Removing those CPU waits is safe only after each resource domain has an ownership rule equivalent
to the window image rule.

## Instrumentation

Behind an environment flag, log one record per submission with:

- monotonically increasing submission generation;
- submission kind;
- swapchain image or offscreen slot index;
- raw image handles for color, resolve, depth, and post-processing targets;
- whether the global future existed;
- whether the selected completion slot existed;
- cleanup, replacement, reset, and exceptional wait events.

For a healthy fixed-window run:

- image indices may repeat;
- a repeated index replaces its prior completion reference;
- the global generation increases monotonically;
- completed future graph retention remains bounded;
- no steady-state CPU wait or device-idle event occurs.

## Implementation sequence

### Experiment 1: clean the global chain

Add `submission_future.cleanup_finished()` alongside the existing image cleanup loop. Run minimal
case A.

If this alone fixes the panic, it confirms that stale global future nodes are sufficient to
trigger the immediate failure. It does not restore the documented per-image ownership invariant.

### Experiment 2: shared window completion

Wrap each successful window fence future in `Arc` and store clones in the global field and the
selected image slot. Keep the global chain as the sole ordering dependency.

Run cases A through H and trace image-slot replacement.

### Experiment 3: exceptional paths

Exercise resize, minimize/restore, suboptimal swapchains, and `OutOfDate`. Confirm that old
swapchain targets are not retained indefinitely and that no untracked submitted work survives a
reset.

### Experiment 4: timing cleanup

Measure future retention, queue submissions, CPU fence waits, and queue/device waits. Only after
the ownership traces are correct should XR and mirror CPU waits be reduced.

## Acceptance criteria

- The `vulkano-frame-future-regression` matrix is validation-clean.
- A successful window submission populates exactly its acquired image slot.
- The global and per-image entries for that submission share one fence-future state.
- Global and per-image cleanup occur regularly without blocking.
- Repeated image indices do not reuse depth, MSAA, post-processing, or frame-slot resources
  without an ordering dependency.
- Normal window rendering performs no CPU fence wait and no device/queue idle wait.
- Resize and `OutOfDate` do not retain old swapchain resources.
- Validation failures propagate as failures.
- Submission traces and retained resources remain bounded over at least 2,000 presented frames.

## Open questions

- Does global cleanup alone eliminate the current case A failure?
- Does cloning one `Arc<FenceSignalFuture<_>>` into both fields behave cleanly under repeated
  cleanup in Vulkano 0.35?
- Should `images_in_flight` be renamed to `window_image_completions` to make its role explicit?
- Which renderer-wide resources genuinely require the global chain once every target domain has
  its own completion slots?
- Can swapchain generations eventually retire asynchronously instead of using `device.wait_idle`
  during recreation?

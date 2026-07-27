# Vulkano frame-future resource-use regression

Status: open, reproducible validation panic.

Related:

- [Compute-cached deformation XR performance regression](compute-cached-deformation-xr-performance-regression.md)
- [Mirror viewer-family captures](mirror-viewer-family-captures.md)
- [Opt-in System, MMS, Vulkano, and XR profiling](opt-in-system-mms-vulkano-xr-profiling.md)

## Symptom

`vtuber-desktop` starts, renders its first frames, and then panics while Vulkano submits the
window command buffer:

```text
access to a resource has been denied
resource use: begin_rendering / ColorResolveAttachment
error: the resource is already in use, and there is no tracking of concurrent usages
```

Disabling MSAA does not prevent the panic. It changes only the reported attachment role:

```text
resource use: begin_rendering / ColorAttachment
error: the resource is already in use, and there is no tracking of concurrent usages
```

This is consistent with the same frame target being rejected in both configurations:

- with MSAA it is used as the resolve attachment;
- without MSAA it is used directly as the color attachment.

The avatar-control capsule reaches its settled state before the panic. The validation error names
an image attachment, not a deformation, bone, vertex, or storage buffer.

## Current evidence

The strongest current hypothesis is stale or incomplete future tracking around swapchain-image
reuse, not skinning or intent dispatch.

`VulkanoRenderer::render_visual_world` currently:

1. calls `cleanup_finished()` only on entries in `images_in_flight`;
2. acquires a swapchain image;
3. takes the renderer-wide `submission_future`;
4. joins it with the new acquire future;
5. submits and stores the result back in `submission_future`.

However, successful window submissions are no longer stored in `images_in_flight`. Those slots
remain `None`, so the frame-start cleanup loop has nothing to clean. The code comment still says
that `images_in_flight` prevents reuse of an image that is in flight, but the implementation no
longer establishes that invariant.

The renderer-wide future is shared by window, mirror, XR-eye, and deformation-cache work. It is
needed to order consumers of shared GPU resources, but it must also be retired correctly and must
not substitute accidentally for swapchain-image lifetime tracking.

## Regression boundary

Commit `ef592dc` (`WIP compute shader for skinning and morph targets`) introduced
`submission_future` and changed the successful window path from:

```rust
let image_future = self.images_in_flight[image_i].take();
// submit...
self.images_in_flight[image_i] = Some(future.boxed());
```

to:

```rust
let submission_future = self.submission_future.take();
// submit...
self.submission_future = Some(future.boxed());
```

The existing cleanup loop over `images_in_flight` remained in place.

Commit `069c4a6` (`split up mirror pass in renderer properly`) subsequently separated
monoscopic and stereoscopic mirror-capture scheduling. It changed the shape and frequency of
work entering the shared submission chain, but it did not make the window-future replacement
above.

`vtuber-desktop` does not author a `Mirror` component. Its reproduction therefore does not
require an actual mirror capture, even though the window renderer still calls the mirror-capture
entry point. This makes the future change in `ef592dc` the more direct regression candidate.

## Questions to answer

1. Does the panic begin at `ef592dc`, or was the invalid lifetime already present and merely
   exposed there?
2. Is the denied image the newly acquired swapchain image, a per-frame post-processing image, or
   another window target?
3. Is the previous use retained only because `submission_future.cleanup_finished()` is missing,
   or is the new acquire future being joined concurrently with an older use of the same image?
4. Do we need both:
   - one renderer-wide ordering chain for shared deformation/runtime-texture resources; and
   - one completion future or fence per swapchain image?
5. Which submissions genuinely share resources across window, mirrors, and XR, and which can use
   independent frame slots?

## Verification plan

### 1. Establish the first bad revision

Build and run the same small desktop reproduction at:

- the parent of `ef592dc`;
- `ef592dc`;
- the parent of `069c4a6`;
- `069c4a6`;
- current `HEAD`.

Keep the scene, window size, GPU, and frame count fixed. Record whether it reaches at least 2,000
presented window frames without a validation error.

If practical, automate the revision comparison with a dedicated non-XR example and use
`scripts/compare_render_stream_revisions.sh` or a similarly isolated worktree runner. Do not
judge the result from whether the avatar looks correct; the gate is Vulkan validation and stable
presentation.

### 2. Reduce the scene independently of renderer revisions

Use a matrix that changes one feature at a time:

| Case | Geometry | Skinning | Bloom | Mirror | MSAA |
|---|---|---:|---:|---:|---:|
| A | one cube | off | off | off | off |
| B | one cube | off | off | off | 4x |
| C | one cube | off | on | off | off |
| D | one cube | off | on | off | 4x |
| E | static GLTF | off | off | off | off |
| F | avatar GLTF | on | off | off | off |
| G | avatar GLTF | on | on | off | off |
| H | avatar GLTF | on | on | on | 4x |

If A fails, skinning, bloom, mirrors, and MSAA are all excluded. If only F and later fail, inspect
deformation resources and submission ordering again. If mirror-free G fails, mirror capture
deduplication is not required to reproduce the panic.

### 3. Add temporary submission tracing

For every submission, log a monotonically increasing submission generation and:

- submission kind: window, monoscopic mirror, stereoscopic mirror, XR eye, or runtime texture;
- acquired swapchain image index, where applicable;
- raw Vulkan image handles for color, resolve, depth, and post-processing targets;
- whether the matching `images_in_flight[image_i]` slot is populated;
- whether `submission_future` exists;
- each cleanup, wait, replacement, and reset of either tracking path.

The trace should make the last previous use of the denied image identifiable. Keep it behind an
environment flag so normal rendering is not noisy.

Run with a full Rust backtrace and Vulkan validation enabled:

```bash
RUST_BACKTRACE=full cargo run --example vtuber-desktop
```

Capture the final acquired image indices and image handles before the panic.

### 4. Run controlled proof patches

These are diagnostic experiments, not automatically acceptable final fixes.

#### Experiment A: retire the global future

Call `cleanup_finished()` on `submission_future` at frame start in addition to the per-image
slots. If the panic disappears across the full matrix, stale nodes in the global future graph
are implicated.

#### Experiment B: force completion

Temporarily wait for the device or previous submission to become idle before every window frame,
then reset tracking to `sync::now`.

If this eliminates the panic, it proves a lifetime/order problem. It is not an acceptable
shipping fix because it serializes the CPU and GPU.

#### Experiment C: restore per-image ownership

Restore the pre-`ef592dc` per-swapchain-image future path for window presentation while retaining
an explicitly designed renderer-wide dependency for shared deformation work.

If this eliminates the panic, compare its trace with Experiment A to determine whether global
cleanup alone is sufficient or whether per-image ownership is required.

#### Experiment D: remove the global join only in an isolated build

Use a fresh `sync::now` future for a minimal, non-skinned, non-mirror desktop scene.

If the minimal scene becomes validation-clean, the invalid dependency is inside the accumulated
global chain. Do not apply this to shared deformation resources as a fix; it intentionally
removes their ordering protection.

### 5. Verify the final ownership model

The final design should document separate responsibilities:

```text
swapchain image index
  -> completion tracking before that image is reused

renderer-wide shared resources
  -> ordered dependency between deformation, mirrors, XR eyes, and window consumers

acquire future
  -> availability of this presentation image

present future
  -> completion and retirement of this image's frame
```

Avoid one future field implicitly serving all four roles unless its cleanup and resource-usage
semantics are demonstrated with traces and tests.

## Fix requirements

- Restore real swapchain-image reuse protection.
- Retire completed nodes from the renderer-wide future chain.
- Preserve ordering for shared deformation and runtime-texture resources.
- Do not use per-frame `device.wait_idle()` or unconditional CPU fence waits as the final fix.
- Keep monoscopic mirror captures out of the XR-eye loop and stereoscopic captures out of the
  window loop.
- Handle swapchain recreation and `OutOfDate` without retaining resources from the old swapchain.
- Replace the internal Vulkano `unwrap()` panic with an engine-visible error where the API permits,
  while still treating validation failures as test failures.

## Acceptance criteria

- At least 2,000 validation-clean presented frames for every applicable desktop matrix case.
- `vtuber-desktop` is validation-clean with MSAA both enabled and disabled.
- Bloom enabled and disabled are both validation-clean.
- A minimal cube scene proves the result does not depend on skinning.
- A skinned avatar proves deformation ordering remains correct.
- Mirror-free and mirror-enabled scenes are both validation-clean.
- OpenXR renders both eyes and the required stereoscopic mirror captures without redundant
  monoscopic captures.
- Submission traces show bounded future/resource retention rather than a chain that grows for the
  lifetime of the process.
- No unconditional queue/device idle or per-consumer CPU fence wait is introduced into the
  steady-state window or XR path.

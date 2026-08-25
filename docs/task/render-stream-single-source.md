# Render streams as the single source for clip-capable phases

Status: complete.

Epic: [Renderer optimisation](epic/renderer_optimisation.md)

## Problem

`VisualWorld` currently builds both a flat draw-order/batch cache and a render stream for each
clip-capable main phase:

- opaque: `draw_order` / `draw_batches` and `opaque_stream`
- cutout: `cutout_order` / `cutout_batches` and `cutout_stream`
- single-layer transparent: `transparent_single_draw_order` /
  `transparent_single_draw_batches` and `transparent_single_stream`
- overlay: `overlay_order` / `overlay_batches` and `overlay_stream`

The main renderer always consumes the streams, even when there are no stencil clips. Without
clips a stream already degenerates to `RenderOp::DrawBatch` entries, so the corresponding flat
batch vectors duplicate cache construction and storage without serving the main color passes.

Command-buffer construction also calls `to_vec()` on the cached opaque, cutout, and overlay
stream operations and instance indices for ordinary views. This allocates and copies the streams
per render view even though they can be borrowed directly. Mirror rendering needs owned filtered
streams only when an instance is actually excluded.

Current 64-bit type sizes provide a useful cost model:

- `VisualInstance`: 228 bytes
- `DrawBatch`: 40 bytes
- `RenderOp`: 40 bytes
- stream instance index: 4 bytes

## Goal

Make render streams the only cached representation for opaque, cutout, single-layer transparent,
and overlay drawing. Borrow cached streams on the common path and construct owned filtered streams
only for views that exclude an instance.

`DrawBatch` remains the batching unit inside `RenderOp::DrawBatch`; this task removes duplicate
flat caches, not batching.

## Implementation

### Consolidate `VisualWorld` caches

- Remove `draw_batches`, `cutout_batches`, `transparent_single_draw_batches`, and
  `overlay_batches`, together with their accessors and cache rebuild calls.
- Keep each phase's sorted order as the input to stream construction and for any existing
  phase-membership or exclusion work.
- Keep `opaque_stream`, `cutout_stream`, `transparent_single_stream`, and `overlay_stream` as the
  authoritative draw caches.
- Preserve `build_draw_batches_for_order` for passes that still use flat batches.

### Add a no-clip stream fast path

- Determine whether any live `VisualInstance` has `is_stencil_clip` once during dirty draw-cache
  preparation. Reuse that result for every phase instead of rediscovering clip presence inside
  each stream build.
- When no stencil clip exists, build each stream directly from its already-sorted phase order:
  append only `RenderOp::DrawBatch` entries, use an effective stencil reference of zero, and copy
  the phase indices directly into the stream instance array.
- On this path, do not calculate maximum stencil depth, scan all instances for clip sources,
  allocate per-depth groups, or recurse through clip levels.
- Keep the existing depth-grouped DFS construction only for worlds that contain stencil clips.
- Treat the clip-presence result as draw-cache state, not a separately maintained public flag:
  registration, removal, or mutation of `is_stencil_clip` already dirties the draw cache and must
  cause it to be recomputed.

### Preserve specialized flat-batch passes

Do not convert these as part of this task:

- background and background-occluded-lit passes
- emissive and emissive-cutout bloom extraction
- background-occluded-lit emissive extraction
- per-view multi-layer transparency

These caches represent distinct pass subsets or view-dependent ordering and are not duplicates of
the four main phase streams.

### Stop cloning ordinary-view streams

- In command-buffer construction, represent each selected stream as borrowed cached slices or as
  owned filtered vectors.
- Borrow the cached opaque, cutout, and overlay streams when `excluded_instance` is `None`.
- Call the existing `*_stream_excluding` builders only when an exclusion is present, retaining
  their owned results until command recording completes.
- Apply the same borrowed-versus-owned convention to single-layer transparency so mirror
  exclusion behavior remains consistent.
- Do not introduce `Arc`, reference-counted snapshots, or stored references into `VisualWorld`.
  The canonical `Vec<VisualInstance>` plus `u32` indices remains the ownership model.

## Correctness constraints

- An unclipped phase must produce the same batch boundaries, instance order, pipelines, and draw
  counts as before.
- Nested clips must retain the exact `EnterClip`, clipped `DrawBatch`, and reverse-order
  `ExitClip` sequence.
- A clip source must still be available for stencil operations in every affected phase even when
  its color draw belongs to a different phase.
- Mirror exclusion must remove the excluded color draw and any stencil operations sourced by that
  instance without changing ordinary views.
- Clip-source visual draws must continue using the incremented effective stencil reference.
- The no-clip fast path must produce the same batch boundaries and stream instance ordering as the
  general stream builder would produce for an unclipped world.

## Test plan

- Retain and run the existing opaque, cutout, transparent-single, overlay, nested-clip, cross-phase
  clip-source, and exclusion stream tests.
- Add a no-clip regression test proving that each stream contains only `DrawBatch` operations and
  covers every phase-order instance exactly once.
- Add focused coverage proving that no-clip construction bypasses depth discovery and per-depth
  grouping; keep the fast-path decision in a small testable helper or expose test-only
  instrumentation rather than relying on timing.
- Test the transition from no clips to at least one clip and back after removal, verifying that
  dirty-cache preparation selects the correct construction path each time.
- Add or extend renderer-facing tests to cover borrowed ordinary streams and owned exclusion
  streams.
- Run the graphics/`VisualWorld` test suite and `cargo check`.
- Compare a representative scene before and after using draw counts and screenshots for window,
  XR, and mirror views.

## Completion criteria

- The four redundant flat `Vec<DrawBatch>` caches and their rebuild work no longer exist.
- Worlds without stencil clips do not perform maximum-depth discovery, full-instance clip-source
  scans, per-depth temporary allocation, or DFS recursion while building phase streams.
- Ordinary window and XR views do not allocate or copy cached streams during command-buffer
  construction.
- Owned stream construction occurs only for an actual instance exclusion.
- Existing stencil clipping, phase ordering, mirror exclusion, and rendered output remain
  unchanged.
- Profiling records the before/after command-buffer preparation time and allocation count for at
  least one unclipped scene and one clipped scene.

## Implementation record

Implemented on 2026-07-24:

- The opaque, cutout, single-layer transparent, and overlay flat batch caches, accessors, and
  rebuild calls were removed. Their sorted phase orders now feed only the authoritative streams.
- Dirty-cache preparation computes clip presence once and passes it to all four stream builders.
  The no-clip path appends batches directly from the sorted order with stencil reference zero,
  without depth discovery, clip-source grouping, or DFS.
- Ordinary window and XR views borrow opaque, cutout, and overlay stream slices. Owned filtered
  streams are constructed only when a mirror view supplies an excluded instance. Single-layer
  transparency follows the same borrowed/owned convention.
- Multi-layer transparency remains a specialized per-view flat cache. Its redundant second
  per-eye rebuild during command recording was removed, preserving the render-view-specific sort
  prepared before stream borrowing (including mirror views).
- Added regression coverage for all four no-clip phase streams and for transitions from no clips,
  to an active clip, and back.

Automated validation:

- `cargo check`: passed.
- `cargo test visual_world --lib`: 12 passed.
- `cargo test engine::graphics --lib`: 19 passed.
- Existing nested-clip, cross-phase clip-source, and mirror-exclusion stream tests pass.

Window smoke validation on 2026-07-24:

- `clip-shape`: visually confirmed working.
- `scrolling`: launched through first window render and remained in its present loop; use this as
  the lightweight two-clip UI smoke scene.
- `ui-layout`: was not a valid renderer smoke result because its MMS source had an unterminated
  outer transform block. The source was repaired and its wrapper now treats MMS errors as fatal
  instead of continuing into an empty scene. It remains a slow editor-integration target because
  its `ED` subtree materializes roughly 12,000 runtime components before window creation.
- `vtuber-mirror-example`: mirror and XR rendering were visually confirmed working.

Revision-comparison measurement on 2026-07-25:

- Probe: `tests/renderer_optimization_profile.rs`, run by
  `examples/scripts/compare_render_stream_revisions.sh` in separate detached worktrees and separate Cargo
  target directories.
- Revisions: `65ce0cb` before the refactor and `89d1526` after it.
- Workload: release build, 2,048 synthetic instances distributed evenly across opaque, cutout,
  single-layer transparent, and overlay phases; 25 independently constructed first dirty-cache
  builds per scenario. Hardware was an Intel Core i5-10400 on Linux 7.1.3.
- Confirmation-run artifact:
  `docs/.debug/render-stream-comparison-20260725T041253Z.txt`.
- Unclipped median cache-build time fell from 69,582 ns to 34,809 ns (50.0%); allocator calls fell
  from 116 to 68 (41.4%); requested allocation bytes fell from 50,528 to 33,280 (34.1%).
- Clipped median cache-build time changed from 70,186 ns to 60,392 ns; allocator calls from 127 to
  123; requested allocation bytes from 71,392 to 70,752. The smaller difference is expected because
  clipped worlds retain depth grouping and DFS construction.
- Generated output stayed structurally equal: the unclipped scenario produced 4 operations and
  2,048 stream instances in both revisions; the clipped scenario produced 13 operations and 2,052
  stream instances in both revisions.
- For one ordinary view, the baseline copied 6,264 bytes of opaque/cutout/overlay stream payload in
  the unclipped scenario and 6,556 bytes in the clipped scenario, using six vector allocations.
  Current code borrows those cached slices, so both copied bytes and copy allocations are zero.

Final visual validation on 2026-07-25:

- Representative unclipped window rendering and the `scrolling` scene were visually confirmed.
- Together with the clipped, mirror, and XR smoke results above, the Phase 1 validation gate is
  complete.

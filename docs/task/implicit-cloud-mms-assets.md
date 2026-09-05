# Task: deterministic implicit cloud MMS assets

Status: active — the compact cloud prefab is implemented; the reusable sky
arrangement and its first desktop consumer are being added.

## Outcome

Provide reusable MMS cloud assets that bake each cloud from intersecting
`ImplicitSphere` puffs into one `ImplicitSurface` mesh, then arrange those
cloud meshes in a background sky layer.

Target assets:

- `assets/components/backgrounds/cloud.mms` exports one cloud generator;
- `assets/components/backgrounds/clouds.mms` exports a sky/background helper
  that attaches generated clouds below `BG.occlusion_and_lighting()`.
- `examples/e2.mms` is the desktop still-life acceptance scene: Bisket in a
  first-person desktop rig, mirror, estradiol tablet, and a simple room.

## Authoring contract

The cloud generator needs a deterministic seed and a bounded detail control:

- `seed` selects stable puff placement and radii;
- `depth` or `max_puffs` limits recursive/detail expansion and therefore bake
  cost;
- every child puff must intersect its parent or another already accepted puff;
- puff radii decrease with depth, with a positive minimum radius;
- the resulting `ImplicitSurface` has explicit padded bounds, voxel size,
  smooth-min radius, color, and one baked mesh output.

The helper must not publish individual visible sphere meshes. `ImplicitSphere`
components are field controls only; the baked implicit mesh is the cloud.

## Required implementation slices

- [x] Establish an MMS-safe deterministic pseudo-random helper. Reuse a stable
      math/noise primitive only if it guarantees repeatable results from seed
      and integer-like indices.
- [x] Establish a bounded expansion expression. If recursive MMS functions are
      unavailable or unsuitable, use an explicitly bounded iterative tree with
      the same `depth`/`max_puffs` observable contract.
- [x] Add `cloud.mms` with a small fixture cloud and validation that all puff
      centers/radii preserve intersection connectivity and stay inside padded
      sample bounds.
- [x] Add `clouds.mms` for a deterministic multi-cloud sky arrangement under a
      background component. Its initial public surface is deliberately small:
      one shared `color`, `puff_count`, `max_puff_size`, and
      `puff_clustering` (angular jitter away from even ring spacing).
- [x] Add `examples/e2.mms` as the concrete desktop acceptance scene. It keeps
      XR head/eye/hand tracking out of scope, preserves Bisket's desktop
      secondary motion and colliders, and uses a head-attached monoscopic
      camera plus a mirror.
- [ ] Replace the temporary cube room's window openings with a layout-authored
      wall factory. A window wall should have a short lower wall, an open gap,
      and a short upper wall; keep this out of the first scene so layout
      behavior can be designed and tested independently.
- [ ] Add MMS materialization tests plus an extraction test proving each cloud
      creates one nonempty baked mesh and no boundary-field error.
- [ ] Tune LOD/bake budgets and document safe defaults; avoid unbounded puffs
      or sampling grids.

## Existing seams and non-goals

`examples/example_util/mod.rs` contains the Rust-only `spawn_cloud_ring`
reference. It creates independent cube puffs, so it is useful for distribution
ideas but is not the output contract here. `ImplicitSurface` already provides
bounded marching-cubes extraction and smooth sphere unions.

This task does not add volumetric rendering, runtime per-frame remeshing,
weather simulation, or rough transmission. The immediate refraction study uses
the existing star background until the cloud assets are complete.

## Acceptance

A scene can import `clouds.mms`, create a stable sky from the same seed, and
show each cloud as one connected-looking baked implicit mesh. Raising the
detail limit adds smaller intersecting puffs without violating the configured
bake budget or sampling bounds.

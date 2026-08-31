# Task: deterministic implicit cloud MMS assets

Status: tracked; not started.

## Outcome

Provide reusable MMS cloud assets that bake each cloud from intersecting
`ImplicitSphere` puffs into one `ImplicitSurface` mesh, then arrange those
cloud meshes in a background sky layer.

Target assets:

- `assets/components/backgrounds/cloud.mms` exports one cloud generator;
- `assets/components/backgrounds/clouds.mms` exports a sky/background helper
  that attaches generated clouds below `BG.occlusion_and_lighting()`.

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

- [ ] Establish an MMS-safe deterministic pseudo-random helper. Reuse a stable
      math/noise primitive only if it guarantees repeatable results from seed
      and integer-like indices.
- [ ] Establish a bounded expansion expression. If recursive MMS functions are
      unavailable or unsuitable, use an explicitly bounded iterative tree with
      the same `depth`/`max_puffs` observable contract.
- [ ] Add `cloud.mms` with a small fixture cloud and validation that all puff
      centers/radii preserve intersection connectivity and stay inside padded
      sample bounds.
- [ ] Add `clouds.mms` for a deterministic multi-cloud sky arrangement under a
      background component, with controls for count, spread, height, seed, and
      detail limit.
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

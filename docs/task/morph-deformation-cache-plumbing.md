# Morph deformation cache plumbing

Status: planned; focused first slice of morph-target support.

Epic: [GPU-cached deformation and morph targets](epic/gpu-cached-deformation-and-morph-targets.md)

Parent task: [Morph targets and editor panel](morph-targets-and-editor-panel.md)

Depends on: [Compute-cached mesh deformation](compute-cached-mesh-deformation.md)

Example control follow-up:
[Manual animation keyframe stepping and XR slide controls](manual-animation-keyframe-stepping-and-xr-slide-controls.md)

## Purpose

Exercise real morph-target GPU data, per-instance blend-factor updates, and the shared deformation
cache before adding glTF morph import or editor controls. Use identity morph targets so the new
path can be validated without requiring a visibly morphing production asset.

The first acceptance question is deliberately narrow: can a morph-capable skinned character keep
the existing deformation-cache behavior and VR performance when its morphs are inactive, while a
synthetic active morph exercises the real GPU path without changing the rendered result?

## Terminology

Avoid the unqualified word `weight` in new engine documentation and APIs because glTF uses it for
two different deformation inputs.

### Morph delta

A morph target is primarily a large immutable array of per-vertex differences from the base mesh.
Those differences are morph deltas:

```text
target "smile":
    vertex 0: position delta, normal delta
    vertex 1: position delta, normal delta
    ...
```

After import normalization, every target has one delta record per vertex. A source sparse accessor
becomes the same dense array with zero deltas for unaffected vertices.

Morph deltas define what the target does. They belong to the mesh, are shared by all instances,
and are uploaded once to an immutable device-local buffer. In the current GPU contract each
`GpuMorphDelta` is 32 bytes, so this is the large part of morph storage:

```text
morph delta bytes = target count * vertex count * 32
```

For example, 52 targets on a 10,000-vertex primitive occupy about 16.6 MB.

### Morph blend factor

A morph blend factor says how strongly one instance applies one target. glTF and common DCC tools
call this a morph weight, but `morph_blend_factor` or the qualified `morph_weight` is clearer in
engine code.

```text
morphed_position =
    base_position
    + smile_blend_factor * smile_position_delta
    + blink_blend_factor * blink_position_delta
```

A factor of `0` disables the target, `0.5` applies half of its deltas, and `1` applies the authored
target fully. Multiple targets may contribute simultaneously. Runtime factors do not need to sum
to one and may legally be negative or greater than one, although the first editor slider may expose
the common `[0, 1]` range.

Morph blend factors are small mutable per-instance state. A 52-target primitive needs 52 host
floats, plus compact GPU records for currently active targets.

### Skin joint weight

A skin joint weight is different: it is immutable per-vertex data describing how strongly each of
that vertex's joint matrices contributes during skinning. New names should say `joint_weights` or
`skin_joint_weights`, never merely `weights`.

| Input | Ownership | Size pattern | Meaning |
|---|---|---|---|
| Morph delta | mesh-shared, immutable | vertices times targets | definition of a target |
| Morph blend factor | instance-owned, mutable | one scalar per target | how much to apply it |
| Skin joint weight | mesh-shared, immutable | four values per vertex | how bones affect a vertex |
| Bone matrix | instance-owned, mutable | one matrix per joint | current skeleton pose |

## Unified deformation-cache model

Keep one final processed-vertex cache rather than separate skin and morph output caches:

```text
immutable mesh inputs                 mutable instance inputs
---------------------                 -----------------------
base positions/normals                optional bone matrices
optional skin attributes              optional morph blend factors
optional dense morph deltas
              \                         /
               morph, then skin in compute
                            |
              persistent mesh-local output range
                            |
          window, mirrors, extraction, and XR eyes
```

Every dirty job recomputes from immutable base vertices. Do not incrementally add to the previous
cached result; doing so would accumulate error and make disabling or reducing a target difficult.

Cache eligibility is semantic:

```text
needs_deformation = has_skin || has_morph_targets
```

The four combinations are:

| Skin | Morph targets | Path |
|---|---|---|
| no | no | existing static graphics path |
| yes | no | compute skinning |
| yes | yes | compute morph accumulation, then skinning |
| no | yes | compute morph accumulation with identity skinning |

Morph-only meshes are not expected to dominate cartoon-character or VTuber content, but the data
model should support them. The compute shader should read skin attributes only when skinning is
active rather than allocating fake joint data for morph-only vertices.

## GPU ownership and addressing

Maintain two distinct morph resources:

1. A renderer-global immutable morph-delta arena. Each GPU mesh records its target-major base,
   vertex count, and target count.
2. A persistent per-instance active-morph palette. Each visual instance records a stable palette
   base and an active count.

Keep dense morph blend factors on the host for direct stable indexing. Reserve `target_count`
`GpuActiveMorph` slots per instance and pack nonzero targets into the prefix. At 8 bytes per slot,
52 reserved targets cost 416 bytes per instance.

Each active record contains the resolved immutable delta base and blend factor:

```text
delta_base = mesh.morph_base + target_index * mesh.vertex_count
GpuActiveMorph { delta_base, morph_blend_factor }
```

Stable per-instance palette ranges mean:

- bone-only changes do not upload unchanged morph blend factors;
- a blend-factor change repacks and uploads only that instance's palette range;
- targets crossing zero change the active count without reallocating;
- zero active targets cause no morph-delta reads in compute;
- instances share immutable deltas but retain independent blend factors.

One unified dirty condition schedules the final cache update:

```text
deformation_dirty =
    bones_changed
    || morph_blend_factors_changed
    || mesh_inputs_changed
    || output_allocation_changed
```

Model, camera, view, material, and viewport changes do not dirty deformation.

## Identity-morph probe

Create a synthetic skinned test mesh with one or more morph targets whose position and normal
deltas are all zero. This exercises real morph buffers and active records while producing the same
cached vertices as skinning alone.

Run these cases:

1. Existing skinning with no morph metadata.
2. The same mesh with 32 to 64 registered targets and every blend factor zero.
3. One zero-delta target active with blend factor `1`.
4. Change that factor once while the skeleton is stationary.
5. Leave the factor unchanged while the skeleton continues animating.
6. Leave both skeleton and factors unchanged.

Expected work:

| Case | Morph-delta reads | Morph upload | Deformation dispatch |
|---|---:|---:|---:|
| skin only | none | none | on bone changes |
| morph-capable, all factors zero | none | initial state only | on bone changes |
| one identity target active | one target per vertex | when activated | on bone changes |
| factor-only change | active targets | affected instance only | once |
| unchanged factor, changing bones | active targets | none | on bone changes |
| all inputs unchanged | none | none | none |

## Validation example

Plan a dedicated example pair derived from the existing `vtuber-mirror-example`:

```text
examples/vtuber-morph-targets.rs
examples/vtuber-morph-targets.mms
```

Preserve the representative workload that made cached skinning valuable:

- a tracked and animated skinned VTuber avatar;
- XR left and right eye rendering;
- at least one mirror observing the avatar;
- the existing shared deformation cache and performance counters;
- no dependency on a production model that already contains visible morph targets.

Until glTF morph import exists, the Rust wrapper may construct or inject synthetic zero-delta morph
targets into the test mesh. The MMS scene owns the environment, avatar, mirror, status text, and
operator controls. Once import exists, keep the synthetic mode as the stable identity oracle and
add a real-model mode separately.

Provide separate command-line or startup presets for reliable measurements:

1. skin-only baseline;
2. morph-capable with 32 to 64 identity targets and all blend factors zero;
3. one identity target active;
4. a factor-change probe with bones held still where practical.

Separate startup presets are authoritative for before/after performance comparisons because they
avoid one mode warming or allocating resources for another. An optional in-world manual deck may
cycle through explanatory and runtime states for interactive validation and recording. That deck
should use the planned
[manual animation stepping API](manual-animation-keyframe-stepping-and-xr-slide-controls.md):
`ButtonB` advances, `ButtonA` goes back, and each slide explicitly updates its complete text,
transform, color, font size, and morph-probe state.

The example should display or log at least:

- current probe mode;
- active morph count;
- morph-blend-factor upload bytes;
- deformation dispatches, jobs, and dirty vertices;
- mirror and XR consumer counts;
- observed FPS/frame timing.

The example is a human-operated validation and video-production surface, not a replacement for
automated output, dirty-range, and unchanged-frame tests.

## Implementation slices

1. Rename new contracts to distinguish skin joint weights from morph blend factors. Existing glTF
   API names such as `read_weights(0)` may remain at their boundary with an explanatory comment.
2. Generalize visual instances and output allocation from skinned-only to deformable.
3. Add optional normalized morph data to the CPU mesh contract and upload it into one immutable
   device-local arena.
4. Add stable per-instance active-morph palette ranges and dirty-range uploads.
5. Populate real `active_morph_base` and `active_morph_count` job fields.
6. Make skin input optional for morph-only jobs while preserving current skinned behavior.
7. Add the synthetic identity-morph fixture, output comparison, dirty-work tests, and counters.
8. Add the `vtuber-morph-targets.rs` / `.mms` validation example and its isolated startup presets.
9. Repeat the representative VR-with-mirrors workload for skin-only, zero-active morph-capable,
   and one-active-identity-target cases.

glTF morph import, target naming, editor controls, animation channels, VRM expressions, and visible
production expression demonstrations remain in the parent task or later work.

## Acceptance criteria

- Merely registering morph targets with all blend factors zero does not reduce the currently
  observed consistent 60 FPS in the representative VR mirrors-and-skinning workload.
- Zero active targets cause no morph-delta reads and no per-frame morph uploads.
- A bone-only change does not upload morph state.
- A blend-factor change dirties and dispatches only the affected instance.
- Unchanged bones and blend factors produce no deformation upload or dispatch.
- An active identity target exercises the real morph loop and produces cached output equal to the
  skin-only result within the packed-normal contract.
- Skin-only, skin-plus-morph, morph-only, and fully static cache eligibility is covered.
- Every graphics consumer continues to reuse one completed deformation generation.

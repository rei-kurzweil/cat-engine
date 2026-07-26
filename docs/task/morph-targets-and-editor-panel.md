# Morph targets and editor panel

Status: planned; begins after compute-cached deformation.

Epic: [GPU-cached deformation and morph targets](epic/gpu-cached-deformation-and-morph-targets.md)

Depends on: [Compute-cached mesh deformation](compute-cached-mesh-deformation.md)

## Problem

The engine does not import and evaluate glTF morph targets or expose their weights for live editor
preview. The previous [blend-shape draft](../spec/blend-shapes.md) proposed multiple graphics
pipelines and runtime dense/sparse storage selection, which would repeat deformation work across
passes and views.

Morph targets should instead extend the shared compute deformation pass. Import normalizes source
accessors once; runtime work is dirty only when an affected instance's weights or other
deformation inputs change.

## Scope

Version 1 supports:

- glTF `POSITION` target deltas
- optional glTF `NORMAL` target deltas
- imported mesh and node default weights
- dense immutable target-major CPU and GPU data
- dense host weights and compact nonzero compute entries
- runtime weight updates with stable primitive/target identity
- a selected-glTF editor panel for live preview

Out of scope:

- tangent target deltas
- morph animation channels
- VRM expression presets
- retargeting
- LOD
- saved/persistent preview weights or poses
- disk caching

## Import and normalized representation

Use the glTF reader's existing accessor decoding and sparse-accessor expansion. Sparse and dense
source accessors must normalize to the same representation:

```text
target 0: [vertex 0 delta, vertex 1 delta, ...]
target 1: [vertex 0 delta, vertex 1 delta, ...]
...
```

For every primitive:

- validate each target's position and optional normal count against the primitive vertex count
- reject malformed data clearly before creating partial runtime or GPU state
- retain immutable normalized arrays in the existing URI/mesh CPU cache
- preserve stable primitive and target identity across CPU, GPU, runtime component, and editor
  updates
- retain imported mesh/node default weights

Dense target-major arrays are the only v1 storage format. Do not add an analyzer, CSR storage,
authoring overrides, or separate dense/sparse pipelines.

Upload immutable target arrays to device-local morph buffers. A content/version-keyed disk cache
for normalized data is an explicit later optimization, not part of v1.

## Runtime deformation

Maintain dense host weights for simple indexing and updates. For a dirty deformation job, build a
compact list of nonzero:

```text
(target_index, weight)
```

Upload only changed weight data and dirty only deformation ranges belonging to affected
renderables. Do not scan or dispatch unrelated instances.

Apply target deltas in bind-pose space before skinning:

```text
morphed_position = base_position + sum(weight * position_delta)
morphed_normal   = base_normal   + sum(weight * normal_delta)
deformed_output = skin(morphed_position, morphed_normal)
```

Position-only targets leave the base normal unchanged before skinning. Define and test the normal
normalization point used by the cached graphics path.

Add runtime morph components and weight-update intents that carry stable primitive/target
identity. Preview weight changes are runtime state and reset to imported defaults after asset or
scene reload.

## Selection and editor panel

Add `EditorPanel::MorphTargets` together with its panel kind, asset, registration, and selection
synchronization.

Resolve the edited glTF instance from:

1. the selected `GLTFComponent`, or
2. the nearest glTF ancestor of the selected component.

Clear or refresh rows when selection changes, the selected instance disappears, its asset reloads,
or target metadata changes.

### Row identity and labels

- Group equal-name targets across primitives into one row when their current weights agree.
- An update from a grouped row emits one intent covering every member.
- Unnamed targets use a stable primitive-qualified fallback label.
- If equal-name members currently have different weights, split them into primitive-qualified
  rows for v1.
- Mixed-value grouped presentation is a v2 follow-up.

Labels and intent payloads must not rely on row order, because primitives and targets need stable
identity across panel refreshes.

### Slider behavior

Each scrollable row contains:

- label
- numeric value
- track
- `Draggable` thumb

On drag start, capture the thumb's local X and current weight. During drag:

1. convert movement into track-local X
2. calculate from the captured values
3. clamp thumb position and weight to `[0, 1]`
4. update the numeric display and thumb
5. emit one grouped weight update

Do not depend on event-handler ordering by rereading a transform that `DraggableSystem` may already
have mutated.

## Test plan

### Import

- Dense and sparse accessors normalize to identical arrays.
- Position-only and position-plus-normal targets import.
- Mesh and node defaults resolve correctly.
- Target or attribute count mismatch fails clearly without partial GPU state.
- Reusing the same URI/mesh uses the existing normalized CPU cache.

### Deformation

- One target at zero and nonzero weights matches a CPU reference.
- Multiple active targets sum correctly.
- Morph-before-skin ordering matches a CPU reference and differs from an intentionally reversed
  reference.
- Position-only targets preserve the expected normal input.
- Updating one weight dirties only affected deformation ranges and uploads only changed weight
  data.
- Unchanged weights produce no deformation work.

### Runtime and editor

- Named targets group across primitives when values agree.
- Unnamed fallback names are stable and primitive-qualified.
- Duplicate-name targets split when current values differ.
- Imported default weights appear initially and return after reload.
- Selection resolves directly and through the nearest glTF ancestor.
- Selection changes and removal clear stale rows and update targets.
- Slider drag uses captured local state, clamps position/value to `[0, 1]`, and emits one grouped
  update.
- Panel content scrolls when targets exceed available space.

### End-to-end

Use a real model with expression targets such as happy, sad, and angry. Confirm live updates from
the panel in desktop rendering on GTX 1050 Ti Mobile and desktop/XR rendering on GTX 1080.

## Completion criteria

- glTF morph targets and defaults import into one validated dense representation.
- Morph buffers are immutable and device-local; weight updates are compact and change-driven.
- Morphs run before skinning inside the shared cached deformation pass.
- Runtime updates preserve stable primitive/target identity.
- The selected-glTF editor panel implements naming, grouping, dragging, selection, and reload
  behavior.
- Automated reference, malformed-input, dirty-work, and panel tests pass.
- Hardware and visual validation required by the epic is recorded.

## Future morph poses

Reserve a future serializable `MorphPoseComponent` concept for named presets containing
`(target key, value)` entries and a pose-like `apply(gltf)` API. A morph pose is a collection of
target weights, not a raw target, and is not part of this task.


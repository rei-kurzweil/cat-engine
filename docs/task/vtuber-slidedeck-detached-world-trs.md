# VTuber slide-deck detached world-TRS implementation

Status: in progress; opaque MMS TRS pass-through and local `trs()` round trip implemented, world
mutation and binding remain.

Related:

- [VTuber slide-deck XR placement and controls](vtuber-slidedeck-xr-placement-and-controls.md)
- [Transform component accessors](../../crates/meow-meow-script/docs/draft/transform-component-accessors.md)
- [Mittens engine transform accessor API](../draft/transform-component-accessors-engine-api.md)
- [Manual animation keyframe stepping and XR slide controls](manual-animation-keyframe-stepping-and-xr-slide-controls.md)
- [Legacy transform-pipeline and command-queue cleanup](legacy-transform-pipeline-and-command-queue-cleanup.md)

## Goal

Make the detached slide placement in `examples/vtuber-slidedeck.mms` use the proposed general MMS
transform API:

```mms
let pose = presentation_anchor.world.trs()
slide_root.world.trs(pose)
```

The operation takes a one-time snapshot. Moving `presentation_anchor` afterward must not move
`slide_root` until another explicit setter call occurs.

## Approved semantics, pending implementation review

`slide_root.world.trs(pose)` means:

> Change `slide_root`'s authored/local TRS so that, after applying its effective inherited parent
> basis, its resulting world transform matches `pose`.

Conceptually:

```text
desired_local_matrix = inverse(effective_parent_world_matrix) * desired_world_matrix
desired_local_trs    = strict_decompose(desired_local_matrix)
```

For a detached root, the effective parent basis is identity, so the local and world TRS values are
equivalent. For an ordinary child, `TransformParent` target, or transform-stream output, the
engine must use the same effective basis as transform propagation.

This setter does not:

- clone or register a transform component;
- reparent `slide_root`;
- retain a reference to `presentation_anchor`;
- create a constraint or continuous follow relationship;
- interpret `pose` in local space.

The corresponding local setter is `slide_root.trs(pose)`. The existing
`slide_root.update_transform(translation, rotation_euler, scale)` remains a local-space legacy API
whose Euler rotation cannot losslessly consume the quaternion in the new TRS value.

## Current foundation

Implemented on the Mittens Engine side:

- [x] Shared `TransformTrs` copied-data struct with translation, `xyzw` quaternion, and scale.
- [x] Shared `TransformMatrix` type and checked TRS-to-matrix composition.
- [x] Local `TransformComponent` translation, rotation, scale, and TRS read accessors.
- [x] Strict matrix-to-TRS decomposition with explicit errors for non-finite, non-affine,
  singular, reflected, and sheared matrices.
- [x] Strict transform-only `TransformSystem` world translation, rotation, scale, and TRS getters.
- [x] Focused Rust tests for those value and read operations.

MMS/local mutation progress:

- [x] An opaque MMS TRS runtime value.
- [x] Local MMS `trs()` getter/setter pass-through binding.
- [ ] Local MMS `rotation()` and `scale()` getter/setter bindings.
- [ ] Effective-parent basis resolution for world writes.
- [ ] World-to-local TRS conversion.
- [ ] A space-aware transform mutation intent.
- [ ] Receiver-bound MMS `transform.world` access.
- [ ] Detached world-pose placement in `vtuber-slidedeck.mms`.

The existing Rust `IntentValue::UpdateTransformWorld` is only a propagation/cache refresh signal.
It is not a world-space setter and must not be repurposed or exposed as one.

## Slice 1: opaque first-class MMS TRS value

Represent a copied pose without allocating three nested general-purpose tables. Tentative runtime
shape:

```rust,ignore
Value::TransformTrs(TransformTrs)
```

The first slide-deck implementation does not need to inspect or decompose this value in MMS. It
only needs lossless pass-through:

```mms
let pose = presentation_anchor.world.trs()
slide_root.world.trs(pose)
```

The engine still decomposes the anchor's cached world matrix into `TransformTrs` when servicing
the getter. "Opaque in MMS" means the script cannot yet split that copied DTO into channels; it
does not mean the engine avoids matrix decomposition.

Checklist:

- [x] Add the runtime value and debug/error formatting.
- [x] Convert engine `TransformTrs` to/from the MMS value without Euler conversion.
- [x] Make the value copy-by-value with no component identity or source reference.
- [x] Allow it to pass directly from `trs()` getter to `trs(value)` setter.
- [x] Reject other value shapes with a useful expected-TRS error.
- [x] Test quaternion-preserving pass-through, copies, and useful errors.

Explicitly defer general MMS channel inspection from the first working slide-deck path. That keeps
the initial value contract small and avoids prematurely choosing between tuple, array, and record
semantics.

## Slice 2: local MMS getter/setter round trip

Implement the lowest-risk end-to-end mutation proof before world conversion:

```mms
let pose = source.trs()
target.trs(pose)
```

Checklist:

- [x] Bind zero-argument local `trs()` and the already-existing `translation()` getter.
- [ ] Bind zero-argument local `rotation()` and `scale()` getters.
- [x] Bind the one-argument local `trs()` setter.
- [ ] Bind granular one-argument translation, rotation, and scale setters.
- [x] Route the TRS setter through the existing transform mutation path rather than directly changing
  renderer caches from the evaluator.
- [ ] Preserve all channels during granular setters by applying a coherent partial patch.
- [x] Preserve quaternion rotation without converting through Euler angles.
- [x] Test that the target changes, the source does not, no component is registered, and moving
  the source later has no effect on the target.
- [x] Keep the existing three-argument `update_transform` behavior working in current examples.

The opaque `TransformTrs` pass-through is sufficient for the `trs()` round trip. Granular
translation/rotation/scale methods use their existing vector-array shapes and do not require the
phase-2 TRS indexing decision.

## Slice 3: effective parent and world-to-local conversion

Add the engine operation required by the world setter:

```rust,ignore
TransformSystem::world_to_local_trs(world, component, desired_world)
```

Checklist:

- [ ] Extract or centralize effective-parent-basis resolution so world writes and propagation use
  the same ordinary-parent, `TransformParent`, and transform-stream semantics.
- [ ] Invert the effective parent matrix and return a structured error for singular bases.
- [ ] Convert the complete desired world TRS to a local matrix and strictly decompose it.
- [ ] Do not silently discard shear created by a rotated, non-uniformly scaled parent.
- [ ] Test a detached root and an ordinary translated parent.
- [ ] Test rotated and uniformly scaled parents.
- [ ] Test non-uniform scale with representable and non-representable desired poses.
- [ ] Test singular parents and reflected matrices.
- [ ] Test `TransformParent` redirection.
- [ ] Test a transform-stream boundary and define whether stream-owned targets reject direct
  authored writes that the stream would overwrite.

## Slice 4: space-aware transform mutation intent

World conversion must happen when the queued mutation executes, not earlier during MMS evaluation;
otherwise hierarchy and parent transforms may change between evaluation and execution.

Checklist:

- [ ] Add `TransformSpace::{Local, World}` and a partial `TransformPatch` or equivalent complete
  TRS payload.
- [ ] Add a new explicit setter intent; do not overload the propagation-only
  `UpdateTransformWorld` name.
- [ ] Validate and normalize supplied channels before mutating the target.
- [ ] For a world patch, read one coherent current world TRS, replace only supplied channels,
  then convert the complete desired pose through the effective parent basis.
- [ ] Funnel the resulting local transform through existing propagation, transition, renderable,
  camera, light, collision, skinning, and BVH update behavior.
- [ ] Guarantee that conversion failure produces no partial mutation.
- [ ] Test intent execution after an intervening parent change to prove conversion is not stale.

## Slice 5: receiver-bound MMS `world` table

Implement `world` as a non-callable receiver-bound host value. It selects coordinate space for an
existing component; it is not a transform clone or copied pose.

Checklist:

- [ ] Make `transform.world` bind the live transform component ID and `TransformSpace::World`.
- [ ] Bind zero-argument world getters to strict `TransformSystem` reads.
- [ ] Bind one-argument world setters to the space-aware mutation intent.
- [ ] Support `translation`, `rotation`, `scale`, and `trs` with the same zero/one-argument rule as
  local accessors.
- [ ] Reject `transform.world()` and `T.world(transform)` with useful diagnostics.
- [ ] Do not allow the bound world table to outlive or silently retarget its receiver.
- [ ] Test that property access and getter calls allocate/register no ECS components.
- [ ] Test the exact snapshot operation between two unrelated transform roots.

## Slice 6: adapt `vtuber-slidedeck.mms`

Separate the transform that receives the world snapshot from the authored presentation offset:

```mms
let slide_offset = T.position(-0.95, 0.15, -1.25)
    .rotation(0.0, 3.14159, 0.0)
    .scale(0.055, 0.055, 1.0) {
    slide_color
}

let slide_root = T {
    name = "detached_slide_root"
    slide_offset
}
```

`slide_root` remains a top-level/detached root. The child `slide_offset` preserves the existing
right/forward/facing/scale calibration relative to each sampled presentation pose. This avoids
requiring general TRS multiplication for the first working example.

Checklist:

- [ ] Remove `slide_root` from the avatar/XR component tree.
- [ ] Split the current authored offset into a child `slide_offset` under an identity
  `slide_root` placement wrapper.
- [ ] Bind or otherwise retain the chosen live `presentation_anchor` component object.
- [ ] On ButtonB, advance the animation and snapshot/place `slide_root` once.
- [ ] On ButtonA, move backward and snapshot/place `slide_root` once.
- [ ] Remove repeated local `slide_root.update_transform(...)` calls from all five keyframes.
- [ ] Decide the initial pre-button placement behavior after live world caches are available.
- [ ] Keep slide content, font size, and color state-complete in every keyframe.
- [ ] Keep the lightweight `EditorUI` panel selection and unobstructed mirror setup.

## Deferred phase 2: TRS channel inspection ergonomics

After detached snapshot placement works, decide how MMS authors inspect or modify individual
channels. A tuple-like numeric contract could be:

```mms
pose[0] // translation: [f32; 3]
pose[1] // rotation quaternion xyzw: [f32; 4]
pose[2] // scale: [f32; 3]
```

Named reads may be more expressive:

```mms
pose.translation
pose.rotation
pose.scale
```

Current MMS arrays support numeric indexing and tables/objects support string-key/dot reads; one
general value does not currently provide both. Do not turn all MMS arrays into JavaScript-style
array/object hybrids solely for TRS. If a specialized `TransformTrs` value eventually supports
both forms, document that as its own value contract and align it with the future tuple/type-system
direction.

- [ ] Decide tuple-like numeric indexing versus named fields, or intentionally support both.
- [ ] Define whether returned channel arrays are independent copies or mutable views; prefer
  independent copies unless MMS gains explicit value/reference semantics.
- [ ] Define how an author reconstructs or patches a TRS after changing one channel.
- [ ] Add indexing/named-read, quaternion-order, out-of-range, and invalid-key tests.

## Source-anchor decision

The first implementation should name the sampled component explicitly, not rely on an ambiguous
nearest camera lookup.

Candidates:

- locomotion root: stable avatar-level pose but may not reflect current head facing;
- tracked XR/head pose: matches where the user is looking but includes head-height and head motion;
- authored presentation anchor under the XR rig: gives the example deliberate control over which
  tracked motion is sampled.

Tentative preference: an authored/bound presentation anchor near the existing `xr_pose` or camera
wrapper. Hardware calibration decides the final source.

## Automated verification

- [x] Unit-test opaque `TransformTrs` MMS conversion and direct getter-to-setter pass-through.
- [x] Add an evaluator test for local `source.trs()` → `target.trs(pose)`.
- [ ] Add TransformSystem world-to-local coverage listed in slice 3.
- [ ] Add an evaluator test for `source.world.trs()` → `target.world.trs(pose)` between unrelated
  roots.
- [ ] Extend the `vtuber-slidedeck` example regression: inject ButtonB, observe slide 1 content,
  and verify that the slide root sampled the source world pose.
- [ ] Move the source transform after placement, propagate transforms, and verify the slide world
  pose remains unchanged.
- [ ] Press ButtonB again and verify the slide takes a new snapshot.
- [ ] Run representative existing animation examples to protect the legacy `update_transform`
  surface.
- [ ] Run `cargo check --all-targets` and the relevant scripting/transform test groups.

## Hardware and performance verification

- [ ] Verify ButtonB advances and places the slide; ButtonA reverses and places it again.
- [ ] Walk and turn after placement; confirm the text remains fixed in world space.
- [ ] Confirm the next button press moves it near the avatar again using the new sampled pose.
- [ ] Recalibrate child offset for stereo readability and the lower portion of a vertical phone
  capture.
- [ ] Check the text directly and through the mirror for facing, clipping, and readability.
- [ ] Confirm no beige mirror obstruction returns.
- [ ] Compare release-mode frame time with the current example while VR and mirror rendering are
  active; world TRS work should occur only on explicit slide changes.
- [ ] Confirm ordinary skinned/mirrored examples show no performance or transform regression.

## Exit criteria

- The exact two-line MMS snapshot API works with a copied quaternion-preserving TRS value.
- `slide_root.world.trs(pose)` updates local channels so the resulting world pose matches `pose`.
- The slide root and sampled XR/avatar anchor remain topologically unrelated.
- The slide stays stationary after the user moves and repositions only on explicit slide changes.
- All five slide states work in both directions with ButtonB/ButtonA.
- Existing local `update_transform` examples continue to work.
- Release-mode VR plus mirror performance shows no meaningful regression.

TRS numeric/named channel inspection is deliberately not an exit criterion for this first
slide-deck implementation.

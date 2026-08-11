# World-TRS snapshot and VTuber slide-deck implementation review

Date: 2026-08-11

Baseline commit: `476e8e8` — local MMS `Transform::trs()` getter/setter support

Status: the first detached world-pose transfer path is working and manually improved in XR;
effective-parent hardening, mutation-error reporting, and granular channel accessors remain.

Implementation follow-up: [Shared effective transform-parent basis resolution](../draft/effective-transform-parent-basis-resolution.md)

## Scope

This review covers the uncommitted work after `476e8e8` which makes this MMS operation real:

```mms
let pose = presentation_anchor.world.trs()
slide_root.world.trs(pose)
```

It also covers the conversion of `vtuber-slidedeck.mms` from avatar-parented text to a detached
world root which is placed by taking an explicit snapshot on each A/B button press.

The work crosses three layers:

1. Mittens Engine defines world-space transform semantics and performs world-to-local conversion.
2. The engine-integrated MMS host exposes the receiver-bound `.world` view and opaque TRS transfer.
3. The VTuber slide-deck example exercises the API and supplies a separately authored local offset.

No files under `crates/meow-meow-script` changed in this slice. This is not a parser or generic
language-runtime feature, so a second review document in that crate would imply a split that does
not exist in the implementation. The existing crate draft remains useful for the eventual general
accessor syntax, but the concrete `.world` behavior belongs to the Mittens host.

## Outcome

The implemented semantics match the intended snapshot model:

```text
presentation_anchor cached world matrix
    -> strict TransformTrs copied value
    -> SetTransformTrs { space: World }
    -> inverse(target effective parent) * desired world matrix
    -> strict local TRS
    -> existing transform mutation and propagation path
```

The receiver expression `some_transform.world` does not call a constructor, clone a transform, or
register an ECS component. It produces a small receiver-bound host value containing the existing
component ID. Calling `world.trs()` with zero arguments reads; calling it with one `TransformTrs`
argument writes. The copied `TransformTrs` itself contains no component identity or relationship
to its source.

The example now has two distinct transform roles:

```text
detached_slide_root             receives a sampled world pose
└── slide_presentation_offset   keeps authored position, facing, and text scale
    └── slide content
```

This split is the important practical result. After a button press, avatar or headset movement
does not move the slide. A later button press samples the current anchor pose and relocates the
same detached slide root.

## What changed in Mittens Engine

### Shared coordinate-space vocabulary

`TransformSpace::{Local, World}` was added beside `TransformTrs`. The new
`IntentValue::SetTransformTrs` carries a complete copied TRS plus the selected space. It is kept
distinct from `UpdateTransformWorld`, whose established meaning is only “refresh transform-derived
caches after topology changes.”

### World-to-local conversion

`TransformSystem::world_to_local_trs` now:

- validates the desired TRS and composes its matrix;
- resolves an ordinary ancestor transform or a `TransformParent` target as the effective basis;
- uses identity for a detached world root;
- rejects a directly transform-stream-owned target rather than authoring a local value which the
  stream would overwrite;
- rejects singular parent matrices;
- computes `inverse(effective_parent_world) * desired_world`;
- strictly decomposes the result, preserving the existing refusal of shear, reflection, singular
  scale, and non-finite values.

The mutation executor performs that conversion when the queued intent executes. This is the right
time: using a local matrix computed during MMS evaluation could become stale if topology or the
parent transform changed before the command drain.

Successful conversion enters the existing `SystemWorld::update_transform` path, retaining normal
world propagation and render/physics/cache side effects. Failed conversion does not partially
modify the target.

## What changed in the MMS host

The engine-integrated runtime gained `Value::TransformWorld { id }`. Dot access recognizes
`.world` only on live transform component objects and returns this bound view. Method dispatch then
exposes only `trs` for the current slice:

```mms
source.world.trs()      // zero-argument getter
target.world.trs(pose) // one-argument setter
```

The getter reads the transform system's coherent cached world matrix and returns the opaque
`TransformTrs` value added by the baseline commit. The setter emits `SetTransformTrs` in world
space. Existing local `source.trs()` and `target.trs(pose)` behavior remains on the older local
`UpdateTransform` path.

This implementation settles the earlier clone ambiguity: calling `.world` selects a method
namespace for the original live component. It does not produce a detached `T {}`. Only `trs()`
produces copied transform data.

## What changed in the slide-deck example

The previous `slide_root` was a transform inside the avatar/XR hierarchy. Every keyframe also
reapplied the same local transform. That made the text continuously inherit user movement and
mixed slide content state with placement state.

The new example:

- materializes `detached_slide_root` as an independent world root;
- moves the visual calibration into `slide_presentation_offset` beneath it;
- retains the camera wrapper as the explicitly named `presentation_anchor`;
- snapshots the anchor world pose on both ButtonB and ButtonA;
- removes repeated transform mutations from all five animation keyframes;
- leaves each keyframe responsible only for text, font size, and color;
- retains the existing XR button traces;
- places the content at local offset `[1, 0.15, 1.0]` with the existing π Y-facing rotation and
  `[0.055, 0.055, 1.0]` scale.

The Z offset was changed from behind the sampled XR camera to `+1.0` after manual XR review. The X
offset is currently `1`, reflecting the latest manual calibration in the working example.

## Findings and remaining risks

### Medium: effective-parent resolution is duplicated

`world_to_local_trs` follows the same conceptual rules as `transform_changed`, but the traversal
is a new implementation rather than one centralized effective-parent resolver shared by reads,
writes, and propagation. Future changes to transform-stream operators or `TransformParent`
semantics could therefore update one path without the other.

The next hardening slice should extract a resolver which returns either an effective basis or an
explicit ownership/unresolved error, then use it from both propagation and world mutation.
The detailed implementation and test plan lives in
[Shared effective transform-parent basis resolution](../draft/effective-transform-parent-basis-resolution.md).

### Medium: queued failures are visible only on stderr

MMS receives `Null` after successfully queuing `world.trs(pose)`. If execution later rejects a
singular parent, stream-owned target, or non-representable local matrix, the mutation executor
prints an error and leaves the component unchanged. This is safe but not yet ergonomic for scripts
or editor tooling.

A future command-result or structured diagnostic channel should expose asynchronous mutation
failure without pretending the queued setter completed synchronously.

### Medium: effective-parent edge coverage is incomplete

Current tests prove a rotated ordinary parent and the detached example root. They do not yet prove:

- `TransformParent` redirection;
- singular, reflected, and non-uniformly scaled parent cases at the world-setter boundary;
- representable versus shear-producing desired poses under non-uniform scale;
- explicit rejection of a transform-stream-owned target;
- execution after an intervening parent/topology change.

Strict matrix decomposition already protects these cases from silent approximation, but the
specific error paths need regression tests.

### Low: initial pre-button placement remains undecided

Before the first A/B press, `detached_slide_root` has its identity world pose. The first button
press produces the intended placement. If the initial prompt should already appear relative to the
XR camera, the example needs a post-initialization placement event or another explicit readiness
hook; evaluating the snapshot too early would read incompletely propagated caches.

### Low: only complete opaque TRS transfer is exposed in world space

The current surface intentionally supports `world.trs()` and `world.trs(value)` only. Granular
translation, rotation, and scale getters/setters, plus inspection of copied TRS channels, remain
separate work. This is sufficient for detached snapshot placement and avoids prematurely changing
general MMS array/table semantics.

## Validation

Focused validation performed on this worktree:

```text
cargo test --example vtuber-slidedeck button_b_places_a_detached_slide_and_advances_its_content
cargo test --lib world_to_local_trs_compensates_for_the_effective_parent
cargo test --lib transform_world_trs_reads_cached_pose_and_emits_a_world_space_setter
cargo test --lib documentation_tests::guide_mms_examples_tokenize_parse_and_runnable_examples_evaluate
cargo check --all-targets
```

These checks pass. The example test proves all of the following in one live MMS scene:

1. ButtonB changes the slide content.
2. The detached root initially matches the sampled anchor world matrix.
3. Moving the avatar/anchor source afterward does not move the slide.
4. Pressing ButtonB again moves the slide to the source's new world pose.

A full `cargo test --lib` run completed 654 tests successfully and reported 40 failures in
documentation catalogs, editor/asset tests, and other areas outside the touched world-TRS path.
This review did not establish whether every one of those failures predates the slice. None of the
focused world-TRS or slide-deck tests failed. The all-target compile remains the clean
repository-wide gate for this review.

## Useful code and planning entry points

- [shared transform values](../../src/engine/transform.rs)
- [world reads and world-to-local conversion](../../src/engine/ecs/system/transform_system.rs)
- [space-aware transform intent](../../src/engine/ecs/rx/signal.rs)
- [transform mutation execution](../../src/engine/ecs/rx/mutation_executor.rs)
- [MMS component-method binding](../../src/scripting/component_method_registry.rs)
- [MMS `.world` evaluation](../../src/scripting/world_evaluator.rs)
- [VTuber slide-deck scene](../../examples/vtuber-slidedeck.mms)
- [implementation tracker](../task/vtuber-slidedeck-detached-world-trs.md)

## Recommendation

Keep the current API and the detached-root/offset-child example structure. The semantic division is
clean: Mittens Engine owns coordinate spaces and conversion; the MMS host exposes receiver-bound
methods; the example owns presentation calibration.

Before expanding to granular world accessors, centralize effective-parent resolution and add the
missing boundary tests. Manual XR review should decide the final X/Y/Z offset, facing rotation, and
initial pre-button behavior independently of the transform API.

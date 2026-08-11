# Transform component accessors: Mittens engine API

Status: in progress; shared values, local reads, strict decomposition, and world reads implemented;
writes pending.

Related:

- [VTuber slide-deck detached world-TRS implementation](../task/vtuber-slidedeck-detached-world-trs.md)
- [MMS transform component accessors](../../crates/meow-meow-script/docs/draft/transform-component-accessors.md)
- [Transform mutation API v2](transform-mutation-api-v2.md)
- [Transform pipeline cleanup checklist](../task/transform-pipeline-cleanup-checklist.md)
- [Legacy transform-pipeline and command-queue cleanup](../task/legacy-transform-pipeline-and-command-queue-cleanup.md)
- [VTuber slide-deck XR placement and controls](../task/vtuber-slidedeck-xr-placement-and-controls.md)

## Goal

Provide the Rust engine operations needed to implement this MMS surface without cloning transform
components:

```mms
let local_pose = transform.trs()
transform.trs(local_pose)

let world_pose = transform.world.trs()
transform.world.trs(world_pose)
```

The Rust API does not need to reproduce MMS member syntax one field at a time. MMS's `world` table
is an evaluator/runtime binding that selects a coordinate space for an existing component. It is
not stored inside `TransformComponent`.

## Why `TransformComponent` should not contain `world: TransformWorldMethods`

A JavaScript-shaped model might look like:

```rust,ignore
struct TransformComponent {
    world: TransformWorldMethods,
}
```

That is not the right ownership model in Rust:

- Rust methods live in `impl` blocks and require no per-instance method-table field.
- A zero-sized `TransformWorldMethods` field would know neither the target `ComponentId` nor the
  containing `World`, so every operation would still need those passed separately.
- A useful reference-holding helper would need to borrow the ECS `World`. A component stored inside
  that same world cannot safely contain such a self-reference.
- World transforms depend on ancestry, `TransformParent`, transform-stream boundaries, and cached
  propagation. Those rules belong to `TransformSystem`, not to a leaf data component.
- `TransformComponent` is currently `Clone + Copy`. Adding an embedded context object would make
  those semantics misleading or impossible.

The existing ownership split is already close to what this API needs:

- `TransformComponent` owns authored/local translation, quaternion rotation, and scale.
- Its inner renderer `Transform` contains `model` and the derived `matrix_world` cache.
- `TransformSystem` maintains that cache and understands the effective hierarchy.

## Shared copied value: `TransformTrs`

Introduce one engine-level plain-data value for copied transform channels:

```rust,ignore
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformTrs {
    pub translation: [f32; 3],
    pub rotation_quat_xyzw: [f32; 4],
    pub scale: [f32; 3],
}
```

`TransformTrs` is not a component. It has no `ComponentId`, is never registered in `World`, and
does not retain a reference to its source. It is the Rust counterpart of the MMS TRS tuple.

`TransformPipelineChannels` currently has this exact shape inside `TransformStreamSystem`.
Prefer extracting or converting through the shared `TransformTrs` type rather than allowing two
independent TRS decomposition and validation contracts to grow. The transform-stream-specific
name can remain as a wrapper or type alias if its domain vocabulary is still useful.

Suggested value operations:

```rust,ignore
impl TransformTrs {
    pub const IDENTITY: Self = /* ... */;

    pub fn normalized(self) -> Result<Self, TransformAccessError>;
    pub fn to_matrix(self) -> TransformMatrix;
    pub fn from_matrix(matrix: TransformMatrix) -> Result<Self, TransformAccessError>;
}
```

`from_matrix` must have an explicit policy for negative scale and shear. It must not silently
pretend that every affine matrix is exactly representable as TRS.

## Recommended module home

Use a focused engine-domain module rather than a generic `core.rs` or `primitives/mod.rs` bucket:

```text
src/engine/transform.rs
```

with:

```rust,ignore
pub mod transform;
pub use transform::{TransformMatrix, TransformTrs};
```

`TransformTrs` and `TransformMatrix` are neither ECS components nor graphics-only types. They are
shared transform-domain values used by ECS propagation, renderer instances, skinning, XR, gizmos,
transform streams, and scripting. A dedicated module makes that ownership explicit and avoids
turning `core` or `primitives` into a miscellaneous dependency bucket.

`TransformMatrix` currently lives in `engine::graphics::primitives`. Move its definition to the
new module but re-export it from the old path initially:

```rust,ignore
// engine/graphics/primitives.rs
pub use crate::engine::transform::TransformMatrix;
```

That preserves existing imports while new shared code uses `engine::transform`. Migrating old
imports can happen later with the deferred terminology cleanup; it should not inflate the first
accessor patch.

The renderer-oriented `Transform` struct can remain in `graphics::primitives` for now because it
also owns cached `model` and `matrix_world` matrices. Add conversions between its local channels
and `TransformTrs` rather than moving that larger type in the same change.

## Local operations belong on `TransformComponent`

Local values require no ECS traversal, so ordinary component methods are appropriate:

```rust,ignore
impl TransformComponent {
    pub fn translation(&self) -> [f32; 3];
    pub fn rotation_quat_xyzw(&self) -> [f32; 4];
    pub fn scale(&self) -> [f32; 3];
    pub fn trs(&self) -> TransformTrs;
}
```

The Rust rotation name includes `quat_xyzw` because Rust callers benefit from representation being
explicit. MMS can still expose the shorter `rotation()` name with a documented quaternion type.

Local writes should reuse the existing intent/update path rather than creating a second mutation
mechanism. The current component already has `set_position`, `set_rotation_quat`, and `set_scale`
methods that emit `UpdateTransform`. They can be regularized around array values and a full TRS
operation, but the implementation must avoid recursively emitting partially stale state.

Possible component-level helpers are:

```rust,ignore
impl TransformComponent {
    pub fn set_translation(
        &mut self,
        emit: &mut dyn SignalEmitter,
        value: [f32; 3],
    );

    pub fn set_rotation_quat_xyzw(
        &mut self,
        emit: &mut dyn SignalEmitter,
        value: [f32; 4],
    ) -> Result<(), TransformAccessError>;

    pub fn set_scale(
        &mut self,
        emit: &mut dyn SignalEmitter,
        value: [f32; 3],
    ) -> Result<(), TransformAccessError>;

    pub fn set_trs(
        &mut self,
        emit: &mut dyn SignalEmitter,
        value: TransformTrs,
    ) -> Result<(), TransformAccessError>;
}
```

These are Rust names; they do not force MMS to use `set_` names. MMS's zero-or-one argument
overload is resolved by the script evaluator before invoking or emitting the engine operation.

## World operations belong on `TransformSystem`

A world-space read needs both a `World` and a component identity. Put this logic beside the
existing `world_model` and `world_position` functions:

```rust,ignore
impl TransformSystem {
    pub fn world_translation(
        world: &World,
        component: ComponentId,
    ) -> Result<[f32; 3], TransformAccessError>;

    pub fn world_rotation_quat_xyzw(
        world: &World,
        component: ComponentId,
    ) -> Result<[f32; 4], TransformAccessError>;

    pub fn world_scale(
        world: &World,
        component: ComponentId,
    ) -> Result<[f32; 3], TransformAccessError>;

    pub fn world_trs(
        world: &World,
        component: ComponentId,
    ) -> Result<TransformTrs, TransformAccessError>;
}
```

`world_translation` can read the cached matrix directly. Rotation, scale, and full TRS should use
one shared decomposition utility so a single `world_trs` call samples one coherent matrix.

The methods should operate on actual `TransformComponent` IDs for the MMS API. The existing
`world_model` also accepts non-transform nodes by finding their nearest transform ancestor; keep
that broader convenience separate so a script bug cannot silently target an ancestor.

## World writes are conversions, not component methods

Setting a world channel means converting a desired world-space value into the target's local
parent space. `TransformComponent` cannot do this alone because it does not own the ECS hierarchy.

The core system operation should resolve an entire desired world TRS to a local TRS:

```rust,ignore
impl TransformSystem {
    pub fn world_to_local_trs(
        world: &World,
        component: ComponentId,
        desired_world: TransformTrs,
    ) -> Result<TransformTrs, TransformAccessError>;
}
```

Conceptually:

```text
desired_local_matrix = inverse(effective_parent_world_matrix) * desired_world_matrix
desired_local_trs    = decompose(desired_local_matrix)
```

The difficult phrase is **effective parent world matrix**. It is not always the cached matrix of
the nearest structural transform ancestor. `TransformParentComponent` and transform-stream
boundaries can replace the inherited basis. The conversion helper must share the same basis
resolution used by propagation; do not reimplement a simpler structural-parent walk in the MMS
evaluator or intent executor.

For a world translation-only setter, read the target's current coherent world TRS, replace only
translation, then resolve the complete desired world TRS back to local space. Rotation-only and
scale-only setters follow the same read-modify-convert procedure.

## Intent shape and execution timing

Do not calculate a world setter's local TRS during MMS evaluation. Hierarchy or transforms may
change before queued intents execute, producing a stale conversion. Keep the coordinate-space
choice and partial patch in the intent until the normal mutation drain point.

A consolidated shape avoids multiplying near-identical intent variants:

```rust,ignore
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformSpace {
    Local,
    World,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TransformPatch {
    pub translation: Option<[f32; 3]>,
    pub rotation_quat_xyzw: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
}

IntentValue::SetTransform {
    component_id: ComponentId,
    space: TransformSpace,
    patch: TransformPatch,
}
```

At execution time:

1. Require `component_id` to identify a live `TransformComponent`.
2. Validate all supplied numbers and normalize the quaternion.
3. Read either the current local TRS or one coherent current world TRS.
4. Replace the channels present in `patch`.
5. For `World`, convert the complete desired world TRS through the effective parent basis.
6. Emit or execute the existing local `UpdateTransform` path so propagation, transitions,
   renderer updates, cameras, collision, skinning, and BVH invalidation remain centralized.

This intent supports all eight MMS accessor families—local/world times
translation/rotation/scale/TRS—with zero arguments handled as synchronous reads and one argument
handled as an intent-producing write.

Open question: should a full `trs` write use a dedicated non-optional payload rather than a patch
with all three fields populated? Either shape is valid if execution and validation are shared.

## Error model

Use structured Rust errors internally and translate them into useful MMS runtime errors:

```rust,ignore
pub enum TransformAccessError {
    MissingComponent(ComponentId),
    NotTransform(ComponentId),
    WorldTransformUnavailable(ComponentId),
    EffectiveParentUnavailable(ComponentId),
    SingularParent(ComponentId),
    NonFiniteValue,
    DegenerateQuaternion,
    NonDecomposableMatrix,
    ShearNotRepresentable,
}
```

Do not return identity values for unavailable world state and do not partially mutate a transform
after a conversion failure.

## Shear and scale policy

The repository currently contains more than one matrix-decomposition implementation, including
logic in `TransformStreamSystem` and `grabbable_system`. Consolidate this before exposing a public
world TRS contract.

Non-uniformly scaled, rotated ancestors can produce shear in a descendant's world matrix. Exact
world translation and rotation access remain possible, but a full TRS decomposition or world TRS
write may be lossy or impossible. The first implementation should detect unsupported shear and
return an error rather than silently changing the pose. A later API can deliberately add an
approximate decomposition mode if a real use case needs it.

## MMS binding boundary

The MMS evaluator can represent `transform.world` as a small bound script-host value such as:

```rust,ignore
struct BoundTransformMethods {
    component_id: ComponentId,
    space: TransformSpace,
}
```

This value belongs to the scripting runtime, not `TransformComponent`. It is not an ECS component,
is not registered, and does not clone transform state. Property access binds the component ID and
space; the terminal method performs a getter or emits a setter intent.

The name above is illustrative. The scripting implementation may use an enum variant instead of a
Rust struct:

```rust,ignore
Value::TransformMethods {
    component_id,
    space: TransformSpace::World,
}
```

## Focused implementation sequence

1. Add `TransformTrs` and central matrix composition helpers with finite-value and quaternion
   tests.
2. Add read-only local methods on `TransformComponent`.
3. Add central matrix decomposition with singular-matrix, negative-scale, and shear tests, then
   add strict transform-only world getters on `TransformSystem`.
4. Extract a shared effective-parent-basis resolver from transform propagation.
5. Add `world_to_local_trs` tests for a root, ordinary parent, rotated parent, non-uniform scale,
   singular parent, `TransformParent`, and transform-stream boundary.
6. Add the space-aware partial transform intent and route it through existing `UpdateTransform`
   behavior.
7. Bind the MMS local methods and receiver-bound `world` method table.
8. Add MMS integration tests proving getters do not register components, setters mutate only the
   receiver, copied values retain no relationship, and world writes preserve unspecified channels.
9. Use the API to detach and place the VTuber slide deck, then perform XR verification.

## Recommended next implementation chunk

Keep the first backend patch to plain values and local reads:

1. Add `src/engine/transform.rs` with `TransformMatrix`, `TransformTrs`, identity, finite-value
   validation, quaternion normalization, and local TRS-to-matrix composition.
2. Re-export `TransformMatrix` from `graphics::primitives` so existing callers do not move yet.
3. Add lossless conversions between `TransformTrs` and the local channels of the existing
   renderer `Transform`.
4. Make `TransformPipelineChannels` a temporary alias or thin conversion around `TransformTrs`;
   do not perform the broader pipeline-to-stream rename in this patch.
5. Add `translation`, `rotation_quat_xyzw`, `scale`, and `trs` read methods to
   `TransformComponent`.
6. Add focused unit tests for identity, conversion round trips, matrix composition, quaternion
   normalization, non-finite rejection, component local getters, and proof that copying
   `TransformTrs` creates no ECS component.

Explicitly defer from this first chunk:

- matrix-to-TRS decomposition and its shear/negative-scale policy;
- world getters;
- local or world setter intents;
- the MMS `world` table and tuple conversion;
- transform-pipeline terminology cleanup;
- `CommandQueue` terminology cleanup.

This first slice should touch roughly one new module plus `engine/mod.rs`,
`graphics/primitives.rs`, `transform_stream_system.rs`, `component/transform.rs`, and their tests.
It is a low-to-medium-risk foundation. World reads are the next separate slice because they force
the decomposition policy; world writes are a third slice because they additionally require
effective-parent conversion and intent timing.

Implementation result:

- [x] Added `engine::transform::{TransformMatrix, TransformTrs, TransformTrsError}`.
- [x] Preserved the old `graphics::primitives::TransformMatrix` path as a re-export.
- [x] Added validated quaternion normalization and TRS-to-matrix composition.
- [x] Added conversions between `TransformTrs` and renderer `Transform` local channels.
- [x] Replaced the duplicated `TransformPipelineChannels` struct with a temporary
  `TransformTrs` alias.
- [x] Added local `TransformComponent` translation, quaternion rotation, scale, and TRS getters.
- [x] Added focused value, conversion, no-component-clone, and transform-stream compatibility
  tests.
- [x] Added strict matrix-to-TRS decomposition. Singular scale, shear, non-affine matrices, and
  negative-determinant/reflected matrices return errors; even negative-axis pairs canonicalize to
  their equivalent positive-scale rotation.
- [x] Added strict transform-only world translation, rotation, scale, and TRS getters on
  `TransformSystem`. Translation remains readable from a sheared matrix; the decomposed getters
  reject it.

## Open questions

- Should the shared value be named `TransformTrs`, `TransformChannels`, or simply `Trs`?
- Should negative scale be preserved by decomposition, rejected initially, or represented by a
  separate handedness flag?
- Should world rotation remain available when the full matrix contains shear, and which
  orthonormalization rule defines it?
- Should world setters participate in `TransitionComponent` interpolation exactly like current
  local `UpdateTransform`, or provide an explicit immediate mode later?
- Does a transform-stream-owned target allow direct world writes, or should the engine reject them
  because the stream will overwrite the result on its next evaluation?

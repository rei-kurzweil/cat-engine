# Joint basis retargeting system

Date: 2026-08-04

Status: proposed architecture; not yet implemented as a retained ECS system

## Purpose

Imported armatures do not share dependable joint-local axis conventions. Once
humanoid bone mapping identifies the joints that have particular anatomical
roles, runtime consumers still need to know how a mapped joint's authored rest
frame relates to Mittens' canonical pose frame.

The joint basis retargeting system derives that conversion once, caches it by
the target joint's `ComponentId`, and serves the same result to every consumer.

```text
bone mapping
  identifies target joints and anatomical landmarks
        │
        ▼
joint basis retargeting system
  derives one canonical anatomical basis per target joint
        │
        ├── AvatarControlSystem
        ├── PointerSystem
        ├── articulated-hand retargeting
        └── future animation/pose consumers
```

This prevents AVC, pointers, and other systems from independently resolving
the same landmarks and recalculating subtly different corrections.

## Core invariant

Within one live ECS world and one imported-armature generation:

> A target joint may have at most one canonical retarget-basis definition.

The primary cache key is therefore only the target joint's generational
`ComponentId`:

```rust
HashMap<ComponentId, RetargetBasisEntry>
```

Left and right hands naturally receive distinct entries because they are
distinct joints. The cache does not need a `Hand`, `Foot`, `Head`, or other
resource-kind enum to distinguish them.

Registering two different definitions for the same target joint is an error.
The system must reject the conflict and report both definition sources. It must
not use first-wins, last-wins, component traversal order, or consumer priority.

Replacing a definition is an explicit atomic configuration change: invalidate
the old entry, validate and derive the new entry, then publish a new cache
generation.

## Canonical frame

Mittens uses one canonical anatomical frame for these cached bases:

```text
-Z = forward
+Y = up
+X = the remaining right-handed orthogonal axis
```

The cached result describes the relationship between that canonical frame and
the imported target joint's rest-local frame:

```rust
struct ResolvedRetargetBasis {
    canonical_to_target_rest: [f32; 4],
    target_rest_to_canonical: [f32; 4],
    forward_target_local: [f32; 3],
    up_target_local: [f32; 3],
    right_target_local: [f32; 3],
    generation: u64,
}
```

The exact Rust representation is an implementation detail, but both quaternion
directions should be named explicitly. Consumers should not repeatedly invert
or guess the stored direction.

OpenXR Aim happens to use `-Z` as forward, but the retained avatar basis is not
owned by OpenXR. A source-specific adapter composes its source frame with the
cached canonical/avatar conversion.

## Basis definition

A definition identifies one target joint and two non-collinear anatomical
directions using already mapped joints or landmarks:

```rust
struct RetargetBasisDefinition {
    target: ComponentId,
    forward: LandmarkDirection,
    up: LandmarkDirection,
    provenance: RetargetBasisProvenance,
}

struct LandmarkDirection {
    from: ComponentId,
    to: ComponentId,
}
```

The mathematical construction is:

```text
forward = normalize(forward.to - forward.from)
up_raw  = up.to - up.from
up      = normalize(up_raw projected perpendicular to forward)
back    = -forward
right   = normalize(up × back)
```

Landmark positions are read from immutable imported rest poses and expressed in
the target joint's rest-local space before constructing the basis.

The definition is relational and generic. The retargeting system does not
dispatch on body-part type.

### Proven hand definition

The validated Bisket hand recipe is:

```text
target  = Hand
forward = Middle1 -> Middle3
up      = Little1 -> Index1
```

That is the first reference definition, not a hand-specific branch inside the
generic cache.

## Relationship to bone mapping

Bone mapping owns identity:

```text
LeftHand    -> imported joint 123
LeftMiddle1 -> imported joint 130
LeftMiddle3 -> imported joint 132
LeftIndex1  -> imported joint 126
LeftLittle1 -> imported joint 138
```

A mapping profile or a small recipe layer uses those mapped slots to construct
the generic basis definition:

```text
target  = mapped LeftHand
forward = mapped LeftMiddle1 -> mapped LeftMiddle3
up      = mapped LeftLittle1 -> mapped LeftIndex1
```

The retargeting system does not search names, infer symmetry, choose humanoid
slots, or decide which recipe applies. It receives resolved component IDs and
derives geometry from them.

Changing how bones were discovered must not change the retargeting math. An
explicit MMS mapping, embedded avatar metadata, and successful automapping
should produce the same definition when they resolve the same joints.

## Cache and lookup contract

After successful registration and derivation:

```rust
retargeting.basis_for(target_joint) -> Option<&ResolvedRetargetBasis>
```

The steady-state lookup must be constant-time and must not:

- run component queries;
- traverse the armature;
- read or compose rest transforms again;
- identify the target's anatomical role;
- allocate a new basis value;
- depend on which consumer requested it first.

Consumers requesting the same target joint receive the same cache generation
and conversion.

The system should also expose status separately from successful lookup:

```text
WaitingForMapping
WaitingForGltf
Ready
Invalid
ConflictingDefinition
```

An absent entry must not ambiguously mean all of those states.

## Validation

Before publishing a cache entry, the system verifies that:

- the target and all landmarks still exist;
- all referenced components belong to the intended imported armature instance;
- immutable rest-pose transforms are available;
- each direction has non-zero length;
- projected up is not collinear with forward;
- the resulting frame is finite, normalized, orthogonal, and right-handed;
- no other definition is registered for the target joint.

Recipe-specific validation, such as requiring finger landmarks beneath a hand,
belongs to the recipe or mapping layer. The generic geometry calculation only
requires that all landmarks can be expressed consistently in target-rest space.

Invalid definitions remain invalid until a relevant lifecycle change occurs.
They are not retried every frame.

## Lifecycle and invalidation

The retained system derives entries only when dependencies change:

1. a basis definition is registered;
2. its owning GLTF becomes initialized;
3. the humanoid bone map becomes ready or changes;
4. the GLTF is respawned;
5. a definition is explicitly replaced or removed;
6. the owning avatar subtree is removed.

Each successful recomputation increments the affected entry or owning-avatar
generation. Consumers retaining a result across lifecycle changes must compare
that generation or request the entry again.

GLTF respawn normally creates new generational component IDs. Old target-keyed
entries must be removed before definitions are resolved against the replacement
joint set.

There is no per-frame discovery or reconciliation pass.

## Ownership

Each definition and cache entry belongs to one imported avatar/GLTF instance.
The system should retain reverse indexes sufficient to invalidate locally:

```text
owning GLTF -> target joints
landmark joint -> dependent target joints
definition component/source -> target joint
```

These indexes support targeted GLTF respawn, mapping changes, and subtree
cleanup without scanning every retarget basis.

## Consumer responsibilities

The retargeting system returns a source-neutral joint basis. Consumers remain
responsible for how it is used.

`AvatarControlSystem`:

- chooses the active controller, Aim/Grip, or wrist/palm source;
- composes the source pose with `target_rest_to_canonical` as required;
- applies the resulting rotation to its visual target and IK pipeline.

`PointerSystem`:

- requests the same target-joint basis when its presentation must share the
  avatar hand frame;
- owns pointer and laser transforms;
- resolves any separate fingertip attachment position.

An arbitrary attachment point such as a fingertip is not part of the target
joint's unique retarget basis. Attachment placement should use mapped joints or
a separate cached rest-space transform so that multiple attachments do not
violate the one-basis-per-target invariant.

## What the system does not do

The joint basis retargeting system does not:

- discover or map bones;
- contain a fixed enum of retargetable body-part types;
- select a basis recipe from anatomical role names;
- parse GLTF, VRM, or VRMC metadata;
- select live tracking or animation pose sources;
- solve IK;
- apply transforms to joints;
- drive finger animation;
- spawn pointers, lasers, or debug geometry;
- perform interactive calibration;
- accept multiple competing bases for one target joint;
- recompute entries every frame.

## Diagnostics

For each target joint, diagnostics should report:

- target component ID and display name;
- owning GLTF/avatar;
- definition provenance;
- forward/up landmark component IDs and names;
- normalized forward, up, and right vectors in target-rest space;
- both conversion quaternions;
- cache generation and lifecycle status;
- conflicts, missing dependencies, or degeneracy reasons.

Diagnostics inspect retained entries. Enabling them must not change basis
selection or trigger continuous recomputation.

## Hand-helper migration

The 0.8 migration is complete. `XRHand` no longer resolves anatomical landmark selectors or
constructs definitions. Explicit `JointRetargetBasis` declarations register through the generic
retained path; AVC and PointerSystem query them by target `ComponentId`. `RestAttachment` resolves
the fingertip offset separately, and the former `avatar_hand_pose_basis.rs` compatibility helper
has been removed.

## Required tests

- one definition per target produces one cached entry;
- left and right joints produce independent entries;
- two consumers receive the same entry and generation;
- duplicate identical definitions are diagnosed rather than silently counted
  twice;
- conflicting definitions for one target are rejected;
- replacing a definition invalidates and republishes atomically;
- missing GLTF/mapping dependencies wait without frame polling;
- missing, zero-length, and collinear landmarks become invalid;
- GLTF respawn removes old generational component keys;
- removing one avatar does not invalidate another avatar's entries;
- the Bisket hand definition reproduces forward `-Z` and palm up `+Y` under
  controller Aim without a fixed correction angle.

## Related documents

- [avatar hand pose basis](avatar-hand-pose-basis.md)
- [bone mapping system](bone-mapping-system.md)
- [avatar control](avatar-control.md)
- [current bone-mapping review](../review/current-humanoid-bone-mapping-and-avatar-slot-resolution.md)
- [humanoid bone-map task](../task/humanoid-bone-map-automapping-and-mms-presets.md)
- [pose-basis review](../review/imported-humanoid-pose-basis-detection-and-conversion.md)

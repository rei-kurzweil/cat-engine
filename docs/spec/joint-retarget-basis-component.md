# `JointRetargetBasis`

`JointRetargetBasis` declares one canonical rest-pose basis for one joint in an imported
armature. It is authored beneath the `GLTF` instance that owns the target and all four
landmarks:

```mms
GLTF.new("../avatar.glb") {
  JointRetargetBasis.new(
    "#J_Bip_L_Hand",
    "#J_Bip_L_Middle1",
    "#J_Bip_L_Middle3",
    "#J_Bip_L_Little1",
    "#J_Bip_L_Index1"
  )
}
```

The five required arguments are the target joint, forward start, forward end, up start,
and up end. Thus this example defines `forward = Middle1 -> Middle3` and
`up = Little1 -> Index1`. Each argument accepts the existing `ComponentRef` selector and
`@uuid:` forms. Serialization preserves those five arguments and their surface forms.

## Resolution and runtime contract

The nearest ancestor `GLTF` owns the definition. Every reference must resolve to exactly
one member of that instance's `armature_joint_transforms`; global, first-match, and
cross-armature GUID resolution are invalid. Registration before imported joints exist has
status `WaitingForGltf`. `GltfInitialized` triggers one targeted retry. Missing or ambiguous
references after initialization are `Invalid` and are not polled each frame.

The component's `ComponentId` is its definition-source identity. Provenance is generated
from that identity, its stored label, and its owning `GLTF`; MMS cannot supply provenance.
All statuses and computed matrices are retained by `JointBasisRetargetingSystem`, not
serialized on the component.

Rest matrices are immutable snapshots derived from `BoneRestPoseComponent`. Landmark
positions are expressed in target-rest-local space. The system orthogonalizes the authored
up direction against forward and constructs a right-handed basis in which canonical `-Z`
maps to forward and canonical `+Y` maps to up. It retains both
`canonical_to_target_rest` and `target_rest_to_canonical`, the normalized forward/up/right
axes, and a monotonically increasing generation for the target.

## Uniqueness and replacement

Two active sources for one target have `ConflictingDefinition`, even when identical. No
basis is published while conflicted. Removing either source validates and republishes the
remaining definition. Rust code may atomically replace a source with
`replace_definition(source, definition)`; the previous publication is invalidated first,
so an invalid replacement cannot leave a stale usable basis. A future component mutation
API must emit `RetargetBasisConfigurationChanged` and follow this same path.

## Boundaries and compatibility

This component does not infer humanoid slots, contain body-part enums, choose tracking
sources, define pointer positions, solve IK, apply transforms, carry per-avatar correction
angles, or define multiple bases for one target.

Full legacy `XRHand` configurations are translated to the same internal definition. The
knuckle recipe uses `Middle1 -> Middle3` and `Little1 -> Index1`; the thumb recipe preserves
the distal-middle direction and `Middle1 -> Thumb1`. Forward-only XR configuration remains
on its shortest-arc compatibility helper because it cannot define two axes. Compatibility
fallback is allowed only when no generic definition exists; it cannot hide `Invalid` or
`ConflictingDefinition`. An authored declaration and a full XR recipe for the same target
therefore intentionally conflict.

The future `HumanoidBoneMap` integration will resolve semantic slots and construct the same
`RetargetBasisDefinition`. It does not change this component's geometry or cache contract.

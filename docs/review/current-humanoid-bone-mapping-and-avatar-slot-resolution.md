# Current humanoid bone mapping and avatar slot resolution

Date: 2026-08-04

Status: current-state audit; no runtime changes made by this review

## Purpose

This review inventories how Mittens currently connects semantic humanoid roles
such as `head`, `left hand`, and `left lower arm` to nodes in an imported GLTF
armature. It also identifies which existing data can support automatic mapping
and automatic hand-basis detection.

The short answer is that Mittens has most of the raw ingredients, but not one
authoritative humanoid bone map:

- GLTF import retains the joint set, hierarchy, names, and immutable rest poses;
- AVC exposes several semantic bone-name fields and caches some resolved IDs;
- a stateless `BoneMappingSystem` can resolve arm and spine chains;
- pose capture and secondary motion already demonstrate GLTF-instance-scoped,
  unambiguous selector resolution;
- the new XR hand-basis path can derive orientation from finger landmarks.

Those pieces are not currently assembled into a shared mapping object. AVC's
actual resolution is local to its initialization code, and the existing
`BoneMappingSystem` is not called by runtime code.

## Terminology

This review uses three distinct meanings that are currently easy to conflate:

- **semantic slot**: an anatomical role such as `head`, `leftHand`, or
  `rightMiddleDistal`;
- **authored reference**: a name, query, or GUID intended to select a GLTF node;
- **resolved mapping**: the `ComponentId` chosen for a semantic slot in one
  spawned GLTF instance, together with how confidently it was chosen.

Bone-local axes are a separate concern. A mapping can correctly identify the
hand bone while still not saying which direction its fingers or palm face.

## Current machinery

| Layer | Current data or behavior | Current consumer | Main limitation |
| --- | --- | --- | --- |
| GLTF import | Every imported node gets a transform and name; skin joints are retained in `armature_joint_transforms` | Skinning, pose capture, secondary motion, debug tooling | No semantic humanoid roles are assigned |
| Rest pose | Each imported node gets an immutable `BoneRestPoseComponent` sidecar | AVC and secondary motion | It describes authored geometry, not anatomy by itself |
| ECS hierarchy | Parent/child topology mirrors the imported node graph | AVC fallbacks and topology-driven systems | Helper/twist joints can make fixed parent counts unreliable |
| AVC authored fields | Strings for head, hands, upper/lower arms, camera, hips, and neck | `AvatarControlSystem` | Fragmented, partly optional, and not a reusable map |
| AVC runtime caches | IDs for the head, hands, camera splice, neck, and generated targets | `AvatarControlSystem` | Not a complete humanoid map; arms are passed directly to IK and legs/fingers are absent |
| `BoneMappingSystem` | Stateless arm/spine chain helpers with optional explicit names and distance walking | No current runtime caller | Existing spec overstates its integration and scope |
| XR hand basis | Middle-finger chain, optional thumb fallback, and optional index/little knuckle landmarks | Controller-driven visual hand target and laser mount | References live on `XRHand`, rather than in shared avatar semantics |
| Pose capture | Exact, instance-owned joint selector resolution | Pose libraries | Resolves pose entries, not semantic slots |
| Secondary motion | `ComponentRef`, scoped resolution, topology discovery, reusable MMS presets | Spring chains and colliders | The pattern is reusable, but the data is not a humanoid map |

## What AVC actually maps today

`AvatarControlComponent` acts as both authored configuration and a partial
runtime cache. The effective slots are:

| Authored field | Default | Current resolution | Missing or disabled behavior |
| --- | --- | --- | --- |
| `head_bone` | `J_Bip_C_Head` | First `#name` match under the model root | Required; init silently retries if it is not found |
| `left_hand_bone` / `right_hand_bone` | `None` | First `#name` match | `None` disables that hand splice; a missing name is skipped |
| upper/lower arm fields | `None` | Explicit first-name match, otherwise two direct parents above the hand | Missing explicit names disable IK for that side; inferred helpers are only warned about |
| `camera_bone` | `None` | Configured name, otherwise `head_bone` | Despite older comments, `None` currently falls back to the head |
| `hips_bone` | `None` | Not consumed by current AVC runtime | Authored and serialized, but currently has no effect |
| `neck_bone` | `J_Bip_C_Neck` | First `#name` match | Rust can opt out with `without_neck_pin`; the MMS surface does not currently expose that opt-out |

The model root itself is selected as the first transform child beneath AVC's
driven subtree. This is another implicit structural assumption rather than a
semantic mapping.

There is no central resolved map for shoulders, chest/spine levels, legs,
feet, toes, eyes, or fingers. Upper/lower arm IDs are resolved while constructing
IK chains rather than retained as named avatar slots. Current AVC does not set
up leg IK.

### Resolution and failure semantics

AVC uses `World::find_component` with an ID/name selector. That returns the
first traversal match; it does not prove that the match is unique or belongs to
the owning GLTF skin. This matters when a scene contains repeated node names,
multiple armatures, helper subtrees, or another imported model below the same
root.

The fallback for an unspecified arm chain is:

```text
hand.parent        -> lower arm
hand.parent.parent -> upper arm
```

It does not currently use bone length to skip helper joints. Names containing
patterns such as `Twist`, `Roll`, `Helper`, `_collider`, or `J_Sec_` cause a
warning after selection but do not trigger a better search.

## The existing `BoneMappingSystem`

`src/engine/ecs/system/bone_mapping_system.rs` contains useful stateless
helpers:

- `resolve_arm_chain`, supporting explicit names and topology/distance fallback;
- `resolve_spine_chain`, supporting a hips name and branching-ancestor fallback;
- ancestor-at-distance and branching-ancestor utilities.

The module is compiled and exported, but repository-wide call-site inspection
finds no runtime consumer. In particular, `AvatarControlSystem` duplicates a
simpler arm fallback instead of calling it.

Consequently, parts of `docs/spec/bone-mapping-system.md` and
`docs/spec/avatar-control.md` describe intended behavior rather than current
behavior. Claims that AVC calls `resolve_arm_chain`, applies the documented
resolution tiers, or has a VRM naming preset should not be treated as
implemented facts.

The name `BoneMappingSystem` is therefore already occupied, but today it means
"chain resolution utility," not "persistent humanoid semantic map."

## What GLTF import makes available

For each initialized GLTF instance, `GLTFComponent` retains:

- `spawned_node_transforms`: all imported node transforms;
- `armature_joint_transforms`: the subset used as skin joints.

Each node also has:

- its imported display name, with limited sanitization;
- parent/child topology;
- immutable local bind/rest translation, rotation, and scale in a
  `BoneRestPoseComponent`.

This is sufficient input for name, topology, length, rest-position, and
left/right symmetry analysis. Candidate search should normally be restricted
to `armature_joint_transforms`; the complete node list may include meshes,
helpers, colliders, and secondary-motion nodes.

Two caveats are important:

- the retained joint vector is a membership set, not a documented semantic or
  topological ordering;
- GLTF skin `skeleton_root` is parsed internally but is not currently used to
  identify an armature root.

No current importer code was found that consumes VRM or VRMC humanoid
`humanBones` metadata. Such metadata is a possible future high-confidence
source, not a feature available today.

## Existing reusable-resolution patterns

Pose capture and secondary motion provide stronger precedents than AVC's
first-match lookup.

Pose application resolves each saved joint selector against the destination
GLTF's owned skin joints. Missing or ambiguous entries reject the mapping
atomically instead of partially applying a pose.

Secondary motion similarly scopes ordinary selectors to the owning GLTF
instance and requires exactly one result. It distinguishes dependencies that
are not ready yet from invalid configuration, and it supports topology-based
discovery such as `SpringBone.from_root(...)`.

Their MMS assets demonstrate two useful packaging forms:

- a library module that imports and aggregates many declarative entries;
- a preset function that returns a complete configured component subtree and
  accepts selector overrides.

A reusable humanoid map can follow these conventions instead of placing every
model's strings directly in every AVC scene.

## Automation signals already available

### Names

Names can provide strong evidence when tokenized rather than matched as one
hard-coded convention. Useful signals include:

- anatomical tokens: `head`, `neck`, `chest`, `spine`, `hips`, `hand`, `foot`;
- segment tokens: `upper`, `lower`, `forearm`, `calf`, `distal`, `proximal`;
- side tokens: `left`, `right`, `.L`, `.R`, `_L_`, `_R_`;
- known convention presets such as VRoid or Mixamo.

The currently inspected Bisket and PC-Rei models both use symmetric
`J_Bip_C_*`, `J_Bip_L_*`, and `J_Bip_R_*` names. That is evidence for a useful
VRoid preset, not evidence that generic GLTF names are standardized.

### Topology and rest geometry

The hierarchy and rest positions can corroborate names or fill gaps:

- a central chain from hips through spine/chest/neck/head;
- paired branches from upper torso through shoulder, upper arm, lower arm, and
  hand;
- paired branches from hips through upper leg, lower leg, foot, and toe;
- finger chains branching from a hand;
- segment lengths that distinguish deforming limbs from tiny helper joints.

Topology alone is insufficient for arbitrary rigs. Extra twist bones, multiple
roots, accessories, nonhumanoid characters, and unusual rest poses can all
produce plausible but wrong chains.

### Symmetry

Left/right symmetry should validate a candidate pair, not invent semantics by
itself. Useful checks include:

- paired side-name tokens;
- matching hierarchy depth and chain shape;
- similar segment-length ratios;
- mirrored rest positions around an estimated sagittal plane;
- common central ancestors for the arm and leg pairs.

An automapper should report ambiguity rather than choosing between similarly
scored candidates.

## Hand orientation is related, but not identical

Knowing the hand slot is necessary but is not enough to orient it. Bone names
identify anatomical landmarks; they do not establish portable local axes.

The controller-hand path now obtains a full hand frame from two non-collinear
rest-pose directions:

- the whole middle-finger chain provides finger-forward;
- the projected little-root-to-index-root direction provides palm width/up.

This resolves the otherwise free roll around finger-forward. A humanoid map
that includes hand and finger slots can supply those landmarks automatically,
removing the need to repeat finger bone names on each `XRHand`. A projected
middle-root-to-thumb-root direction remains a less stable fallback when the
knuckle pair is unavailable. The basis math
and its degeneracy rules remain separate from slot discovery.

See
[imported-humanoid-pose-basis-detection-and-conversion.md](./imported-humanoid-pose-basis-detection-and-conversion.md)
for the full axis-conversion analysis.

## Current gaps

1. There is no shared asset/instance-level semantic humanoid map.
2. AVC strings, XR hand landmarks, pose selectors, and other consumers cannot
   reuse one resolved identity for a bone.
3. Resolution is frequently first-match rather than unique and GLTF-scoped.
4. The more capable `BoneMappingSystem` is disconnected from AVC.
5. Missing required AVC mappings can produce indefinite silent initialization
   retries; optional failures have inconsistent diagnostics.
6. There is no provenance, confidence, ambiguity report, or dry-run inventory.
7. There is no explicit global automapping policy because general automapping
   does not yet exist.
8. There is no per-slot authored "intentionally absent" value that prevents a
   future automapper from filling an optional slot.
9. VRM humanoid metadata is not imported.
10. Existing specs do not consistently distinguish implemented code from the
    intended mapping design.

## Architectural conclusion

AVC already has the beginnings of internal slots, but they are fragmented
fields and caches rather than an authoritative map. The semantic map should be
owned by the avatar's GLTF instance so AVC, XR hands, IK, pose tooling, and
future retargeting can share it. AVC may still cache the few resolved IDs it
actively mutates.

The safest resolution order is:

1. explicit authored slot or explicit absence;
2. embedded format semantics, when supported;
3. high-confidence convention/name matching;
4. topology, geometry, and symmetry inference;
5. unresolved or ambiguous, with a report.

Explicit mappings must never be silently overwritten. Automatic inference
should fill only missing `Auto` slots and should be opt-out capable both for the
whole AVC integration and for individual slots. The implementation proposal is
captured in
[humanoid-bone-map-automapping-and-mms-presets.md](../task/humanoid-bone-map-automapping-and-mms-presets.md).

## Code and document entry points

- [AvatarControlComponent](../../src/engine/ecs/component/avatar_control.rs)
- [AvatarControlSystem](../../src/engine/ecs/system/avatar_control_system.rs)
- [BoneMappingSystem](../../src/engine/ecs/system/bone_mapping_system.rs)
- [GLTFComponent](../../src/engine/ecs/component/gltf.rs)
- [GLTFSystem](../../src/engine/ecs/system/gltf_system.rs)
- [BoneRestPoseComponent](../../src/engine/ecs/component/bone_rest_pose.rs)
- [PoseCaptureSystem](../../src/engine/ecs/system/pose_capture_system.rs)
- [SecondaryMotionSystem](../../src/engine/ecs/system/secondary_motion_system.rs)
- [current bone-mapping specification](../spec/bone-mapping-system.md)
- [current avatar-control specification](../spec/avatar-control.md)

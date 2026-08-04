# Avatar hand pose basis

Date: 2026-08-04

Status: implemented for explicitly configured controller-driven avatar hands;
future humanoid bone maps should provide its landmarks

## Why this exists

Imported humanoid armatures do not share dependable bone-local axes. A hand
bone's local `+X`, `+Y`, or `+Z` cannot safely be assumed to mean
finger-forward, palm-up, or palm-normal across GLTF exporters and avatars.

OpenXR poses, by contrast, have semantic frames. In particular, controller Aim
uses local `-Z` as forward. Driving an imported hand directly from that
orientation can therefore point or roll the visible hand incorrectly even when
the correct hand bone was selected.

`avatar_hand_pose_basis.rs` converts anatomical directions measured from the
avatar's immutable rest pose into one explicit canonical hand frame. This
avoids per-avatar Euler angles, calibration quaternions, and assumptions about
the hand bone's authored local axes.

## Relationship to bone mapping

The pose-basis module does not discover anatomy. It should be fed by a humanoid
bone map.

```text
GLTF joint hierarchy
        │
        ▼
bone mapping
  "these joints are left hand, middle, index, and little"
        │
        ▼
avatar hand pose basis
  "these mapped landmarks define finger-forward and palm-up"
        │
        ▼
AVC/controller retargeting
  apply the inverse basis to the tracked semantic pose
```

Bone mapping answers **which bones have which anatomical roles**. Hand-pose
basis construction answers **how those already mapped bones orient the hand**.

The current implementation is transitional: `XRHand` carries explicit
`ComponentRef` selectors for the required landmarks. Once a shared humanoid
bone map exists, it should supply resolved landmark IDs instead. The basis math
should remain independent of whether mapping came from an MMS preset, embedded
avatar metadata, name matching, topology inference, or manual authoring.

## What it does

The module:

- resolves currently configured landmarks uniquely within the owning GLTF
  instance;
- reads immutable `BoneRestPoseComponent` transforms rather than the animated
  runtime pose;
- expresses landmark positions relative to the configured hand bone;
- validates that the middle-finger joints form an ancestral chain beneath that
  hand;
- rejects missing, ambiguous, zero-length, or collinear inputs;
- derives a fingertip position and a canonical-to-avatar hand rotation;
- provides additional rest-pose vectors for gated alignment diagnostics.

The preferred full-palm basis is:

```text
forward = normalize(Middle3 - Middle1)
up_raw  = Index1 - Little1
up      = normalize(up_raw projected perpendicular to forward)
back    = -forward
right   = normalize(up × back)
```

The resulting frame maps:

```text
canonical -Z -> avatar finger-forward
canonical +Y -> avatar little-to-index palm width
canonical +X -> remaining orthogonal palm axis
```

AVC applies the inverse of this frame beneath the raw tracked target. With an
OpenXR Aim orientation, the mapped avatar finger direction then follows Aim
`-Z`, while its little-to-index direction follows Aim `+Y`.

## Fallback levels

The module currently supports three levels of information:

1. **Knuckle basis, preferred:** whole middle-finger direction plus
   `Little1 -> Index1`. This constrains forward and palm roll.
2. **Thumb-root fallback:** final middle-finger direction plus projected
   `Middle1 -> Thumb1`. This constrains roll, but the thumb root may sit too far
   wristward to represent palm width reliably.
3. **Forward-only fallback:** final middle-finger direction alone. This aligns
   pointing but leaves roll determined by a mathematical shortest-arc choice,
   not anatomy.

Callers and diagnostics should preserve which level produced the result.

## What it does not do

The module does not:

- find hands, fingers, or other humanoid bones;
- score names, topology, symmetry, or mapping confidence;
- decide whether a model is humanoid;
- parse VRM/VRMC humanoid metadata;
- select OpenXR Aim, Grip, GripAim, wrist, or palm tracking sources;
- apply the correction to runtime transforms;
- solve arm, wrist, or finger IK;
- animate finger joints;
- spawn pointers or lasers;
- define per-avatar adjustment angles;
- run interactive calibration.

Those responsibilities belong respectively to humanoid bone mapping, XR input
routing, AVC/IK, articulated-hand retargeting, and pointer presentation.

## Current consumers

- `AvatarControlSystem` requests the basis and applies its inverse to the
  controller-driven visual hand target when `ControllerGripAim` is active.
- `PointerSystem` uses the same derived frame for the avatar fingertip mount so
  the visible hand and its pointer do not use competing conversions.
- Alignment diagnostics compare distal/whole middle-finger, thumb-root,
  knuckle-width, and palm-normal directions in Aim-local space.

The pointer is a consumer of the result, not the owner of the conversion.

## Reference case

`examples/vtuber-mirror-example.mms` explicitly maps Bisket's hand and finger
landmarks. Its successful reference basis is:

- `J_Bip_[L/R]_Middle1 -> J_Bip_[L/R]_Middle3` for forward;
- `J_Bip_[L/R]_Little1 -> J_Bip_[L/R]_Index1` for palm up/width.

When controller Aim points forward, diagnostics should report approximately:

```text
whole_middle             = [0, 0, -1]
little_to_index          = [0, 1, 0]
little_to_index_roll_deg = 0
```

This reference validates the basis construction. It does not make the Bisket
bone names a generic mapping convention.

## Related material

- [implementation](../../src/engine/ecs/system/avatar_hand_pose_basis.rs)
- [pose-basis review](../review/imported-humanoid-pose-basis-detection-and-conversion.md)
- [current bone-mapping review](../review/current-humanoid-bone-mapping-and-avatar-slot-resolution.md)
- [humanoid bone-map task](../task/humanoid-bone-map-automapping-and-mms-presets.md)
- [avatar control](avatar-control.md)
- [bone mapping](bone-mapping-system.md)


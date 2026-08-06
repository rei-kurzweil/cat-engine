# Task: Shared humanoid bone map, conservative automapping, and MMS presets

Date: 2026-08-04

Status: proposed; documentation and design only

Related review:

- [current-humanoid-bone-mapping-and-avatar-slot-resolution.md](../review/current-humanoid-bone-mapping-and-avatar-slot-resolution.md)
- [imported-humanoid-pose-basis-detection-and-conversion.md](../review/imported-humanoid-pose-basis-detection-and-conversion.md)

Optional inference draft:

- [Local LLM-assisted humanoid bone mapping](../draft/llm-assisted-humanoid-bone-mapping.md)

## Problem

Mittens currently asks `AvatarControlComponent` and `XRHand` to carry separate
bone-name strings for the same imported avatar. Resolution is local to each
consumer, often uses the first matching name, and cannot expose one coherent
answer to questions such as:

- which imported joint is the left hand?
- was it explicitly mapped or inferred?
- are the left and right limb chains structurally symmetric?
- which finger landmarks define the hand's anatomical forward and up axes?
- should missing slots be inferred, left unresolved, or intentionally disabled?

The existing `BoneMappingSystem` contains useful chain helpers, but it is not
connected to AVC and it is not a persistent semantic map.

## Goal

Add one GLTF-instance-owned humanoid semantic map that can be authored in MMS,
resolved once after GLTF initialization, inspected, and consumed by AVC and
other systems.

The first implementation should:

1. provide stable internal humanoid slots;
2. preserve explicit author intent and offer clear automapping opt-outs;
3. resolve references uniquely within the owning GLTF's skin joints;
4. fill missing slots conservatively using metadata, names, topology, rest
   geometry, and symmetry;
5. expose provenance, confidence, ambiguity, and validation diagnostics;
6. let MMS modules export reusable per-avatar or per-convention maps;
7. use mapped finger landmarks to derive full hand orientation without
   avatar-specific quaternion calibration;
8. migrate AVC without breaking existing scenes.

## Non-goals

This task does not initially require:

- full animation retargeting between arbitrary skeletons;
- driving every finger from OpenXR hand tracking;
- leg IK or a new whole-body solver;
- assuming every GLTF is humanoid;
- guessing through ambiguous rigs;
- replacing pose libraries or secondary motion;
- treating a bone name as a portable local-axis convention.

It also does not depend on an LLM or general AI-harness primitives. Local LLM
inference may later propose mappings for unresolved or ambiguous slots, but it
must consume the same deterministic inventory, preserve explicit decisions,
and pass the same scoped validation described here.

## Proposed ownership model

Introduce an authored component associated with one GLTF instance, tentatively
named `HumanoidBoneMapComponent`. It owns slot references and resolution policy.
The runtime resolver produces a retained `ResolvedHumanoidBoneMap` keyed by the
owning GLTF component.

```text
GLTF instance
  ├── imported joint hierarchy
  └── HumanoidBoneMapComponent
        ├── authored slot references / explicit absences
        ├── mapping policy
        └── runtime resolved slots + report
                         │
                         ├── AVC head/neck/arm setup
                         ├── XR hand basis landmarks
                         ├── future articulated-hand driving
                         ├── pose/retargeting tools
                         └── editor and diagnostics
```

The map belongs to the avatar asset instance rather than AVC because anatomy is
a property shared by all those consumers. AVC should cache only IDs needed by
its active splice and IK lifecycle.

`BoneMappingSystem` can be expanded or refactored into the stateless resolver;
the persistent authored/resolved map should remain a separate concept so the
existing name does not hide ownership.

## Semantic slots

Use a typed enum internally rather than arbitrary consumer-defined strings.
The first useful set is:

- center: hips, spine, chest, upper chest, neck, head;
- left/right arms: shoulder, upper arm, lower arm, hand;
- left/right legs: upper leg, lower leg, foot, toes;
- left/right hand landmarks: thumb proximal/root, index proximal/root, middle
  proximal/root, middle intermediate, middle distal/tip, and little
  proximal/root.

Eye and complete finger slots can be added when a concrete consumer needs
them. Unknown extension slots may be supported separately, but they must not
weaken validation of the standard set.

Each slot needs an authored state, not just `Option<ComponentRef>`:

```text
Unspecified       automatic resolution may fill this slot
Reference(ref)    explicit selector or GUID; never silently replaced
Absent            intentionally unavailable; automatic resolution must not fill
```

Each resolved result should retain:

- resolved `ComponentId`, if any;
- provenance: explicit, embedded metadata, convention preset, name inference,
  or topology/geometry inference;
- confidence/score and supporting evidence;
- validation warnings;
- unresolved versus ambiguous status.

## Mapping policy and opt-out

The new behavior needs a deliberate compatibility and opt-out surface. Use an
AVC consumption policy with these modes:

| Mode | Behavior |
| --- | --- |
| `Legacy` | Do not invoke the new semantic-map automapper for AVC. Preserve current AVC name fields and direct-parent arm fallback. This is the compatibility escape hatch. |
| `ExplicitOnly` | Consume explicit map entries and explicit AVC overrides only. Do not fill missing slots from metadata, names, or topology. |
| `Auto` | Preserve all explicit entries/absences, then fill only unspecified slots using the conservative resolver. |

During rollout, existing scenes should deserialize as `Legacy`. New scenes or
an explicit builder call can opt into `Auto`. After the resolver is validated
against a sufficiently broad model corpus, a separate compatibility decision
can make `Auto` the default; that default change is not implicit in this task.

Per-slot `Absent` is required even in `Auto`, so an avatar with no toes, no
neck, or an intentionally unmanaged hand cannot have a plausible-looking but
unwanted candidate inserted.

Explicit AVC fields should initially have highest precedence and be reported as
legacy explicit overrides. A later migration can remove duplicate fields only
after MMS serialization and existing examples have moved to map presets.

## Resolution pipeline

Resolution must operate within exactly one initialized GLTF instance and should
be atomic from the consumer's perspective.

### 1. Establish the candidate set

- wait for GLTF initialization;
- use `armature_joint_transforms` as the normal candidate set;
- reconstruct hierarchy/depth from ECS parent links;
- read immutable local rest poses and compute positions in a consistent model
  or armature-root space;
- identify one or more candidate skeleton roots without relying on vector order.

If an authored reference points to a spawned non-joint node, reject it by
default and require an explicit escape hatch for a proven use case.

### 2. Resolve explicit authored states

- resolve `ComponentRef::Guid` or query within the owning GLTF;
- require exactly one result;
- retain `Absent` as a tombstone;
- report missing and ambiguous explicit references as configuration errors;
- never substitute an inferred candidate for a broken explicit reference.

### 3. Consume embedded semantics

When VRM/VRMC humanoid metadata support exists, translate it into semantic
slots with high confidence. The current importer does not implement this, so
metadata parsing is a distinct prerequisite or phase, not an assumed input.

### 4. Score names and known conventions

- normalize case and separators without discarding the original name;
- recognize anatomical, segment, and side tokens;
- support named convention profiles such as VRoid and Mixamo;
- penalize helper/twist/collider/secondary-motion tokens;
- do not accept a name result that conflicts strongly with topology or side.

Convention profiles should be data/preset driven where practical. They must
not become hidden AVC special cases.

### 5. Infer and validate topology

- find a central hips-to-head chain;
- find paired torso-to-hand branches for arms;
- find paired hips-to-foot branches for legs;
- skip short helper/twist joints using rest length and branching evidence;
- find finger chains below each mapped hand;
- validate ancestor ordering and prevent one joint from occupying conflicting
  slots.

### 6. Use bilateral symmetry as corroboration

Compare candidate left/right pairs using:

- complementary name tokens;
- hierarchy depth and branch shape;
- segment count and length ratios;
- mirrored rest positions around an estimated sagittal plane;
- shared central ancestors.

Symmetry should raise or lower confidence. It must not force a choice where two
candidate pairs remain plausible.

### 7. Commit or report

Build and validate a candidate result before making AVC topology changes.
Commit only resolved slots meeting the required confidence threshold. Preserve
ambiguous/unresolved slots in the report and let each consumer state which
slots it requires.

## Hand orientation integration

When the map resolves a hand plus middle-finger and proximal index/little landmarks, reuse the
existing anatomical-basis construction:

- whole middle-finger direction -> finger-forward;
- projected little-root-to-index-root -> palm width/up;
- Gram-Schmidt/cross products -> orthonormal full frame.

AVC/XRHand can then request the derived basis from the humanoid map rather than
repeat selectors in scene code. Thumb-root-based and forward-only fallbacks may
remain available when the knuckle landmarks are absent, but diagnostics must
state which weaker basis was used and when palm roll is unconstrained.

This is not an additional calibration angle. It is a conversion derived from
the imported avatar's rest geometry and a documented semantic controller frame.

## MMS preset and import design

Follow the existing pose-library and secondary-motion convention: a normal MMS
module exports a function that returns declarative configuration. One model can
have a precise explicit preset, while a convention module can provide name
defaults plus `Auto` validation.

Illustrative API only:

```mms
// assets/components/humanoid/bisket.mms
export fn bisket_humanoid_map() {
  return HumanoidBoneMap.explicit()
    .bone("hips", "#J_Bip_C_Hips")
    .bone("head", "#J_Bip_C_Head")
    .bone("leftUpperArm", "#J_Bip_L_UpperArm")
    .bone("leftLowerArm", "#J_Bip_L_LowerArm")
    .bone("leftHand", "#J_Bip_L_Hand")
    .bone("leftMiddleProximal", "#J_Bip_L_Middle1")
    .bone("leftMiddleIntermediate", "#J_Bip_L_Middle2")
    .bone("leftMiddleDistal", "#J_Bip_L_Middle3")
    .bone("leftThumbProximal", "#J_Bip_L_Thumb1")
    // mirrored right-side entries and remaining slots
}
```

```mms
import { bisket_humanoid_map } from
  "../assets/components/humanoid/bisket.mms"

T {
  GLTF.new("../assets/models/bisket.11.0.glb") {
    bisket_humanoid_map()
  }
}
```

The exact MMS builder names should follow registry conventions established
during implementation. The important contract is that selectors resolve
against the owning GLTF instance, not the entire world.

Useful library forms are:

- per-asset exact maps for known avatars;
- convention presets with caller overrides;
- an aggregate manifest for editor discovery or generated maps;
- a saved inspection result that can be reviewed and converted to an explicit
  preset rather than recomputed forever.

## Diagnostics and instrumentation

Add a dry-run/inventory report available before AVC mutates the armature. For
each slot, report:

- selected node name and ID;
- authored state and provenance;
- confidence and contributing evidence;
- parent chain and rest-space position;
- left/right partner and symmetry score;
- reasons rejected candidates lost;
- unresolved, ambiguous, invalid, or intentionally absent status.

Also report structural checks:

- duplicate slot assignments;
- broken expected ancestry;
- helper/twist candidates inside deforming chains;
- implausible zero/short segment lengths;
- multiple skeleton roots;
- hand basis availability and degeneracy;
- whether the AVC consumer used `Legacy`, `ExplicitOnly`, or `Auto`.

Logging should be gated and concise by default. An editor/debug table or a
serializable report will be more useful than high-volume per-frame output.
Mapping occurs on GLTF readiness/configuration changes, never as a frame poll.

## Lifecycle

Use the retained lifecycle pattern already established by secondary motion:

1. register a map component when its authored subtree initializes;
2. wait without polling if the owning GLTF is not initialized;
3. resolve on `GltfInitialized`;
4. invalidate only when map configuration, GLTF respawn, or relevant topology
   changes;
5. notify AVC and other consumers when a resolved map version changes;
6. remove retained results when the map or owning GLTF is removed.

The first implementation may use a simpler one-shot initialization path if it
documents limitations, but it must not add whole-world or per-frame rescans.

## Migration plan

### Phase 1: Data model and inspector

- add typed slots, authored states, policy, resolved records, and reports;
- implement unique GLTF-owned joint resolution;
- expose read-only inventory/dry-run output;
- keep AVC behavior unchanged in `Legacy` mode.

### Phase 2: Explicit MMS maps

- register the component and builder methods in MMS;
- add per-asset preset modules for Bisket and PC-Rei;
- add round-trip serialization tests;
- consume explicit hand landmarks for the existing hand-basis calculation;
- leave inference disabled unless explicitly requested.

### Phase 3: Conservative automapping

- implement metadata/name/topology/symmetry scoring;
- integrate or replace the useful `BoneMappingSystem` chain helpers;
- add ambiguity thresholds and diagnostics;
- validate against rigs from more than one naming/export convention.

### Phase 4: AVC consumption

- resolve head, neck, hands, and arm chains from the shared map;
- treat current AVC string fields as highest-precedence legacy overrides;
- initialize splices only after required slots are validated;
- replace silent retry and partial first-match behavior with explicit waiting
  versus invalid states;
- keep `Legacy` as an explicit opt-out.

### Phase 5: Cleanup and broader consumers

- migrate examples away from repeated AVC/XRHand bone strings;
- expose the same map to pose/retargeting tools and future finger/leg systems;
- decide, with migration evidence, whether `Auto` should become the default;
- reconcile the bone-mapping, avatar-control, hand-tracking, and GLTF specs with
  implemented behavior.

## Test matrix

### Resolver unit tests

- explicit unique, missing, and ambiguous selectors;
- explicit `Absent` remains absent in `Auto`;
- explicit entries are never replaced by inference;
- side-token normalization across common naming styles;
- helper/twist joints skipped without losing deform chains;
- duplicate/conflicting slot rejection;
- multiple skeleton roots and nonhumanoid rigs remain unresolved safely.

### Geometry and symmetry tests

- mirrored ordinary T-pose and A-pose rigs;
- asymmetric but valid limb lengths;
- rotated/scaled model roots;
- negative or mirrored transforms;
- extra shoulder, forearm twist, and finger helper joints;
- hands whose local axes differ while rest geometry is anatomically equivalent.

### Integration tests

- Bisket and PC-Rei explicit MMS presets resolve to their owned joint sets;
- two copies of the same GLTF do not cross-resolve names;
- two different avatars with repeated bone names remain instance scoped;
- `Legacy` reproduces current AVC selection;
- `ExplicitOnly` never invokes inference;
- `Auto` fills unspecified slots but honors per-slot absence;
- GLTF readiness/respawn triggers one targeted re-resolution;
- controller Aim forward produces finger-forward with little-to-index/up vertical for
  both hands when full landmarks exist.

### Serialization tests

- all slot states and mapping modes round trip through MMS;
- reusable imported map functions evaluate in a GLTF subtree;
- saved explicit maps preserve selectors rather than runtime IDs;
- runtime provenance/confidence caches are not serialized as authored truth.

## Acceptance criteria

This task is complete when:

- one authoritative humanoid map can be attached to and resolved within a GLTF
  instance;
- explicit MMS presets can be imported and reused like pose or secondary-motion
  assets;
- `Legacy`, `ExplicitOnly`, `Auto`, and per-slot absence have tested semantics;
- automapping never replaces an explicit decision and refuses ambiguous cases;
- AVC can consume mapped head/neck/hand/arm slots without first-world-match
  lookup;
- XR hand orientation can obtain mapped finger landmarks and derive both
  forward and roll without per-avatar rotation constants;
- mapping reports explain every selected, unresolved, absent, and ambiguous
  slot;
- no mapping scan or retry runs every frame;
- affected specifications are updated to distinguish current implementation
  from remaining future work.

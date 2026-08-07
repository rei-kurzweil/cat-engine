# Draft: Extending humanoid bone mapping beyond human anatomy

Date: 2026-08-06

Status: exploratory draft; no runtime or serialization API is committed by this document

Related work:

- [Shared humanoid bone map, conservative automapping, and MMS presets](../task/humanoid-bone-map-automapping-and-mms-presets.md)
- [Future humanoid bone-mapping editor](../task/humanoid-bone-mapping-editor.md)
- [Local LLM-assisted humanoid bone mapping](llm-assisted-humanoid-bone-mapping.md)

## Motivation

`HumanoidBoneMap` deliberately describes a stable human-shaped control core: head, torso, eyes,
arms, hands, legs, feet, and a few hand landmarks. Many avatars retain that core while adding
meaningful skeletal anatomy such as animal ears, one or more tails, a muzzle or jaw, horns, wings,
or digitigrade legs.

Those joints should not be treated as arbitrary leftovers. They may need editor selection,
secondary motion, procedural posing, gaze and expression control, animation retargeting, or
attachment points. At the same time, expanding the fixed `HumanoidSlot` enum with every possible
creature feature would make the human control contract unstable and still fail to describe novel
rigs.

This draft uses **transhumanoid bone map** as a working name for a humanoid map plus optional
anatomical extensions. The eventual public name may instead be `AvatarBoneMap`,
`AnatomicalLandmarkMap`, or another term that does not imply a species classification.

## Preserve mixed-convention automapping

The existing mapper accepts compatible evidence from several naming styles on the same armature.
That is desirable. Real models are renamed, merged, passed through Blender, or exported by a tool
that preserves only part of the original convention. Automapping should not require the entire rig
to be classified as VRoid, Mixamo, or any other single convention before individual landmarks can
resolve.

Convention knowledge is therefore best understood as composable name and topology evidence, not
as mutually exclusive whole-rig modes. A joint may match a VRoid-style core name while an added
tail follows a creator-specific or generic naming convention. Explicit mappings remain
authoritative regardless of which evidence would otherwise match.

## Profiles are capability bundles, not identity inference

Terms such as nekomimi, catgirl, furry, dragon, or bird are useful when discussing expected feature
sets, but the engine should not infer a character's identity or species label. It should report
anatomical capabilities that were explicitly authored or conservatively detected.

For initial design examples:

- a **nekomimi** profile is a humanoid core plus paired animal-ear structures;
- a **catgirl** profile is a subset of nekomimi avatars that also requires a tail structure;
- a broader furry or anthropomorphic avatar may add a tail, paired ears, muzzle/jaw landmarks,
  digitigrade leg landmarks, wings, horns, or other extensions in any combination.

These profile names can be optional authoring conveniences or validation bundles. They must not be
the storage schema. An avatar should be representable directly as a set of capabilities such as
`humanoid + paired_ears + tail`, including unusual combinations and intentionally absent features.

## Proposed layered model

Keep `HumanoidBoneMap` as the compatibility and control foundation, then attach zero or more typed
extension maps owned by the same GLTF instance.

```text
GLTF instance
  |
  +-- HumanoidBoneMap / retained humanoid report
  |     head, torso, arms, hands, eyes, legs, camera anchor
  |
  +-- optional anatomical extension reports
        paired ears
        tail collection
        muzzle and jaw
        digitigrade leg landmarks
        wings
        horns or antlers
        future/custom extension namespaces
```

This can eventually be exposed as one composite `AvatarBoneMapReport`, but consumers should be
able to request only the capabilities they understand. Avatar control must not stop working merely
because an optional ear or tail mapping is missing or ambiguous.

The first implementation should prefer typed extension structures over a single ever-growing enum.
Possible conceptual shapes are:

```text
PairedEarMap
  left:  appendage structure
  right: appendage structure

TailMap
  tails: one or more appendage structures

AppendageStructure
  root
  ordered deforming joint chain
  optional tip
```

These are design shapes, not proposed Rust declarations. Variable-length ordered chains are more
appropriate for tails, ears, tentacles, and similar appendages than fixed slots such as `tail_1`
through `tail_8`. Collections also allow twin tails or other repeated anatomy without changing the
schema.

## Candidate extension vocabulary

The vocabulary should grow from concrete consumers rather than attempting to catalogue every
creature in advance.

| Capability | Initial semantic structure | Likely consumers |
| --- | --- | --- |
| Paired animal ears | left/right roots and optional ordered chains | ear posing, expression, secondary motion |
| Tail | root plus ordered chain; support multiple tails | secondary motion, procedural posing, attachments |
| Muzzle and jaw | muzzle anchor, jaw, optional nose | facial posing, lip sync, attachments |
| Digitigrade legs | existing humanoid leg chain plus hock/ankle/foot landmarks | IK and locomotion retargeting |
| Wings | paired roots plus ordered or branched structures | procedural control, animation retargeting |
| Horns/antlers | one or more rooted structures | attachments, editor inspection |

Not every visible feature has a bone. A rigid ear, horn, or muzzle may be represented only by a
transform, be part of the head mesh, or be driven by morph targets. Each extension must state
whether it requires a skin joint, permits an arbitrary transform, or needs a different semantic
system entirely. The bone map should not invent joints for mesh-only features.

## Automapping evidence

Extension automapping should reuse the current conservative approach while adding
extension-specific evidence:

1. exact and tokenized names from any compatible naming convention;
2. left/right or repeated-instance evidence where names establish it;
3. hierarchy and ordered-chain topology;
4. rest geometry relative to an already resolved humanoid core;
5. embedded metadata when a format supplies appropriate semantics;
6. explicit authored references, which always take precedence.

Examples of useful name tokens include `ear`, `cat_ear`, `kemono_ear`, `tail`, `tail_base`, `jaw`,
`muzzle`, `wing`, and `horn`. These examples are not sufficient by themselves: hair, clothing,
accessories, and physics rigs can contain similar names and topology.

The matcher should continue allowing mixed naming styles. Evidence should carry its source and
confidence per resolved structure rather than assigning one convention label to the entire model.

### Secondary-motion bones require different filtering

The humanoid core currently rejects helper-like tokens including `spring` and `secondary` during
name inference. That policy cannot be applied globally to anatomical extensions. Ear and tail
joints are frequently themselves spring or secondary-motion joints, or have such joints interposed
in their deforming chains.

Candidate filtering must therefore be semantic-specific:

- core arm and leg resolution may reject twist, collider, spring, and secondary helpers;
- ear and tail resolution may accept spring or secondary deforming joints;
- collider-only and non-transform control nodes should still be excluded;
- reports should distinguish the anatomical chain from associated physics helpers when possible.

## Explicit authoring and saved maps

The authored merge rule should remain consistent with `HumanoidBoneMap`:

- explicit references and explicit absence are authoritative;
- automapping fills only unspecified structures;
- ambiguous inference remains unresolved and reports its candidates;
- a reviewed result can be exported as MMS without serializing runtime-only component IDs;
- maps may contain a humanoid core, extension maps, or both.

A per-avatar asset might therefore explicitly correct one ear root and a tail chain while allowing
the ordinary humanoid slots and the other ear to automap. A reusable convention asset may supply
known names, but it should not be required for internal detection.

Variable-length chains need an authored representation that is stable across GLTF instances. An
MMS-facing sketch might eventually resemble:

```mms
TranshumanoidBoneMap.new()
  .ear("left", [name="Ear_L"])
  .ear("right", [name="Ear_R"])
  .tail_chain([
    [name="Tail_01"],
    [name="Tail_02"],
    [name="Tail_03"],
  ])
```

This syntax is illustrative only. It does not commit to a new component, builder, selector-array
syntax, or the working `TranshumanoidBoneMap` name.

## Validation and diagnostics

Extension reports should retain the same qualities as the humanoid report: provenance, ambiguity,
explicit errors, generation, and GLTF ownership. Validation should be local to each capability.

Initial validation rules could include:

- every mapped deforming joint belongs to the owning GLTF's skin-joint set;
- every ordered chain follows transform ancestry without cycles;
- left and right ears have distinct roots and plausibly share a head-region ancestor;
- a tail root is attached at or below the resolved hips region, unless explicitly overridden;
- a tail chain does not silently consume leg, spine, clothing, or collider joints;
- repeated appendages remain separate structures rather than one concatenated chain;
- failure of an optional extension never invalidates an otherwise usable humanoid core.

Geometry may corroborate a named ear or tail, but it should not infer species, gender, or character
identity. An explicit unconventional mapping should be accepted when structurally valid and marked
as explicit rather than rejected for violating a heuristic expectation.

## Editor implications

The future bone-mapping editor should present the humanoid core and detected extensions separately.
It should support:

- adding an extension capability without selecting a species label;
- reviewing and reordering variable-length chains;
- splitting accidentally merged repeated appendages;
- marking a feature absent or intentionally mesh-only;
- showing name, topology, geometry, metadata, and symmetry evidence;
- exporting only authored decisions, not transient inference results;
- optionally applying a named profile as a validation checklist.

A catgirl checklist could require the humanoid core, paired animal ears, and at least one tail. A
nekomimi checklist could require only the humanoid core and paired animal ears. These checks should
describe expected capabilities, not alter the underlying map format.

## Incremental path

1. Keep the current `HumanoidBoneMap` and mixed-convention automapper unchanged while its new
   behavior is reviewed.
2. Collect real VRoid, Mixamo-derived, nekomimi, catgirl, and furry armature inventories, including
   hybrid and renamed rigs.
3. Prototype a generic ordered-appendage report without connecting it to runtime pose control.
4. Add paired-ear and tail schemas as the first two extensions, with explicit authoring and
   diagnostics before broad automapping.
5. Add conservative name/topology inference and tests for false matches against hair, clothing,
   colliders, and secondary-motion helpers.
6. Integrate extension review and MMS export into the future mapping editor.
7. Add consumers such as secondary motion or procedural posing only after the semantic contract is
   stable.

## Open questions

- Should the composite public concept be called `TranshumanoidBoneMap`, `AvatarBoneMap`, or should
  extension maps remain independent components?
- Is one generic ordered-appendage schema sufficient for ears and tails, or do their validation and
  runtime needs justify distinct types?
- How should branched structures such as wings and antlers be represented without turning the map
  into a duplicate skeleton graph?
- Should arbitrary transforms be permitted for rigid extension anchors, as they are for the camera
  anchor, or should bone and attachment maps remain distinct?
- How should embedded VRM/VRMC secondary-animation metadata contribute evidence without conflating
  physics groups with anatomical meaning?
- Which extension must be proven by a current engine consumer before becoming a stable public slot
  or schema?


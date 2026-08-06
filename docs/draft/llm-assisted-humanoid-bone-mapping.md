# Draft: Local LLM-assisted humanoid bone mapping

Date: 2026-08-06

Status: draft; no runtime API is committed by this document

Related work:

- [Shared humanoid bone map, conservative automapping, and MMS presets](../task/humanoid-bone-map-automapping-and-mms-presets.md)
- [Current humanoid bone mapping and avatar slot resolution](../review/current-humanoid-bone-mapping-and-avatar-slot-resolution.md)
- [Joint retargeting, rest attachment, and XR hand alignment](../review/joint-retargeting-rest-attachment-and-xr-hand-alignment.md)
- [HTTP / Router / MCP roadmap](http-engine-api-roadmap.md)

## Purpose

Explore a small, local, OpenAI-compatible LLM as an optional assistant for
identifying humanoid joints in unfamiliar armatures. The first useful workflow
should find likely head, neck, arm, hand, and hand-landmark joints and produce a
reviewable `HumanoidBoneMap` proposal.

The model is not the authoritative mapper and does not drive transforms. It is
one evidence source above a deterministic armature inventory and below the
same scoped resolver and validator used by authored MMS maps.

The initial target is deliberately modest:

- run against a server on localhost;
- use `POST /v1/chat/completions` with an OpenAI-style JSON body;
- send the request through Mittens' existing `HttpClient` component and system;
- work on an 8k-context model, with 16k also supported, while keeping the
  complete prompt and response below roughly 4k tokens;
- use one non-streaming request per mapping job;
- return structured JSON which is validated before it becomes a proposal;
- let a person inspect and save the result as explicit authored configuration.

This is a focused mapping feature, not yet a general in-engine agent framework.

## Separation of concerns

These are related but distinct operations:

| Concern | Responsibility |
| --- | --- |
| Armature inventory | Describe joints, hierarchy, names, and rest geometry without guessing semantics. |
| Humanoid mapping | Assign semantic slots such as `leftHand` and `head` to imported joints. |
| Basis detection | Use mapped landmark joints to construct canonical anatomical bases. |
| Pose driving | Apply tracked or authored poses through AVC, PointerSystem, pose drivers, or future consumers. |
| LLM assistance | Propose semantic slot assignments when deterministic evidence is incomplete or awkward. |

The LLM should not calculate live transforms, choose controller poses, or
replace `JointBasisRetargetingSystem`. Once finger landmarks are mapped, the
existing deterministic retarget-basis path remains responsible for geometry.

## Proposed first architecture

```text
initialized GLTF instance
        |
        v
deterministic ArmatureInventory
  - inventory-local joint keys
  - names and hierarchy
  - compact rest-space geometry
  - deterministic candidates/evidence
        |
        v
bounded OpenAI chat-completions request
        |
   HttpClient component
        |
        v
localhost inference server
        |
        v
structured mapping proposal
        |
        v
schema + GLTF-scope + topology validation
        |
        +---- invalid/ambiguous ---> diagnostic report
        |
        v
human review -> explicit HumanoidBoneMap MMS/preset
```

The first implementation can coordinate this flow from MMS or one narrow
mapping system. It should reuse `HttpClientComponent` as the transport instead
of adding provider-specific sockets or linking a model runtime into Mittens.

The current lower-level HTTP request intent already supports method, headers,
URL, and text body. The MMS `HttpClient.post(url, body_text)` surface does not
currently expose headers. Before the authored workflow sends JSON, add a small
general HTTP-client capability such as `post_json` or a request method that
accepts headers. That addition belongs to `HttpClient`; the humanoid mapper
should not own HTTP protocol behavior.

## Local OpenAI-style protocol

The default endpoint should be an explicitly configured loopback URL, for
example:

```text
http://127.0.0.1:8080/v1/chat/completions
```

Initial request shape:

```json
{
  "model": "local-model",
  "messages": [
    {
      "role": "system",
      "content": "You map an imported armature to the supplied humanoid schema. Return JSON only. Never invent a joint key."
    },
    {
      "role": "user",
      "content": "<compact mapping task and armature inventory>"
    }
  ],
  "temperature": 0.1,
  "max_tokens": 700,
  "stream": false
}
```

The response reader should consume
`choices[0].message.content`. If a local server supports a compatible
structured-output or JSON-schema option, the request may use it, but the first
version must not require that optional extension. Prompted JSON plus strict
local validation is the portable baseline.

Streaming is unnecessary for a mapping job. The current `HttpClient` reads a
complete response body, which fits the initial non-streaming contract.

Authentication is optional for a loopback-only server. The transport design
should still permit headers so the same component remains generally useful.
API keys must never be embedded in generated MMS presets or diagnostic output.

## Context budget

Eight thousand tokens is the model-context floor, not the target payload size.
The complete exchange should normally remain below 4,000 tokens, including
system instructions, slot schema, armature data, deterministic evidence, and
the model's answer. This leaves substantial headroom for tokenizer differences
and provider-added formatting on an 8k model. A 16k model should receive the
same compact request by default rather than automatically filling the larger
window.

The system prompt has a hard ceiling of 2,000 tokens and should normally be
much smaller. Mapping facts belong in the per-job user payload rather than
being repeated as general system prose.

A practical target budget is:

| Content | Target maximum |
| --- | ---: |
| System instructions | 900 tokens normally; 2,000 hard maximum |
| Task, slot list, and output schema | 450 tokens |
| Armature inventory and deterministic evidence | 1,400 tokens |
| Model output | 700 tokens |
| Typical complete exchange | 3,450 tokens |

The remaining space below 4k is a working margin, not permission to add prompt
boilerplate. If the system prompt approaches its 2k ceiling, the inventory or
output budget must shrink so the whole exchange still remains below 4k.

Token counts vary by model. Mittens should enforce a conservative serialized
character or byte budget and, where the server exposes tokenization, may also
use its token count. Oversized armatures should be reduced deterministically;
truncating arbitrary JSON at the context boundary is invalid.

### Compact inventory format

Do not send full matrices, ECS dumps, UUID prose, component trees, or mesh
vertices. A useful joint record needs approximately:

```json
{
  "key": 17,
  "name": "J_Bip_L_Hand",
  "parent": 12,
  "depth": 5,
  "rest": [-0.42, 1.21, 0.03],
  "length": 0.14,
  "children": [18, 23, 28, 33, 38]
}
```

`key` is an inventory-local opaque identifier. It is meaningful only for this
request and is resolved back to a `ComponentId` by trusted code. Rest positions
should use one documented GLTF/model space, rounded to enough precision for
relative geometry. Original joint names must be retained even if normalized
tokens are also supplied.

The inventory can also include compact deterministic evidence:

- normalized side and anatomy tokens;
- likely central and bilateral branches;
- ancestor/descendant constraints;
- approximate mirrored partner;
- name/convention scores;
- helper, twist, collider, or secondary-motion penalties.

This lets a small model arbitrate among plausible candidates instead of asking
it to rediscover every geometric fact from raw data.

### Oversized armatures

Reduction should proceed in this order:

1. remove non-joint imported nodes from the candidate payload;
2. collapse verbose evidence into numeric scores and short reason codes;
3. retain central-chain and bilateral branch candidates plus their ancestry;
4. exclude clearly unrelated secondary/collider branches while reporting that
   they were excluded;
5. split the task by body region only if it still does not fit.

A regional split must share the same inventory key table and a compact central
chain summary. A deterministic reconciliation pass then rejects duplicate or
topologically inconsistent assignments. The first vertical slice may instead
refuse oversized inputs with a useful diagnostic.

## Output contract

The response is a proposal, not a serialized runtime cache. A minimal shape is:

```json
{
  "schema_version": 1,
  "assignments": [
    {
      "slot": "leftHand",
      "joint_key": 17,
      "confidence": 0.97,
      "evidence": ["name:left+hand", "chain:below-left-lower-arm"]
    },
    {
      "slot": "leftMiddleProximal",
      "joint_key": 28,
      "confidence": 0.91,
      "evidence": ["name:middle1", "topology:finger-branch"]
    }
  ],
  "unresolved": [
    {
      "slot": "neck",
      "reason": "two plausible joints"
    }
  ],
  "warnings": []
}
```

Only known typed slots and supplied joint keys are accepted. Confidence is a
model claim, not trusted probability. Free-form evidence is diagnostic text
and never an instruction to the engine.

The validator must enforce at least:

- valid JSON and supported schema version;
- known slot names and inventory-local joint keys;
- exactly the same owning GLTF instance;
- no prohibited duplicate slot assignments;
- expected ancestry and left/right consistency;
- explicit authored assignments and `Absent` states are never overridden;
- required landmark relationships are geometrically usable;
- ambiguity thresholds and consumer requirements are applied by deterministic
  policy, not by the model.

Malformed output may receive one bounded repair request containing the schema,
validation errors, and the previous answer. It must not enter an unbounded
agent loop. A semantically invalid answer should normally become a report for
review rather than be repeatedly coaxed into passing.

## Relationship to deterministic automapping

The non-LLM mapper remains required. Its precedence should be:

1. explicit authored references and explicit absence;
2. imported semantic metadata such as VRM/VRMC human-bone assignments;
3. exact convention presets and high-confidence deterministic matches;
4. deterministic topology, rest geometry, and bilateral symmetry;
5. optional LLM proposal for unresolved or ambiguous slots;
6. unresolved, with diagnostics.

An alternative review mode may ask the model to assess a complete
deterministic proposal, but it still cannot override explicit configuration or
bypass validation.

Regex is part of deterministic name evidence, not a competing end-to-end
mapper. Patterns should operate on normalized tokens, contribute scored
evidence, and be checked against hierarchy and side. An LLM is most useful for
unusual abbreviations, mixed conventions, and explaining why several
candidates remain plausible.

The saved artifact should be an explicit MMS map or preset using stable
selectors/references. It should not be a cached model response containing
ephemeral runtime component IDs. Therefore inference is an import, inspection,
or authoring-time operation, not something required every time an avatar loads.

## Lifecycle and status

Mapping inference is event-driven:

1. wait for the owning GLTF to initialize;
2. capture an immutable inventory and its generation;
3. issue at most one active request for that mapping source and generation;
4. receive `HttpResponse` or `HttpError` from `HttpClient`;
5. discard a response if the GLTF, mapping configuration, or inventory
   generation changed while it was in flight;
6. parse and validate the proposal;
7. publish a reviewable report, not live joint manipulation.

Suggested conceptual statuses are `WaitingForGltf`, `Ready`, `Requesting`,
`Proposed`, `InvalidResponse`, `TransportError`, `Stale`, and `Cancelled`.
Exact type and component names should be chosen with the deterministic
`HumanoidBoneMap` implementation rather than committed here.

Inference must never run in the render/update loop, retry continuously, or be
triggered by ordinary controller motion. Timeouts and retry counts must be
bounded and visible.

## Security and authority

The first version should default to loopback endpoints (`127.0.0.1` or `::1`)
and require an explicit opt-in for any remote host. The model receives only the
armature inventory and mapping task; it does not need arbitrary files, world
mutation, shell access, or general engine tools.

Model text is untrusted data. It may nominate supplied keys and provide
explanations, but it cannot emit MMS to execute, construct arbitrary component
references, invoke intents, or select a joint outside the owning GLTF.

Reports should record:

- endpoint identity without secrets;
- requested model name;
- inventory generation and content hash;
- prompt/schema version;
- response hash and validation result;
- accepted/rejected assignments and reasons;
- whether a human saved an explicit map from the proposal.

This provenance makes experiments reproducible without treating the model
answer as permanent authored truth.

## Initial vertical slice

The first implementation should remain small:

1. implement the deterministic armature inventory/report required by the main
   humanoid-map task;
2. add general JSON/header request support to the MMS `HttpClient` surface;
3. configure one loopback OpenAI-style endpoint and model name;
4. request a non-streaming proposal for head, neck, left/right upper arm,
   lower arm, hand, and the finger landmarks required for hand bases;
5. parse and validate one versioned JSON response shape;
6. show the proposal and diagnostics without automatically changing the live
   avatar;
7. provide an explicit action to export or copy an MMS map for review.

This slice is successful if a local 8k-context model can inspect compact
Bisket and PC-Rei inventories in a sub-4k exchange, produce valid proposals,
and fail clearly on an ambiguous or nonhumanoid fixture. A 16k model should be
tested for compatibility, but the workflow must not rely on using its extra
context.

## Deferred general AI-harness work

A future Mittens AI-harness epic may introduce reusable execution units,
provider configuration, context assembly and budgeting, structured-output
repair, tool registries, nested workflows, cancellation, tracing, and replay.
Those primitives could eventually host this mapper, but the mapper should not
wait for them and should not prematurely define their universal API.

Likewise, MCP is not required for the local inference call. `HttpClient` is the
transport from Mittens to the OpenAI-style provider. MCP may later expose a
curated mapping or inspection operation to external assistants, which is a
different boundary.

## Open questions

- Should the first coordinator be an MMS-authored workflow, a method on the
  future humanoid-map component, or a small dedicated inference component?
- Which compact inventory encoding performs best within the sub-4k exchange
  budget: JSON records, a columnar text table, or JSON plus short reason codes?
- Should a proposal be able to fill only unresolved slots, or also challenge
  low-confidence deterministic assignments in a review-only report?
- What selector form should exported presets use when names are duplicated but
  an inventory-local hierarchy path is stable?
- Is one repair request useful for the local model, or is strict failure more
  predictable for the first slice?
- Which fixture corpus and expected maps are sufficient before enabling the
  feature outside a developer tool?

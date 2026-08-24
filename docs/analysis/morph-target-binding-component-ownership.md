# Analysis: ownership of `MorphTargetBindingComponent`

Status: open design question. This document records the current implementation
and the decision criteria; it does not prescribe a source change.

## Current behavior

`MorphTargetBindingComponent` is not authored in MMS and is absent from
`examples/vtuber-eye-tracking-mirror.mms`. `GLTFSystem` creates it at runtime as
a child of every spawned target-bearing `RenderableComponent`.

```text
GLTFComponent instance
└── imported node TransformComponent
    └── RenderableComponent
        └── MorphTargetBindingComponent
            { gltf, node_index, primitive_index }
```

Trace: [`gltf_system.rs:1077`](../../src/engine/ecs/system/gltf_system.rs#L1077).
The only current consumer is `RenderableSystem`: it discovers the child binding,
uses its GLTF/node/primitive tuple to filter the owning GLTF instance's factors,
and writes the sparse primitive-local result to `VisualWorld`.
Trace: [`renderable_system.rs:1390`](../../src/engine/ecs/system/renderable_system.rs#L1390).

## Why it is in the component graph

- It is per-renderable data: two instances can share geometry yet need different
  GLTF-owned factor state.
- Its ECS-subtree lifetime is naturally correct. Deleting an imported subtree
  removes the renderable and its association together.
- The lookup is local and inspectable: the consumer starts from a renderable and
  finds its association without a separate cross-system `ComponentId` map.
- It resembles `SkinnedMeshComponent`, another imported-renderable sidecar used
  by a system that scans the world. See
  [`skinned_mesh.rs:1`](../../src/engine/ecs/component/skinned_mesh.rs#L1) and
  [`skinned_mesh_system.rs:339`](../../src/engine/ecs/system/skinned_mesh_system.rs#L339).

## Why it feels wrong as a public component

- It is import bookkeeping, not authored scene intent or a user-facing feature.
- No user should choose its values: only the importer knows the correct GLTF
  instance/node/primitive tuple.
- It currently implements `to_mms_ast()` as `MorphTargetBinding.new()`, but
  `MorphTargetBinding` is not registered in the MMS component registry. If an
  imported subtree were serialized, that AST would not round-trip as authored
  MMS. The usual import path marks spawned node subtrees serialization-off, but
  that does not make the public-looking `Component` API conceptually sound.
- A normal component export in `component/mod.rs` makes it appear equivalent to
  authorable domain components such as `MorphTargetMapComponent`.

## Comparison: `SkinnedMeshComponent`

`SkinnedMeshComponent` is not merely runtime state today. It is a public MMS
component with a `SkinnedMesh.new(skin_index)` constructor, a registry entry,
runtime-config schema, guide entry, and a MMS round-trip test.

| Question | `SkinnedMeshComponent` | `MorphTargetBindingComponent` |
| --- | --- | --- |
| Created by GLTF import | Yes | Yes |
| MMS constructor exists | Yes: `SkinnedMesh.new(skin_index)` | No |
| MMS registry/config exists | Yes | No |
| Public value that can be authored | `skin_index` | None |
| Runtime-only value | `skin_id: Option<SkinId>` | All fields |
| Current system consumer | `SkinnedMeshSystem` | `RenderableSystem` |

Traces: [`skinned_mesh.rs:11`](../../src/engine/ecs/component/skinned_mesh.rs#L11),
[`component_registry.rs:2175`](../../src/scripting/component_registry.rs#L2175),
[`runtime_config.rs:974`](../../src/scripting/runtime_config.rs#L974), and
[`tests.rs:6726`](../../src/scripting/tests.rs#L6726).

`SkinnedMesh` exposes only construction with `skin_index`; there are currently
no MMS builder methods, script getters/setters, or runtime mutation methods for
either `skin_index` or `skin_id`. Thus it is *authorable/serializable metadata*,
but not presently a rich script-control surface.

## Options to decide later

### A. Retain an ECS sidecar, make it explicitly internal/runtime-only

Keep the useful local ownership/lifetime behavior, but remove authoring signals:
make the type crate-private or place it under an import/runtime module, remove
MMS AST generation, and avoid public re-export. A name such as
`ImportedMorphPrimitiveBinding` would clarify its role.

### B. Store the association in a system-owned runtime table

For example, `RenderableSystem` or `GLTFSystem` could own
`RenderableComponentId -> { GLTFComponentId, node, primitive }`.

This removes the internal association from the global ECS graph, but adds explicit
registration/removal and makes inspection/debugging less local. It is attractive
only if the project treats the component graph as authored/domain data rather
than as a home for lifecycle-bound runtime sidecars.

### C. Promote it to an authored morph-routing component

This would require a real use case: an MMS constructor, valid serialization,
clear user ownership of the values, documentation, and likely script APIs. It
would let users manually reroute a renderable to GLTF morph state, but it risks
invalid references and duplicates the importer’s structural knowledge.

## Decision questions

1. Is the component graph intended to contain lifecycle-bound runtime sidecars,
   or only authored/domain-level state?
2. Is manually associating an arbitrary renderable with a GLTF primitive a real
   supported workflow, or should only the importer establish that relation?
3. Should `SkinnedMesh` remain an authorable escape hatch, or should it follow
   the same internal-sidecar policy if no manual skin-index workflow exists?
4. If script control is desired, what meaningful operations are safe? For skin,
   a plausible public control is enable/disable or explicit skin selection; for
   morph routing, higher-level semantic factor controls are likely safer than
   exposing raw node/primitive indices.

No source code should change until those questions have an intended API answer.

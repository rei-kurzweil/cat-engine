# Task: internalize imported skin and morph primitive bindings

Status: analysis and design task; do not change implementation until ownership
and retrieval strategy are decided.

Related:

- [Morph target lifecycle review](../review/morph-target-lifecycle.md)
- [Morph binding ownership analysis](../analysis/morph-target-binding-component-ownership.md)
- [Skinned mesh system specification](../spec/skinned-mesh-system.md)

## Problem

`SkinnedMeshComponent` and `MorphTargetBindingComponent` currently represent
relationships created by glTF import and consumed by rendering systems:

- `SkinnedMeshComponent` relates an imported renderable to a glTF skin.
- `MorphTargetBindingComponent` relates an imported renderable to a GLTF
  instance plus its source node/primitive.

Both are children in the global ECS component graph. That aids discovery but
makes importer bookkeeping look like authored or directly editable domain state.

Desired policy:

> Components normally represent authored state or a tangible, supported runtime
> surface: editing them has defined system/intent effects. Import-only relations
> that users cannot safely create or manipulate should default to system-owned
> runtime state.

This task decides whether both bindings should become system state, and how to
do that without worse lookup or lifecycle costs.

## Current shapes and retrieval cost

### Morph binding

GLTF import adds one `MorphTargetBindingComponent` to each target-bearing
renderable. `RenderableSystem::tick` visits every registered renderable, scans
its children for the binding, reads the bound GLTF factor map, filters factors to
the node/primitive, and updates `VisualWorld`.

```text
per tick: every registered renderable
  -> scan children for MorphTargetBindingComponent
  -> if found, read/filter factor map
```

Trace: [`gltf_system.rs:1077`](../../src/engine/ecs/system/gltf_system.rs#L1077)
and [`renderable_system.rs:1390`](../../src/engine/ecs/system/renderable_system.rs#L1390).

Steady-state work is proportional to all registered renderables plus their
immediate-child scans, even though only target-bearing renderables need factor
work. `VisualWorld::set_active_morphs` prevents downstream invalidation when the
sparse factor vector is unchanged.

### Skin binding

GLTF import adds one `SkinnedMeshComponent` under each skinned renderable and
resolves its runtime `SkinId`. `SkinnedMeshSystem` scans all world components for
`SkinnedMeshComponent` and rebuilds binding/reverse indexes every tick; only its
later palette work is limited to dirty bindings.

```text
per tick: all world components
  -> collect SkinnedMeshComponent ids
  -> rebuild binding and reverse indexes
  -> update only dirty bindings
```

Trace: [`gltf_system.rs:1086`](../../src/engine/ecs/system/gltf_system.rs#L1086),
[`skinned_mesh_system.rs:229`](../../src/engine/ecs/system/skinned_mesh_system.rs#L229),
and [`skinned_mesh_system.rs:339`](../../src/engine/ecs/system/skinned_mesh_system.rs#L339).

## Candidate representations

### A. System-owned imported-binding records

Prefer dense, append-only runtime record vectors owned by the consuming systems.
Records carry component IDs as payload; normal steady-state retrieval iterates
the relevant vector and does not hash to rediscover a relationship:

```text
MorphBinding { renderable, gltf, node_index, primitive_index }
SkinBinding  { renderable, mesh_transform, gltf, skin_id }
```

GLTF import explicitly appends records. Teardown tombstones/removes records when
an imported subtree/renderable is removed. The consumer then iterates only its
own compact active-record list.

Expected cost:

- morph: direct `O(number of morph-bearing renderables)` iteration, no child scan;
- skin: no world-wide component scan or unchanged-frame index rebuild; retain
  direct reverse indexes for dirty binding updates.

Main risk: lifecycle maintenance. Register, unregister, reparent, reload,
failed/partial import, and replacement paths must not leave stale IDs. A sparse
index may still be justified for skin transform invalidation, but it must be a
targeted reverse index, not the primary relationship store or a per-frame
reconstruction mechanism.

### B. Runtime association inside `RenderableSystem`

Store dense imported morph records beside existing registered/pending renderable
state. This offers direct access at visual registration/removal and comparable
lookup cost to A. Do not make `RenderableSystem` own skin-joint policy that
belongs in `SkinnedMeshSystem`.

### C. Renderer or `VisualWorld` metadata

Copy associations onto a `VisualInstance` once one exists. Rendering lookup can
be direct, but this is insufficient alone: factor drivers and joint resolution
originate in ECS and must find their targets before/without GPU registration. It
also makes GLTF provenance a renderer-domain concern.

### D. Retain ECS sidecars but make them internal

Keep local subtree ownership and automatic lifetime, while removing authoring and
serialization signals. This is lowest migration risk, but retains scans and still
places import-only relations in the global component graph.

## Required analysis before choosing

### Ownership and lifecycle

- Identify authoritative creation/removal events for imported renderables.
- Specify cleanup for GLTF deletion, asset reload, failed import, detach/reparent,
  and removal before visual registration.
- Preserve multiple GLTF instances sharing a source mesh while retaining separate
  factors, joint transforms, and palettes.
- If editor/scripts need provenance, expose a diagnostic query rather than a
  mutable authoring component by default.

### API boundary

- Decide whether manually selecting a different glTF skin is a real workflow.
  If yes, design a deliberate high-level override plus dirty/rebind contract.
- Decide whether arbitrary renderable → `(GLTF, node, primitive)` routing is a
  supported workflow. If not, it must remain internal.
- Define valid targets, failures, and downstream effects before exposing MMS or
  script getters/setters.

### Cost measurement

Capture desktop and XR scenes with many static renderables, target-bearing
primitives, and multiple skinned GLTF instances. Measure:

- world components scanned per frame by `SkinnedMeshSystem`;
- renderables and child IDs inspected by morph synchronization;
- binding records iterated, vector compaction/tombstones, allocations, and any
  targeted reverse-index hash-map work;
- CPU time for discovery and index rebuilding;
- dirty palette/morph/deformation job counts;
- repeated spawn/despawn and reload cleanup correctness.

Compare current ECS-sidecar discovery with a prototype or instrumented model of
dense system-owned records. Do not choose a table only for organizational purity:
it must have explicit lifecycle correctness and no worse hot-path behavior.

### Baseline instrumentation

The current implementation includes opt-in baseline logging. Start the process
with `CAT_PROFILE_IMPORTED_BINDINGS=1`; each system emits one 360-frame summary:

- `[ImportedBindingProfile][morph]` reports CPU time, registered renderables,
  child IDs inspected, morph bindings found, and GLTF factor-map entries scanned.
- `[ImportedBindingProfile][skin]` reports scan and index-rebuild CPU time, all
  world components inspected, skinned components found, and reconstructed
  bindings.

This is deliberately a baseline measurement of the current discovery approach.
Record it before migration and retain equivalent counters for the replacement so
the result compares direct record iteration with the work it displaces.

### Baseline capture: VTuber eye-tracking mirror with mirrors active

Captured on 2026-08-24 with `CAT_PROFILE_IMPORTED_BINDINGS=1`, using the
360-frame window and the mirrors active. Windows were stable; this records fixed
ECS discovery/index work rather than a tracking-dependent workload:

| Path | Baseline per frame | Fixed work observed |
| --- | --- | --- |
| skin component scan | approximately `0.021 ms` | `2,402` world components inspected to find `16` `SkinnedMeshComponent`s |
| skin index rebuild | approximately `0.027 ms` | `3` bindings reconstructed |
| morph synchronization | approximately `0.033–0.034 ms` | `332` renderables, `881` child IDs, and `8` morph bindings inspected; `3,648` factor entries scanned |

Thus the measured discovery/index baseline is approximately `0.082 ms/frame`
before any actual skin palette update or morph-caused deformation dispatch.

For comparison, the earlier 120-frame run had the mirrors disabled: it found
`2,288` world components, `320` renderables, and `845` child IDs, with a lower
approximately `0.076 ms/frame` baseline. Mirrors account for the additional
objects, but do not add a skin or morph binding and do not change the `456`
factor entries scanned per morph binding.

The morph count reveals an additional optimization requirement independent of
where the association lives: `3,648 / 8 = 456` factor entries per bound
primitive. The current bridge recomputes the same GLTF-wide active-factor list
for each of the eight primitives. A replacement should compute/group the active
factors once per dirty GLTF instance, then distribute primitive-local slices to
its dense binding records. This removes repeated factor-map scans as well as the
unrelated-renderable/child scans.

## Migration constraints

- Preserve user-authored `MorphTargetMapComponent`; it is GLTF-owned semantic
  configuration, not import bookkeeping.
- Preserve `GLTFComponent` factor state and `(node, primitive, target)` identity.
- Preserve shared deformation-cache output across window, mirrors, and XR eyes.
- Keep skin reverse-index invalidation semantics or improve them.
- Do not serialize import-only relationships into MMS.

## Completion criteria

- Document an ownership choice for both skin and morph associations.
- Public/authorable components have a supported tangible mutation contract; all
  other associations are internal runtime state.
- Test creation, cleanup, reload, and multiple-instance behavior.
- Benchmark the selected retrieval path against current behavior.
- Avoid unrelated world/renderable scans merely to rediscover imported bindings,
  unless measurement justifies the trade-off.
- Preserve skinning, morph driving, cache, desktop, and XR parity.

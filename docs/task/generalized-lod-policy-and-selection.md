# Generalized LOD policy and selection

Date: 2026-09-02

Status: architecture direction accepted; API and implementation planned.

Initial consumer: [Adaptive mirror detail](../desktop/adaptive-mirrors.md).
Existing mesh-specific proposal: [Blend shapes: LOD and authorable base mesh](../spec/blend-shapes.md#10-lod-and-authorable-base_mesh).

## Outcome

Provide one reusable authored policy for deciding how much detail an object
deserves for a viewer. Each consuming system maps that selection onto its own
cost controls:

- mirrors choose capture extent and, later, capture region;
- meshes choose geometry and morph variants;
- shadows may choose map resolution;
- procedural content may choose subdivision or bake detail; and
- animation or secondary motion may choose update rate or disable distant
  detail.

The LOD layer chooses a tier/detail factor. It does not perform these
consumer-specific changes itself.

## Locked architecture decisions

### Policy and effect are separate

`LODComponent` owns authored selection policy: metric, thresholds/bands,
hysteresis, cooldown, and any explicit bounds-source override.

The consumer owns the meaning of a result. `MirrorSystem` remains responsible
for mirror capture targets and projections; a mesh consumer remains responsible
for mesh and morph-resource changes.

### Projected coverage is the preferred metric

Projected screen coverage is the default for visual cost because it accounts
for object size, distance, camera projection, and foreshortening. Raw
world-space distance remains an explicit fallback/authoring option for objects
without usable bounds or behaviors where distance is the desired semantic.

Coverage should be measured from clipped projected bounds rather than only an
unclipped bounding sphere when practical.

### Selection is discrete and stable

The selector produces a small discrete tier plus an optional normalized detail
factor. Consumers use discrete allocation/resource bands rather than resizing
or swapping continuously every frame.

Threshold hysteresis and a minimum dwell/cooldown prevent oscillation near a
band boundary. Upward and downward thresholds may differ.

### Selection is viewer-dependent runtime state

Authored components do not serialize one `current` LOD. Runtime selection is
keyed by at least:

```text
(LOD component, viewer family)
```

A stereoscopic family selects one coordinated tier, using the greatest detail
requirement across its eyes. This prevents incompatible left/right choices.

Consumers that can vary per view, such as mirror capture generation, consume
each family selection independently. Consumers backed by one shared resource,
such as the current global `RenderableComponent.base_mesh`, must either support
per-view resources or conservatively use the highest requested tier across
active viewer families.

### Authored quality remains a ceiling

Existing consumer quality controls retain meaning as the preferred/native
maximum. LOD may reduce cost beneath that ceiling but does not silently exceed
it. A consumer may also define an authored minimum or disable adaptation.

## Conceptual runtime contract

Names remain provisional, but responsibilities should resemble:

```rust
enum LodMetric {
    ProjectedCoverage,
    Distance,
}

struct LodSelection {
    tier: u8,
    detail: f32,
}

struct LodSelectionKey {
    lod_component: ComponentId,
    viewer_family: ViewerFamily,
}
```

`LODSelectionSystem` evaluates policy after camera/viewer state and world bounds
are available, but before consumers build view-specific render work.

## Open API decisions

- Exact MMS syntax and whether bands are general scalar outputs or named tiers.
- Whether a policy attaches directly beneath each consumer or may be shared by
  a subtree/reference.
- The canonical projected-coverage measure: viewport fraction, pixel count,
  longest projected axis, or a combination.
- Timing units for cooldown and how deterministic/headless tests supply them.
- How total budgets can downgrade individual selections after policy
  evaluation without making results unstable.
- How selection changes become observable to diagnostics and authored code
  without encouraging per-frame handler churn.

## Implementation slices

### Slice 1: selector and mirror consumer

- [ ] Define viewer-family identity shared with render-view construction.
- [ ] Resolve world bounds and clipped projected coverage for each LOD target.
- [ ] Implement discrete bands, asymmetric hysteresis, cooldown, and an
      in-memory per-family selection cache.
- [ ] Add selection diagnostics without exposing process-local state as
      serialized authored data.
- [ ] Make `MirrorSystem` map selected tiers to bounded capture extents.
- [ ] Test simultaneous mono/stereo viewers and coordinated stereo eyes.

### Slice 2: mesh consumer reconciliation

- [ ] Refactor the mesh/morph LOD proposal so `MeshLODComponent` maps generic
      tiers to mesh resources rather than owning camera-distance selection.
- [ ] Initially select the highest requested tier across viewer families if
      mesh binding remains global.
- [ ] Decide whether per-render-view mesh LOD is worth the batching/resource
      complexity before implementing it.

### Slice 3: budgets and additional consumers

- [ ] Add an optional total detail/pixel budget with deterministic degradation
      ordering.
- [ ] Validate another consumer before treating the API as general-purpose.
- [ ] Measure CPU selection cost, GPU savings, resource churn, and visible
      popping in representative desktop and XR scenes.

## Acceptance criteria

- One policy implementation drives at least mirrors and one other consumer.
- A desktop and XR viewer can request different tiers without overwriting one
  shared serialized `current` value.
- Stereo eyes always receive a coordinated selection.
- Normal movement near a threshold does not cause per-frame target allocation
  or resource swapping.
- Disabling adaptive LOD preserves the consumer's authored quality behavior.

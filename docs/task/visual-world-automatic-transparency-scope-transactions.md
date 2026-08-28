# Implement automatic transparency scope transactions

Status: proposed / ready for implementation design review

Related:

- `docs/bugs/layout-background-transparency-order-varies-between-launches.md`
- `docs/task/layout-background-multilayer-transparency-correctness.md`
- `docs/task/layout-transparent-background-overlap-classification.md`
- `docs/task/single-layer-transparency-depth-write-contract.md`

## Goal

Implement a transaction-based API that lets `LayoutSystem` and future planar systems submit a
complete set of ambiguously classified transparent objects to `VisualWorld`. `VisualWorld` then
owns overlap analysis, cached resolution, and final single-layer or multilayer draw-list membership.

Classification work should occur when a scope transaction commits, not once per candidate and not
once per frame when the scope is unchanged.

## Current renderable entry path

The current layout-generated background path is:

```text
LayoutSystem::tick
  -> block::sync_bg_quad
  -> block::spawn_bg_quad
  -> create __bg -> ColorComponent -> RenderableComponent
  -> World::init_component_tree
  -> RenderableComponent::init emits RegisterRenderable { component_id }
  -> the command queue is flushed after LayoutSystem::tick
  -> SystemWorld::register_renderable
  -> RenderableSystem::register_renderable
  -> RenderableSystem::flush_pending uploads/resolves the mesh
  -> VisualWorld::register(component_id, ...)
```

`RenderableSystem` does not select a draw list solely from the material. It resolves the effective
renderable style and passes the material, color, opacity, authored `multiple_layers`, cutout,
background, overlay, and related flags into `VisualWorld`. `VisualWorld` uses those values to build
the opaque and transparent streams.

The layout transaction must augment this path, not replace it. In particular, do not add an
alternate `RenderableComponent::init` or a second `RegisterRenderable` intent for automatic
transparency. Mesh upload, bounds attachment, clipping registration, raycast registration, and
ordinary renderable cleanup must continue through the existing path.

## Proposed entry-point change

### Possible solution: two joined input paths

One possible implementation is to let two separate paths provide different parts of the same
visual instance's state to `VisualWorld`:

```text
normal renderable path
RenderableComponent -> RenderableSystem -> VisualWorld::register(...)
  supplies: component identity, mesh, material, transform, color, opacity, and render-phase flags

automatic-transparency metadata path
LayoutSystem -> begin/add/commit scope transaction -> VisualWorld
  supplies: scope identity, candidate ComponentId, root-local bounds, stacking depth, and stable order
```

These are two input paths, not two renderable creation or initialization paths. The existing
`RenderableComponent::init` and `RegisterRenderable` flow continues to create exactly one visual
instance. `VisualWorld` joins the independently arriving records by the renderable's stable
`ComponentId`.

The join must be insensitive to arrival order:

- if renderable registration arrives first, register the instance using the conservative
  multilayer fallback and reclassify it when committed metadata arrives;
- if committed metadata arrives first, retain it and apply its resolution when the instance later
  registers;
- if both are present, changes committed for the scope update the existing instance in place;
- removing a visual instance does not necessarily remove still-valid scope metadata, while removing
  a candidate from the next committed scope snapshot removes its automatic classification.

This approach keeps GPU resource registration and general renderable lifecycle in
`RenderableSystem`, while keeping scope-level overlap classification and final draw-list membership
in `VisualWorld`. It is a candidate design to validate during implementation, not a requirement to
introduce a second renderable API.

Pass the narrow automatic-transparency interface into layout processing:

```rust
pub fn tick(
    &mut self,
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    transparency: &mut dyn AutomaticTransparencySink,
)
```

At the existing `SystemWorld` call site, `VisualWorld` can implement this trait and be passed to
`LayoutSystem` through the restricted trait interface. This changes layout's entry point to the
classification service without giving layout access to instance buffers, draw streams, cameras,
or other renderer state.

For each dirty root, layout should use that interface while it already has the authoritative
geometry and style data:

```text
begin_scope_update(root, criteria)
  -> reconcile layout and create/update/remove normal ECS background subtrees
  -> add_candidate(renderable_component_id, bounds, depth, stable_order) for each eligible __bg
commit_scope_update(transaction)
```

For a newly spawned background, `add_candidate` uses its nested `RenderableComponent` ID; it does
not need an `InstanceHandle`. Currently, `spawn_bg_quad` returns the `__bg` transform ID and
`sync_bg_quad` returns nothing, so layout reconciliation must be adjusted to surface a small
candidate record containing the renderable ID and the already-computed geometry/stacking facts.
For example, `sync_bg_quad` could return `Option<LayoutTransparencyCandidate>` for its caller to
submit, or accept a transaction-scoped candidate collector. This is a return-value/data-flow
change, not an alternate component initialization path.

The ordinary registration intent is still queued and is normally processed by the flush
immediately after `LayoutSystem::tick`. Therefore the first implementation must deliberately
support this common ordering:

```text
automatic metadata commits first -> VisualWorld instance registers afterward
```

An existing background has the opposite shape: its instance may already be registered when the
new layout transaction commits. Commit must immediately update that instance's resolved queue
membership when the classification changes.

Layout should derive candidate alpha and geometry from the `StyleComponent` and the just-computed
layout result, not from queued `SetColor` or `UpdateTransform` intents that have not executed yet.
This prevents the transaction from classifying the previous layout generation.

### Registration behavior inside VisualWorld

`VisualWorld::register` already receives the stable `ComponentId` and maintains
`component_to_handle`. Extend registration so it consults committed automatic-transparency
metadata for that component:

```text
no automatic candidate       -> preserve ordinary authored policy
committed automatic result   -> install its resolved single/multilayer state and stable order
candidate but unresolved     -> conservatively install multilayer state
```

Similarly, committing a transaction must look up any already-registered candidates through
`component_to_handle` and update them in place. This is the join between the normal renderable
registration path and the side-band transaction path.

The current `multiple_layers: bool` input may remain during an incremental implementation, but
the join must define precedence explicitly: an explicit authored multilayer requirement must not
be downgraded by automatic classification. If the implementation needs to distinguish an explicit
single-layer promise from the current default `false`, introduce an internal policy enum at the
`RenderableSystem`/`VisualWorld` boundary rather than creating a different initialization method.

## Initial use case

The first consumer is a dirty `LayoutRoot` containing translucent layout-generated `__bg` quads.
Nested backgrounds whose projected rectangles overlap in layout-local X/Y must resolve to ordered
multilayer transparency. Isolated backgrounds may resolve to the single-layer path when the scope
contract permits it.

The API should be reusable by another system that can describe a stable planar transparency scope,
but the first implementation does not need to classify arbitrary 3D transparent geometry.

## Proposed public shape

Expose a narrow transaction interface rather than giving layout unrestricted access to all of
`VisualWorld`:

```rust
trait AutomaticTransparencySink {
    type TransactionToken: Copy;

    fn begin_scope_update(
        &mut self,
        scope: TransparencyScopeId,
        criteria: PlanarTransparencyCriteria,
    ) -> Self::TransactionToken;

    fn add_candidate(
        &mut self,
        transaction: Self::TransactionToken,
        candidate: AutomaticTransparencyCandidate,
    ) -> Result<(), TransparencyTransactionError>;

    fn commit_scope_update(
        &mut self,
        transaction: Self::TransactionToken,
    ) -> Result<(), TransparencyTransactionError>;
}
```

Names and exact types may change during implementation. Preserve these semantics:

- `begin` creates a staging replacement for one scope;
- `add_candidate` only updates staging state;
- `commit` atomically replaces the active scope snapshot and resolves it once;
- uncommitted staging state never affects rendering.

An explicit abort operation is optional. Starting a newer generation or dropping an uncommitted
transaction may discard the older staging snapshot.

## Identity model

Use stable ECS identity at the transaction boundary:

```text
TransparencyScopeId       layout root ComponentId for the initial consumer
TransparencyCandidateId   generated renderable ComponentId
TransactionToken          scope + monotonically increasing generation
```

Do not require an `InstanceHandle` when adding a candidate. Layout reconciliation can finish before
the generated renderable is uploaded and registered in `VisualWorld`.

`VisualWorld` already retains component-to-instance identity. It should be able to handle both
orders:

```text
candidate metadata commits -> visual instance registers later
visual instance registers   -> candidate metadata commits later
```

Until both sides exist and a committed result is available, automatic transparent content uses the
conservative multilayer fallback.

## Planar criteria and candidate data

The initial criteria should describe a stable planar scope, not a camera projection:

```rust
struct PlanarTransparencyCriteria {
    basis_x: [f32; 3],
    basis_y: [f32; 3],
    front_normal: [f32; 3],
    front_facing_only: bool,
}
```

Layout should submit the cheapest facts it already owns:

```rust
struct AutomaticTransparencyCandidate {
    component: ComponentId,
    bounds_xy: Rect2,
    stacking_depth: f32,
    stable_order: u32,
}
```

If layout rectangles are always expressed directly in scope-local coordinates, storing the full
basis in `VisualWorld` may only be necessary for validation, front-direction conventions, and
future consumers. Do not make `VisualWorld` reconstruct layout rectangles from GPU meshes or query
ECS layout topology.

## VisualWorld state

Add CPU-side automatic-transparency state, preferably outside the compact instance data copied into
GPU buffers:

```text
active scope descriptors
staging transactions by scope/generation
candidate component -> scope lookup
candidate component -> resolved classification/order
dirty automatic-transparency scopes
```

The conceptual policy is:

```text
SingleLayer       explicit authored/system promise
MultiLayer        explicit authored/system requirement
Automatic(scope)  resolved by the committed scope transaction
```

The current `VisualInstance.multiple_layers` boolean may remain as a resolved cache initially, or
be replaced by a policy plus resolved state. Whichever representation is chosen must preserve
existing explicit `Opacity.multiple_layers()` behavior.

## Commit behavior

On a valid commit:

1. validate the criteria and every candidate;
2. compare the staged generation with the active scope generation;
3. atomically replace the active candidate snapshot;
4. remove metadata for candidates omitted from the new snapshot;
5. compute scope-local overlap groups;
6. resolve classification and stable stacking order;
7. update registered instances whose resolution changed;
8. mark draw caches dirty only when membership or order changed;
9. retain resolved metadata for candidates whose visual instance has not registered yet;
10. discard staging data.

Invalid, unsupported, incomplete, or stale transactions must not partially update the active
snapshot. Their candidates remain conservatively multilayer until a valid committed state exists.

## Initial overlap algorithm

Start with pairwise rectangle overlap inside each committed scope:

```text
two translucent candidates overlap when their open-area X and Y intervals overlap
```

Edge contact alone should not create a multilayer group unless rasterization tests demonstrate that
an epsilon is required. Define and test the epsilon rather than inheriting floating-point accident.

Build connected overlap groups:

- isolated candidate: eligible for scope-local single-layer;
- group with two or more candidates: multilayer;
- ambiguous geometry or invalid stacking data: conservative multilayer.

Use `stacking_depth` and `stable_order` to make equal or nearly equal depth cases deterministic.
The view-sorted multilayer path may still determine ordinary far-to-near order, but registration
order must not remain its final tie-break for candidates in a committed scope.

Pairwise `O(n^2)` detection is acceptable for the first implementation because typical layout
scopes contain small candidate sets. Preserve the transaction boundary so a future sweep or spatial
index can replace the algorithm without changing callers.

## LayoutSystem integration

During reconciliation of a dirty layout root:

1. begin one transaction for that layout transparency scope;
2. traverse every eligible translucent generated `__bg` in the scope;
3. submit its root-local padding-box rectangle and resolved stacking metadata;
4. commit only after the layout root has completed reconciliation.

Opaque backgrounds do not need automatic transparent candidates. A candidate whose alpha changes
to opaque disappears from the next committed snapshot and returns to normal opaque classification.

Prefer passing a narrow `AutomaticTransparencySink` into layout processing. This keeps layout tests
independent of the Vulkan renderer and avoids teaching `LayoutSystem` about draw-list internals.
If borrow structure makes direct sink access impractical, use an equivalent staged coordinator in
`SystemWorld`; preserve atomic begin/add/commit semantics.

## Invalidation and lifecycle

Commit a new scope generation when any classification input changes:

- layout becomes dirty and is reconciled;
- a generated background is added or removed;
- alpha crosses the opaque/translucent boundary;
- scope-local bounds or stacking depth changes;
- relevant content attaches, detaches, or reparents;
- a candidate moves relative to its scope.

Moving, rotating, or scaling the entire layout root in world space should not invalidate its
root-local overlap groups.

Also handle:

- candidate removal before visual registration;
- visual removal while committed candidate metadata remains;
- layout-root/scope removal;
- stale transaction tokens;
- a newer transaction superseding an uncommitted older transaction;
- candidate migration between scopes;
- duplicate candidate submission within one generation.

## Cross-scope contract

This implementation proves overlap only within one declared planar scope. It does not prove that an
isolated layout background satisfies a global depth-writing single-layer contract against arbitrary
transparent world geometry.

Before resolving an isolated automatic candidate to a depth-writing fast path, choose one explicit
policy:

- layout scopes are isolated transparency composition groups;
- automatic `SingleLayer` means single only within the scope and uses a non-depth-writing layout
  path;
- cross-scope overlap is evaluated separately per view;
- or candidates without a global isolation guarantee remain multilayer.

Until `docs/task/single-layer-transparency-depth-write-contract.md` resolves this, use the
conservative policy needed for correctness. Do not silently infer global isolation from local
rectangle analysis.

## Implementation stages

### Stage 1 — transaction storage and lifecycle

- [ ] Add scope, transaction token, criteria, candidate, policy, and error types.
- [ ] Implement begin/add/commit staging without changing draw-list classification.
- [ ] Implement generation checks, atomic replacement, stale candidate removal, and scope removal.
- [ ] Cover registration-before-metadata and metadata-before-registration orderings.

### Stage 2 — VisualWorld classification

- [ ] Implement planar rectangle validation and overlap grouping.
- [ ] Cache resolved candidate classification and stable stacking metadata.
- [ ] Integrate automatic resolution with opaque, single-layer, and multilayer list construction.
- [ ] Use conservative multilayer fallback for unresolved or invalid state.
- [ ] Remove component/hash iteration order as a transparency ordering tie-break.

### Stage 3 — LayoutSystem producer

- [ ] Add a narrow sink/coordinator to layout reconciliation.
- [ ] Define one scope per relevant layout root.
- [ ] Submit every translucent generated `__bg` once per dirty reconciliation.
- [ ] Commit only after the scope snapshot is complete.
- [ ] Ensure opaque/translucent alpha transitions and subtree removal update the next snapshot.

### Stage 4 — renderer correctness

- [ ] Verify overlapping groups enter an order-correct multilayer stream.
- [ ] Verify isolated candidates follow the chosen cross-scope safety policy.
- [ ] Preserve existing authored `multiple_layers` behavior.
- [ ] Verify clipping, overlays, emissive materials, text backgrounds, and background render phases.

### Stage 5 — performance and observability

- [ ] Count automatic scopes, candidates, overlap edges, resolved single/multilayer candidates, and
      fallback candidates in debug statistics.
- [ ] Measure transaction commit time, draw-cache rebuild time, draw calls, and transparent fragment
      cost on representative editor layouts.
- [ ] Confirm unchanged scopes perform no overlap work per frame or per eye.
- [ ] Decide from measurements whether pairwise overlap remains sufficient.

## Tests

### Unit tests

- transaction state is invisible before commit;
- commit atomically replaces the previous generation;
- stale tokens cannot mutate active state;
- omitted candidates are removed;
- isolated, overlapping, edge-touching, and invalid rectangles classify as specified;
- connected overlap groups are deterministic;
- equal stacking depths use stable scope order;
- unresolved candidates fall back to multilayer;
- late visual registration adopts the committed result.

### Integration tests

- repeated fresh evaluation of `data-viz-json-file` produces identical classifications;
- nested panel/chart/bar backgrounds form the expected overlap groups;
- a dirty layout update causes exactly one begin/add*/commit sequence;
- moving the whole layout root does not rebuild root-local overlap groups;
- resizing/reflowing the layout commits a new generation;
- attach/detach and alpha changes remove stale classifications;
- two independent layout scopes do not corrupt one another.

### Runtime validation

- verify stable color and opacity across repeated desktop launches;
- compare front-facing and moderately oblique views;
- validate both post-processed and non-post-processed render graphs;
- record desktop and XR performance for a layout with many translucent backgrounds.

## Complexity and performance target

For a scope with `n` candidates and `k` overlap edges:

- staging: `O(n)` time and memory;
- initial pairwise classification: `O(n^2)` time;
- cached active metadata: `O(n + k)` space;
- active plus staging during rebuild: approximately `O(2n + k)` space;
- unchanged-frame classification work: `O(1)` per scope, excluding ordinary draw-list use.

The expected practical cost for ordinary panels is small. Sorting/detection is paid only at commit;
draw calls and transparent pixel overdraw are expected to dominate once candidate counts grow.

## Acceptance criteria

- `VisualWorld` exposes atomic begin/add/commit automatic-transparency scope updates.
- Layout submits complete scope snapshots only when relevant layout state changes.
- Partial and stale transactions never affect active rendering.
- Candidate registration order does not affect classification or stacking order.
- Overlapping translucent layout backgrounds consistently enter a correct multilayer path.
- Unresolved and globally ambiguous cases use the documented conservative fallback.
- Unchanged layouts do not repeat overlap analysis every frame or eye.
- The API remains usable by a future planar system without depending on layout-specific component
  types.

## Non-goals

- Camera-perfect classification of arbitrary 3D transparent geometry.
- General order-independent transparency.
- Replacing explicit author-selected single-layer or multilayer policies.
- Enabling transparent depth writes before their global contract is resolved.

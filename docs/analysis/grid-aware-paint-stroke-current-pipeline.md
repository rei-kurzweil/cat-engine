# Grid-aware paint stroke current-pipeline investigation

Date: 2026-08-19

Status: first source and test investigation; documentation only

Related:

- [Grid-aware paint stroke interaction model](../task/grid-aware-paint-stroke-interaction-model.md)
- [Editor grid and paint 0.8.0 release gate](../task/editor-grid-paint-0.8.0-release-gate.md)
- [Grid snapping](../spec/grid-snapping.md)
- [Paint system reducer](../draft/paint-system-reducer.md)

## Scope

This pass traces the existing implementation without changing `src/`. It asks
what information reaches Paint today, which grid state is authoritative in each
path, what the three tools actually do, and whether a grid span or footprint
needs to become persistent scene state.

Inspected paths:

- `src/engine/ecs/system/gesture_system.rs`
- `src/engine/ecs/system/editor_paint_system.rs`
- `src/engine/ecs/system/editor/paint_panel.rs`
- `src/engine/ecs/system/editor/context.rs`
- `src/engine/ecs/system/grid_system.rs`
- `src/engine/ecs/system/object_placement_preview.rs`
- `src/engine/ecs/system/paint_placement.rs`
- `src/engine/ecs/component/grid.rs`
- `src/engine/ecs/component/raycastable.rs`
- `src/scripting/component_registry.rs`
- `crates/meow-meow-script`

## Executive findings

1. Paint does not receive a live pointer ray or the renderable currently under
   the pointer during a drag. It receives a point already mapped by
   `GestureSystem` and the renderable captured at drag start.
2. Desktop and ordinary XR drags currently use different mapping and
   continuation rules. Desktop captures a plane perpendicular to the initial
   ray; ordinary XR requires continued contact with the original renderable.
3. The selected-grid state used by gizmos is not used by Paint. Paint snapping
   only resolves from a hit on grid runtime geometry, but committed grid runtime
   geometry is deliberately non-raycastable.
4. `GridStep.cell` currently addresses points at integer multiples of spacing,
   which are grid-line intersections. Existing design docs also use the word
   cell for half-spacing-centered areas. That phase distinction must be settled
   before generating paths.
5. Free Draw is a one-preview placement gesture, not a continuous draw stroke.
   Spray Can is immediate, random world-space mutation with no preview or
   rollback. Line has no placement implementation.
6. Paint has no cancellation event, pointer identity, captured grid frame,
   endpoint pair, visited address set, preview collection, or atomic stroke
   transaction.
7. Nothing found in `crates/meow-meow-script` requires in-progress paint state
   to be a language feature. The host already owns Grid/GridBinding component
   integration. The narrow 0.8 hypothesis is therefore transient host/editor
   state with ordinary serialized output, not a new MMS language construct.

## Current event and mutation pipeline

```text
pointer activations + sorted ray hits
  -> GestureSystem captures one drag renderable
  -> EventSignal::DragStart / DragMove / DragEnd / Click
  -> editor-root paint handler
  -> private PaintEvent normalization
  -> small PaintState reducer
  -> mutable PaintStrokeRuntime + direct world mutation
```

The reducer records only `Idle` versus `Dragging`. The tool-specific working
state lives in one `PaintStrokeRuntime` per installed editor paint handler.
That runtime is not keyed by pointer/raycaster.

`PaintEvent` drops several pieces of input information:

- `DragStart.raycaster`
- `DragStart.ray_dir_world`
- activation source
- desktop screen position
- drag delta
- `DragEnd.hit_point`

`DragMove` does not contain a current ray direction at the engine-event level,
so Paint cannot independently intersect the pointer ray with the captured grid
plane from the normalized event it receives.

## Gesture mapping findings

### The renderable is captured upstream

`GestureState.drag_renderable` is assigned at press time. Every later
`DragMove` is emitted to that same renderable and carries that same renderable
ID. Therefore the equality check against `PaintStrokeRuntime.captured_renderable`
does not implement a meaningful surface-continuity policy; it confirms the
capture already imposed by `GestureSystem`.

Paint cannot currently observe a transition from terrain renderable A to
adjacent terrain renderable B during one drag. Solving that only inside the
paint handlers is impossible with the current event payload.

### Desktop default

The default `GestureSystem.drag_update_policy` is `StartPlaneProjection`.
For a normal desktop target whose raycastable policy is `Auto`, the gesture:

- is captured until release; and
- maps the current ray to a plane through the initial hit whose normal is the
  initial ray direction.

That is a stable screen-facing drag plane. It is not the hit surface plane and
is not the selected grid plane.

### Ordinary XR default

For an ordinary spatial pointer target that is not a controller-draggable,
`Auto` resolves to:

- `RequireTargetContact`; and
- `ContactHit` mapping.

The gesture emits moves only while the pointer still intersects the exact
drag-start renderable. Crossing a mesh/renderable boundary or losing contact
temporarily stops move delivery.

### Consequence for parity

Equivalent physical gestures do not yet supply equivalent world points to
Paint:

| Input | Continuation | Mapped point |
| --- | --- | --- |
| Desktop | captured | initial-ray-normal plane |
| Ordinary XR | original-target contact required | current hit on original target |

Grid address generation can normalize event frequency, but it cannot repair
this input semantic difference after the fact. The paint design needs an
explicit pointer-to-address contract before tool behavior is specified.

## Click and drag ordering

On release, `GestureSystem` emits `DragEnd` first. It may then emit `Click` if
the pointer-specific click threshold and same-target checks pass.

For Free Draw:

- `DragStart` creates one preview asset;
- `DragMove` repositions that asset;
- `DragEnd` commits that asset unconditionally when the preview exists; and
- the later Free Draw click handler is intentionally a no-op.

So a click placement currently succeeds through the drag lifecycle, not the
click handler. A test that sends a bare `Click` does not exercise the real Free
Draw placement path.

For Spray Can:

- `DragStart` creates no preview and places nothing;
- the first `DragMove` places one object because there is no last position;
- later moves place when their mapped world point is at least `0.5` units from
  the last accepted point;
- each accepted point gets a random offset of up to `1.5` in world XZ; and
- a no-movement gesture places one object in the later `Click` handler.

For Line:

- `is_paint_active` accepts the tool when focus and asset requirements pass;
- the visible status path labels it unsupported;
- stroke state can still enter `Dragging`; and
- preview startup, click, and move handlers all return no placement.

This is slightly different from a single authoritative activity gate rejecting
Line: activation, status, and effect support currently disagree.

## Cancellation and preview lifecycle

There is no paint-domain cancellation event. `PaintEvent` has only
`StrokeEnded`, and its effect commits any active preview.

The following do not currently form explicit paint cancellation paths:

- pointer component or raycaster disappearance;
- focus change;
- tool or asset change;
- editor switch;
- active-grid deletion or disable;
- a second pointer beginning a gesture; or
- loss of the target without a later release event.

`GestureSystem` can remove its pointer state when a pointer or raycast source
disappears without emitting `DragEnd`. In that case Paint can retain an active
runtime and preview. Starting another stroke replaces `PaintStrokeRuntime`, but
the replacement path does not first remove the previous preview subtree.

Free Draw previews use a raycastable wrapper. The preview shell adds
`Selectable.off()` and `Serialize.off()`, but it does not disable that wrapper's
raycasting. This supports the existing concern that preview geometry can affect
later ray-hit ordering, especially for XR's original-target-contact rule.

Spray Can has a different failure mode: placements are attached immediately as
the gesture advances. Because there is no transaction or rollback collection,
a later cancellation cannot remove the partial spray stroke as one operation.

No general editor undo/redo operation abstraction was found in the current
source. "Atomic undo" in the existing release docs is a future-facing contract,
not an available primitive that Line can already consume.

## Grid authority findings

There are three relevant grid-resolution paths:

| Consumer | Current source |
| --- | --- |
| Gizmo translation | object `GridBinding`, otherwise `EditorContextState.active_grid_owner_transform` |
| `GridSystem::active_grid_for_editor` | `EditorComponent.selected` if that selection is a grid/component owner |
| Paint preview/update | grid owning the hit renderable |

The Grid panel records an independent active-grid owner in editor context. This
is the suitable workspace concept because selecting an ordinary scene object
should not discard the active grid. `EditorComponent.selected` is also used for
ordinary scene selection, so `active_grid_for_editor` is not a safe long-lived
paint authority by itself.

Paint's `PaintContext::grid_snap` does not consult either selected-grid path.
It calls `grid_hit_context_for_renderable`. Meanwhile committed grid live
runtime installs:

- `SelectableComponent::off()`; and
- `RaycastableComponent::disabled()`.

Therefore the normal committed grid visual cannot win a scene hit, and the
documented status text `snap only on shown grid hits` describes a path that is
normally unavailable. Existing focused tests prove snap math, grid selection,
binding, and a manually constructed snapped preview, but do not prove selected
grid -> ordinary scene hit -> snapped Free Draw.

This does not reopen general gizmo/object snapping. It isolates the missing
selected-grid handoff in Paint.

## Grid address phase ambiguity

`GridSystem::snap_hit` computes:

```text
u = round(local_x / spacing)
v = round(local_z / spacing)
point = (u * spacing, 0, v * spacing)
```

The result is named `GridStep { cell: [u, v] }`, but its point lies on the
intersection of two rendered grid lines when rendering and snapping agree.

A geometric grid cell instead occupies the area between four lines. Its center
for a lower-left integer address would be:

```text
((u + 0.5) * spacing, 0, (v + 0.5) * spacing)
```

Both are valid placement lattices, but they are not interchangeable. Before
Line or Free Draw path generation, the docs need one of these outcomes:

1. define paint addresses as grid vertices/intersections and stop calling them
   cells;
2. define paint addresses as cell areas/centers and add an explicit half-cell
   phase; or
3. define an address plus anchor/phase policy so assets can use either lattice.

Grid dimensions are also not currently part of `ActiveGrid`; it carries frame
and spacing but not `size_x`/`size_z`. Address conversion therefore has no
bounds policy even though `GridComponent` and the visual have finite sizes.
The design must state whether paint uses an infinite mathematical grid or is
clipped to the authored visual extent.

## Surface placement findings

Current placement combines two concepts:

- a pointer hit identifies a target renderable and point; and
- an optional grid snap can replace both the surface point and normal with the
  grid point and grid normal.

When a grid snap is present, `resolve_surface_placement_frame` does not retain
the hit surface's normal or height. It treats the grid itself as the placement
surface. This is suitable for painting directly on a grid plane, but differs
from the release-gate language about retaining arbitrary scene contact while
quantizing only in-plane coordinates.

The tools therefore need a declared surface mode:

- **grid-plane mode**: address and placement frame both come from the captured
  grid; or
- **surface-following mode**: the selected grid supplies in-plane addressing,
  while scene geometry supplies height and orientation at each sample.

Line can plausibly start with grid-plane mode. Free Draw and Spray Can need an
explicit product decision because their names suggest surface-following use,
but the current selected-grid goal suggests a discrete grid footprint.

## Do we need persistent grid sub-selection?

This pass found no current requirement for the line endpoint pair or brush
footprint to survive commit, be selected by another system, serialize into the
scene, or be edited afterward.

What is required during a gesture is richer than ordinary editor selection:

- captured grid identity and frame;
- pointer identity or an explicit single-pointer lock;
- start/current address;
- ordered path or footprint;
- visited-address deduplication;
- preview handles keyed by address; and
- explicit commit/cancel state.

That evidence supports a **shared transient grid-gesture model as the leading
hypothesis**, while keeping the general persistent grid-subselection model open
only if post-commit editing or cross-system handoff becomes a real requirement.
This is a scope recommendation, not a commitment to a particular Rust/ECS
layout.

## MMS and `meow-meow-script` boundary

The language crate does not contain paint-tool or grid-stroke semantics. Engine
Grid and GridBinding support is registered in the host's
`src/scripting/component_registry.rs`.

For 0.8, the narrow boundary suggested by the current code is:

- in-progress pointer samples, endpoints, visited addresses, previews, and
  random state remain transient host/editor state;
- committed paint results remain ordinary authored component subtrees;
- a committed snapped object may retain the existing serializable
  `GridBinding`; and
- no new `meow-meow-script` syntax is needed unless MMS programs must create,
  inspect, or replay strokes as values.

This keeps `meow-meow-script 0.8` involved through stable component
materialization/serialization rather than through a premature editor-gesture
language feature.

## Existing automated evidence

Executed on 2026-08-19 without source changes:

- `cargo test -p mittens-engine editor_paint_system::tests::`
  - 14 passed;
- `cargo test -p mittens-engine gesture_system::tests::`
  - passed;
- `cargo test -p mittens-engine grid_system::tests::`
  - 12 passed.

The paint tests confirm the current one-placement drag, explicit Line no-op,
Spray Can scatter, color behavior, editor routing, and preview binding. They use
synthetic paint/gesture events and do not cover the complete pointer -> gesture
mapping -> selected grid -> paint path.

Missing evidence:

- Free Draw through a real `GestureSystem` click;
- selected grid plus ordinary non-grid scene hit;
- a continuous multi-address Free Draw stroke;
- desktop/XR address parity;
- cross-renderable surface continuity;
- cancellation and preview cleanup;
- multi-pointer arbitration;
- Line endpoint/path behavior; and
- deterministic/rollback-safe Spray Can behavior.

## Constraints for the next design pass

Before implementation, documentation should decide these in order:

1. Define the paint lattice: vertex/intersection, cell center, or explicit
   anchor phase.
2. Define the pointer-to-address mapping independently for grid-plane and
   surface-following tools.
3. Choose the one active-grid source Paint captures at stroke start.
4. Decide whether the grid frame is immutable for the gesture and what happens
   when its live grid changes or disappears.
5. Define cancellation as a first-class terminal outcome distinct from
   `DragEnd` commit.
6. Decide single-pointer locking versus pointer-keyed paint sessions.
7. Define preview reconciliation and commit atomicity before tool-specific cell
   generation.
8. Only then choose Free Draw interpolation, Line rasterization, and Spray Can
   footprint/randomness policies.

The strongest current scope direction is to specify one shared transient
address/session contract and let each tool provide a pure path or footprint
policy. A persistent editor grid-region selection remains deferred until a use
case requires it.

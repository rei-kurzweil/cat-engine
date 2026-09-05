# Grid-aware paint stroke interaction model

Date: 2026-08-19

Status: implemented contract for the `mittens-engine 0.8` paint release gate.

Related:

- [Paint as a first-class editor interaction mode](paint-as-first-class-editor-interaction-mode.md)
- [Editor grid and paint 0.8.0 release gate](editor-grid-paint-0.8.0-release-gate.md)
- [Grid + Gizmo + Paint end-to-end UX and test matrix](grid-gizmo-paint-end-to-end-ux-and-test-matrix.md)
- [Unified grid snap mode](unified-grid-snap-mode-mms-gizmo-and-paint.md)
- [Paint system reducer](../draft/paint-system-reducer.md)
- [Paint system reducer event model](../analysis/paint-system-reducer-event-model.md)
- [Current paint-stroke pipeline investigation](../analysis/grid-aware-paint-stroke-current-pipeline.md)
- [Paint-stroke live diagnostics](../how_to/paint-stroke-live-diagnostics.md)
- [Paint procedural RenderAssets instantiation](paint-procedural-render-assets-instantiation.md)
- [Paint-stroke debug performance and desktop/XR artifacts](paint-stroke-debug-performance-and-desktop-xr-artifacts.md)

## Adopted contract

Paint uses one transient, editor-owned `GridGestureSession`; MMS does not
receive a stroke API. A session captures the selected enabled grid's frame,
finite extent, tool, asset, brush radius, reference mode, and owning pointer at
drag start. Any change to those captured inputs, explicit cancellation, lost
pointer, focus/editor/tool/asset change, or grid disable/delete rolls back all
provisional placements.

- Paint addresses are cell centres: `(u, v)` maps to
  `((u + 0.5) * spacing, (v + 0.5) * spacing)` in grid-local X/Z.
- The selected grid is an analytic finite plane. It wins only when it is the
  nearest eligible hit; starting there locks contact and normal to that plane.
- A stroke begun on geometry projects each candidate cell along the captured
  grid normal, on the captured hit side. Missing projections omit the cell.
- Free Draw uses an ordered, deduplicated supercover. Line uses a thin,
  8-connected reverse-symmetric raster with diagonal exact ties. Spray uses
  the deduplicated union of filled grid-local Euclidean disks.
- A release commits one undo/redo group. Existing committed objects do not
  occupy cells, so separate strokes may stack. One pointer owns one editor
  stroke; other pointers are ignored.
- Without a selected enabled grid, Free Draw and Spray retain their legacy
  surface behaviour. Line is inactive and reports that it requires a grid.

### Live integration failure: analytic-plane drag start (2026-08-21)

The pure finite-plane helper has unit coverage, but the live paint gesture does
not currently honor this contract. In the desktop-only `paint-grids-desktop`
repro, a Free Draw stroke can start when the pointer intersects ordinary
raycastable geometry near or behind the selected grid. It does **not** start
when the pointer intersects only the visible finite grid plane in empty space.

This demonstrates a wiring/raycast-candidate failure, not evidence that the
analytic intersection math itself is wrong. The runtime is still requiring a
scene renderable hit before it emits or accepts paint `DragStart`, rather than
considering the selected enabled grid's analytic plane as an eligible nearest
candidate.

Direction updated 2026-09-05: perform this arbitration under the new
`EditorInteractionMode::Paint`. Reuse the grid's existing live renderable for
BVH broad phase, keep it non-selectable, and use the analytic finite plane for
the exact start/continuation hit. Paint mode suppresses Select, 3D Cursor, and
gizmo input while this candidate is eligible.

Required investigation:

- At pointer activation, compare the nearest scene/BVH hit with
  `GridSystem::intersect_captured_grid_plane(...)` for the selected grid.
- Feed the nearer finite analytic-plane result into the same gesture/pointer
  candidate arbitration used for scene hits, with stable target ownership.
- Trace why an empty-space plane result is absent, rejected, or lost before
  Paint receives `DragStart`.
- Preserve ordinary geometry behavior when it is genuinely nearer than the
  grid plane; this is not a request for the grid to win through foreground
  geometry.

Acceptance addition:

- Free Draw can begin on an unobscured, in-bounds selected-grid plane with no
  scene renderable behind it.
- If a scene surface is nearer than that plane, normal scene-hit behavior wins.
- Starting from an analytic plane produces the captured grid address, plane
  contact, preview, and commit defined by this tracker.

## Purpose

Figure out the interaction and data model needed to make Free Draw, Spray Can,
and Line dependable consumers of the selected grid. This tracker is for
investigation and contract definition. It deliberately does not choose a Rust
type hierarchy, ECS component layout, reducer boundary, or MMS syntax yet.

The immediate question is broader than snapping one world-space point. A paint
gesture can refer to:

- one cell under the pointer;
- a start cell and a changing end cell;
- the ordered cells crossed between pointer samples; or
- a footprint of cells affected by a brush.

The editor can select an active grid today, but it does not yet have an explicit
model for these transient references to locations or portions of that grid.

## Current checkpoint

Treat snapping ordinary objects to the selected grid as working runtime
behavior unless a focused reproduction shows otherwise. Do not make general
grid snapping a blocker for this tracker.

The remaining paint behavior is different:

- Free Draw starts one placement preview, moves that preview during the drag,
  and commits that one preview at stroke end. It does not represent or retain
  the sequence of grid cells crossed by the gesture.
- Free Draw and Spray Can reject a move when its raw renderable differs from
  the renderable captured at stroke start. A stroke across adjacent pieces of
  one usable surface can therefore stop unexpectedly.
- Spray Can creates placements immediately, uses a random world-XZ offset, and
  gates samples by world-space distance. It does not describe a grid-local
  radius, cell density, visited cells, preview set, or deterministic replay.
- Line is exposed in the Paint panel. General paint activation can accept it,
  the status path calls it unsupported, and its preview/click/move handlers are
  no-ops. Activation, feedback, and effect support therefore disagree.
- `PaintStrokeRuntime` has general gesture flags, a captured renderable, a last
  position, and at most one `PlacementPreviewSession`. It has no captured grid
  frame, start/end cell pair, ordered cell path, or brush footprint.
- The existing `GridStep`/snap result can identify a quantized step for one hit,
  but it is not yet a complete stroke or grid-subselection model.

The Color tool is not part of this investigation. It edits the hit renderable
and is reported to work.

## Investigation checkpoint: 2026-08-19

The first source/test trace is recorded in
[Current paint-stroke pipeline investigation](../analysis/grid-aware-paint-stroke-current-pipeline.md).
It narrows the problem in several important ways:

- Paint receives a gesture-mapped point and the drag-start renderable, not the
  live pointer ray or current renderable under the pointer.
- Desktop maps ordinary drags to a captured plane perpendicular to the initial
  ray. Ordinary XR requires continuing contact with the initial renderable.
  Address parity cannot be solved solely inside the current paint handlers.
- Gizmos use `EditorContextState.active_grid_owner_transform`, while Paint
  still resolves snapping from grid-owned hit geometry. Committed grid visuals
  are non-raycastable, so Paint lacks the selected-grid handoff it needs.
- `GridStep.cell` currently quantizes to grid-line intersections. It is not yet
  the half-cell-centered address described by parts of the release plan.
- Free Draw owns one preview, Spray Can mutates immediately with no rollback,
  and Paint has no cancellation event or pointer-keyed session.
- No current need was found to persist or serialize an in-progress grid span.
  A shared transient grid-gesture model is the leading scope hypothesis, while
  a general persistent grid-subselection feature remains unproven.
- No new `meow-meow-script` language feature appears necessary for the narrow
  0.8 path. Gesture state can remain host/editor-internal while committed
  objects use ordinary serialized components and the existing `GridBinding`.

This checkpoint does not select concrete Rust types or an ECS ownership layout.
It does establish that pointer mapping, address phase, active-grid authority,
and cancellation must be decided before choosing tool algorithms.

## Vocabulary to validate

Use these as neutral concepts during the investigation, not as proposed type
names:

- **active grid**: the editor-selected enabled grid that supplies identity,
  frame, axes, spacing, and anchor policy;
- **grid address**: one discrete cell coordinate in a particular grid, such as
  `(grid identity, u, v)`;
- **grid span**: a start address and current/end address on the same captured
  grid;
- **grid path**: an ordered, deduplicated sequence of addresses crossed by a
  gesture or generated between two endpoints;
- **grid footprint**: the set or weighted set of addresses affected by one
  brush sample;
- **paint stroke session**: transient gesture state that captures the tool,
  asset/color operands, grid contract, samples, previews, and commit/cancel
  state.

An active grid and a grid span are not necessarily the same kind of
"selection." The former is durable editor context. The latter may be ephemeral
interaction state that disappears on commit or cancellation. We should not add
a persistent scene component merely because both are described as selected in
the UI.

## Interaction contracts to decide

### Shared gesture lifecycle

For all three tools, decide and document:

1. what must be present at pointer/drag start;
2. what grid and tool state is captured for the lifetime of the gesture;
3. how a pointer hit or ray is converted to a grid address;
4. how skipped pointer samples are interpolated;
5. how previews are added, updated, and removed;
6. whether placements happen continuously or commit atomically at drag end;
7. what cancels the gesture and whether cancellation rolls back work; and
8. how desktop and XR gesture streams yield the same grid result.

At minimum, investigate cancellation on panel focus loss, editor switch, asset
change, tool change, grid deletion/disable, invalid ray, and explicit pointer
cancel. A grid transform or spacing change during a stroke also needs a stated
policy.

### Free Draw

Expected user behavior to validate:

- a click places one object;
- a drag places objects in every crossed cell, without holes caused by event
  frequency and without duplicate placements in one cell;
- revisiting a cell during one stroke has an explicit policy;
- movement across adjacent compatible renderables does not terminate the
  stroke merely because the raw renderable ID changed; and
- previewed and committed objects use the same grid addresses and poses.

Open questions:

- Is an active grid required, or may Free Draw retain an unsnapped surface mode
  when no grid is active?
- Should the path be a supercover of all cells touched by the continuous
  segment between samples, or a thinner discrete line?
- Does revisiting a cell do nothing, replace the earlier preview, or create an
  additional object?
- Is continuity based on the captured grid plane, compatible scene surfaces,
  a surface owner, or reprojection at each address?

### Line

Working interaction hypothesis to test:

- Line requires an active enabled grid and a selected asset.
- Pointer/drag start captures the grid and the first grid address.
- Pointer movement updates a second address in that captured grid.
- The tool derives an ordered set of cells between the two addresses and
  reconciles a preview object per cell.
- Pointer/drag end commits the final derived cell set as one editor operation.
- A click or zero-length drag produces one cell.

This does not require committing to a persistent "two selected grid points"
component. First determine whether the endpoint pair is only gesture-local or
must also support editing after placement.

Questions that must be resolved before implementation:

- thin Bresenham-style cells versus a supercover of every touched cell;
- how exact corner crossings break ties;
- whether endpoint calculation projects the pointer ray onto the captured grid
  plane or depends on a scene-geometry hit;
- how orientation/contact height is chosen for objects along the line;
- whether reverse drags produce the same cell set in reverse order; and
- whether an already occupied cell is skipped, replaced, stacked, or causes
  the operation to fail.

### Spray Can

The current random world-XZ scatter is not a sufficient grid contract. Decide
whether Spray Can is fundamentally:

- a grid-local disk footprint sampled around the current address;
- a probabilistic decision per cell inside that footprint;
- a rate-based stream whose candidates are quantized to cells; or
- an intentionally unsnapped surface-space tool that only uses a grid when one
  is active.

The chosen behavior needs explicit units for radius, density/rate, spacing
between samples, duplicate suppression, and orientation. Determine whether the
random seed must be captured so previews, commit, undo/redo, tests, and MMS-host
behavior can reproduce the same result.

## Candidate ownership models to compare

Do not select one until the gesture traces and acceptance cases below have been
worked through.

### Tool-private transient state

Each tool stores its own captured grid and cells inside the editor paint
system. This is the smallest change, but risks three subtly different grid
addressing, interpolation, cancellation, and preview implementations.

### Shared transient grid gesture model

Paint tools share one runtime representation for captured grid identity,
addresses, paths/footprints, and preview reconciliation. Tools provide the cell
generation policy. This may give Free Draw, Spray Can, and Line consistent
behavior without making an in-progress gesture persistent scene state.

### General grid sub-selection model

The editor exposes points, spans, paths, or regions as a reusable selection
concept, potentially useful beyond paint. This is justified only if the user
must inspect, edit, serialize, or hand the region between systems after the
gesture. It carries the largest selection, lifecycle, visualization, and MMS
surface-area cost.

Compare the candidates using:

- one authoritative world/ray-to-grid-address conversion;
- deterministic cell generation independent of input event frequency;
- preview/commit identity and efficient incremental reconciliation;
- cancellation and cleanup guarantees;
- undo/redo transaction boundaries;
- desktop/XR parity;
- testability without a renderer;
- whether MMS needs to observe or construct the state; and
- whether the model remains useful outside these three tools.

## Investigation plan

- [x] Add opt-in gesture/Paint traces, in-world point markers, and a dedicated
      desktop/XR diagnostic scene so live investigation has observable inputs.
- [ ] Record live desktop and XR event traces for click, short drag, long drag,
      pointer cancellation, and a drag across adjacent renderables.
- [x] Reproduce Free Draw in the running editor and identify whether activation,
      raycast continuity, preview lifecycle, or commit is the first failure.
- [x] Record from source the current selected-grid inputs available at stroke start and on
      every move, including grid owner identity, world transform, axes, spacing,
      and enabled state.
- [x] Trace the current pointer-to-paint event payload and identify desktop/XR
      mapping differences.
- [x] Audit preview commit/cancellation paths and current test coverage.
- [ ] Prototype pure grid-address conversion and cell-generation examples on
      paper/tests before connecting them to ECS mutation.
- [ ] Work through horizontal, vertical, diagonal, shallow, steep, reversed,
      zero-length, fast-pointer, and repeated-cell paths.
- [ ] Compare the three ownership models against the same gesture traces.
- [ ] Decide whether a gesture captures an immutable grid frame or reacts to
      grid changes while active.
- [ ] Decide the surface continuity and reprojection policy separately from the
      grid cell-generation policy.
- [ ] Decide preview, atomic commit, cancellation, and future undo semantics.
- [x] Classify the narrow 0.8 MMS boundary provisionally as host/editor-internal
      gesture state plus ordinary serialized committed objects; reopen only if
      MMS must construct, observe, or replay strokes as values.
- [ ] Write the selected interaction contract and acceptance matrix before
      implementation begins.

## Minimum acceptance matrix for the eventual design

| Case | Free Draw | Line | Spray Can |
| --- | --- | --- | --- |
| Click/zero movement | exactly one cell/object | exactly one cell/object | explicit one-sample policy |
| Fast movement | no event-frequency holes | same final cells as slow movement | density/rate follows stated policy |
| Repeated address | no accidental duplicate | stable preview, no respawn churn | follows explicit repeat policy |
| Reverse movement | path policy is deterministic | same set in reverse order | distribution contract remains valid |
| Adjacent renderables | continues when surfaces are compatible | endpoint remains addressable | continues when surfaces are compatible |
| Grid disabled/deleted | cancels or freezes by stated policy | cancels or freezes by stated policy | cancels or falls back by stated policy |
| Preview versus commit | identical addresses and poses | identical addresses and poses | identical seeded result or no preview |
| Desktop versus XR | same address/path result | same endpoint/cell result | same brush contract |

Also cover translated, rotated, vertical, and non-unit-spacing grids, as well as
cell occupancy, serialization of committed objects, and cleanup of every
preview/helper on cancellation.

## Exit criteria for this discovery task

This task is complete when:

- Free Draw, Line, and Spray Can each have an agreed gesture-level contract;
- the relationship between active-grid selection and transient grid
  point/span/path/footprint state is explicit;
- one ownership model has been selected with its tradeoffs recorded;
- ray-to-grid addressing, path/footprint generation, surface continuity,
  preview, commit, cancellation, and undo boundaries are specified;
- the MMS boundary is classified as public, host-only, or internal; and
- implementation work can be split into focused tasks with deterministic
  acceptance tests.

## Out of scope

- reopening general selected-grid transform snapping without a reproduction;
- changing the Color tool;
- implementing Fill;
- selecting an ECS or MMS architecture in this document's initial pass; and
- implementing the three paint tools before the discovery exit criteria are
  met.

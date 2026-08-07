# Grid + Gizmo + Paint End-to-End UX and Test Matrix

Date: 2026-08-06

Status: active, untriaged detailed tracker for the
[`mittens-engine 0.8.0` editor grid and paint release gate](editor-grid-paint-0.8.0-release-gate.md)

Primary demo: `examples/bisket-vr-demo.mms`

Related:

- `docs/spec/grid-snapping.md`
- `docs/spec/grid-material.md`
- `docs/task/grid-tool-and-surface-placement-followups.md`
- `docs/task/grid-panel-select-delete-hide-and-gizmo.md`
- `docs/bugs/free-draw-paint-does-not-snap-to-grid-while-grid-tool-placement-does.md`
- `docs/bugs/paint-panel-free-draw-special-case.md`
- `docs/bugs/transform-gizmo-screen-size-varies-with-camera-distance.md`

## Outcome we want

Grid lines, transform gizmos, Free Draw, and Line should share one understandable
coordinate contract:

- the selected active grid defines the coordinate frame and spacing
- a visible line means the same boundary in rendering and snapping
- moving a cell-sized solid keeps its cell boundaries on grid lines
- translation changes only the axes the user is manipulating
- paint preview and commit produce the same pose
- paint remains continuous across adjacent pieces of scene geometry
- Free Draw supports a deliberate click and drag interaction
- Line has an explicit two-endpoint interaction and grid sampling policy

The test matrix at the end of this document is intended to be run directly in
`bisket-vr-demo`.

## Current findings

### 1. The reported cube/grid mismatch is real and deterministic

The current gizmo snap quantizes the selected transform's origin:

```text
grid-local x = round(x / spacing) * spacing
grid-local z = round(z / spacing) * spacing
```

`GridSystem::snap_point_preserving_plane_offset(...)` applies that rule to the
candidate transform origin. It has no knowledge of the selected object's bounds,
pivot, or original cell phase.

The voxel terrain intentionally uses cell-centered cubes. Its cubes are size `3`,
and each center is placed at:

```text
cell boundary + cube_size / 2
```

With the default grid spacing of `1`, terrain cube centers therefore have a
half-unit phase while their edges land on integer grid lines. As soon as a gizmo
snaps the cube's center/origin to an integer, its edges move to half-integers.

This is not floating-point drift. It is a mismatch between:

- intersection/origin snapping in the gizmo
- boundary/cell-centered layout in the voxel terrain

The same mismatch occurs with a centered `1 x 1 x 1` cube: placing its origin on a
grid line puts its edges halfway between adjacent lines.

Relevant code:

- `assets/components/floors/voxel_terrain.mms`
- `src/engine/ecs/system/grid_system.rs`
- `src/engine/ecs/system/gizmo_system.rs`

#### Terrain prefab alignment checkpoint

Completed on 2026-07-28:

- the prefab now computes an integer `terrain_origin_x/z`
- each cube is placed from an explicit `cell_min_x/z + cube_half`
- odd width/length configurations preserve whole-unit X/Z boundaries instead
  of choosing exact half-unit symmetry around the prefab origin
- `voxel_terrain_cube_xz_boundaries_land_on_whole_local_units` materializes an
  odd-width terrain and asserts every cube's X/Z minimum and maximum are whole
  prefab-local units

The current examples use even terrain dimensions and zero parent X/Z
translation, so this cleanup deliberately does not move their cubes. If
`bisket-vr-demo` still shows a thick grid line through a cube center before any
gizmo movement, the remaining mismatch is in the rendered grid frame/phase or
runtime grid transform, not the terrain prefab positions.

Manual follow-up confirms that untouched terrain cubes now appear correctly
aligned, but moving one with the snapped translation gizmo changes it to a
half-cell phase in X/Z. This confirms the failure boundary:

```text
authored terrain placement: aligned
first snapped gizmo translation: X/Z edges offset by 0.5
```

The terrain prefab is no longer on the critical path for that observed failure.
`GRID-01`, `GRID-06`, `GIZMO-01`, `GIZMO-02`, and `GIZMO-06` are.

### 2. Grid rendering and grid snapping do not yet use the same frame

Snap math transforms points into the selected grid's local matrix and uses the
`GridComponent.spacing`.

The fragment shader instead draws from `v_world_pos.xz`, with minor spacing
hard-coded to `1.0` and major spacing hard-coded to `8.0`. Consequently a
translated, rotated, or non-unit-spacing grid can display lines that do not
represent the coordinates returned by the snap helpers.

The default horizontal grid at the world origin hides most of this problem, but
it is an end-to-end correctness blocker for authored grids.

Relevant code:

- `assets/shaders/grid-square.frag`
- `assets/shaders/grid.vert`
- `src/engine/ecs/system/grid_system.rs`

### 3. Gizmo translation snaps both grid-plane axes

For every translation handle, gizmo snapping calls
`snap_point_preserving_plane_offset(...)`. That helper rounds both grid-local X
and Z, even if the user is dragging only one world/local axis.

This can change a coordinate the user did not manipulate. It is especially
surprising when:

- the object begins off-grid on the untouched axis
- the selected gizmo axis is not parallel to a grid-local axis
- the user drags along the grid normal

The eventual snap policy needs to preserve locked/unmanipulated degrees of
freedom explicitly.

### 4. Paint snapping does not use the selected active grid

Gizmo translation resolves the selected grid from
`EditorContextState.active_grid_owner_transform`.

Free Draw and Grid Tool use a different rule:

```text
current hit renderable -> owning grid -> snap_hit(...)
```

Therefore paint only requests a grid snap when the ray is hitting the grid's own
renderable. It does not use the selected grid to quantize a terrain or wall hit.

At present the live grid visual is also forced to:

- `SelectableComponent.enabled = false`
- `RaycastableComponent.enable = false`

That makes the documented “snap on shown grid hits only” path ineffective for a
normal committed grid. The UI, status text, gizmo path, and paint path currently
describe different meanings of “active grid”.

Relevant code:

- `src/engine/ecs/system/editor_paint_system.rs`
- `src/engine/ecs/system/grid_system.rs`
- `src/engine/ecs/system/editor/context.rs`

### 5. Free Draw is drag-only, and renderable continuity is too strict

`handle_free_draw_click(...)` is a no-op. A Free Draw result is only produced by:

1. `DragStart` creating a preview
2. zero or more accepted `DragMove` events updating it
3. `DragEnd` committing it

This makes a click or a pointer gesture that does not cross the drag threshold
appear broken.

During a drag, the system records the exact renderable hit at `DragStart`.
Every later Free Draw move is discarded if its renderable differs. This is a
likely cause of intermittent behavior in `bisket-vr-demo`, because:

- the terrain contains many separate cube renderables
- a pointer naturally crosses cube boundaries during one stroke
- a preview or other foreground renderable may become the closest later hit

Continuity should be based on the intended surface/scene policy, not raw
renderable identity.

Relevant code:

- `src/engine/ecs/system/editor_paint_system.rs`
- `src/engine/ecs/system/editor/paint_panel.rs`

### 6. Free Draw activation depends on panel focus and asset selection

Paint is active only while the Paint or Color panel is the recorded focused
panel, the tool is recognized, and an asset is selected. Clicking another panel
can disable scene painting without changing the visible tool selection.

This may be intentional as an input-routing rule, but it needs visible state and
tests because it contributes to “works sometimes” reports.

### 7. Line is exposed but not implemented

`PaintTool::Line` exists and the Paint panel exposes `Line`, but:

- click handler: no-op
- stroke-move handler: no-op
- preview-session start: unsupported
- activity/status gate: rejects Line as unsupported
- an existing test explicitly expects Line not to place anything

Line should be treated as unimplemented, not as a partially working grid tool.

## Snap contract decision

### Recommended user-facing model

Use three explicit inputs for every snapped operation:

1. **Grid frame**: selected active grid transform and spacing.
2. **Operation constraint**: gizmo axis, surface contact, or line segment.
3. **Snap anchor**: the point or phase on the manipulated object that should
   align to the grid.

Do not implicitly equate “object transform origin” with “the part the user
expects to align”.

### Translation snap anchor options

These policies should be evaluated before implementation:

| Policy | Result | Strength | Weakness |
|---|---|---|---|
| Absolute pivot | Origin lands on intersections | Simple; current behavior | Centered odd-cell solids straddle lines |
| Incremental delta | Drag delta is a spacing multiple | Preserves existing terrain phase | An initially misaligned object stays misaligned |
| Bounds anchor | A chosen bounds corner/face lands on lines | Matches cell/block editing | Needs stable bounds and rotation rules |
| Cell center | Origin lands at half-cell centers | Natural for `1 x 1 x 1` voxels | Wrong for intersection-authored objects |

Recommended MVP:

- default solid-object gizmo snapping to **bounds anchor**
- retain an internal **absolute pivot** mode for transforms that deliberately use
  intersection pivots
- consider an **incremental delta** modifier later

For the reported terrain case, bounds-anchor snapping makes the cube edge stay on
the line regardless of whether the cube is one or three cells wide.

### Axis constraint rule

Snapping must not change a degree of freedom that the active gizmo operation did
not change.

Examples:

- grid-local X translation snaps X and preserves grid-local Z
- grid-normal translation preserves both grid-local in-plane coordinates
- an oblique world-axis translation snaps only the scalar displacement along the
  chosen axis, rather than independently rounding two coordinates after the fact

### Paint snap rule

Recommended paint semantics:

- the selected active grid supplies in-plane coordinate quantization
- the raycast scene surface supplies contact, normal, and off-plane placement
- visibility is presentation; enabled/selected state controls whether the grid
  participates in snapping
- paint must not require the grid visual itself to win the raycast

This is a hybrid of the current two helpers. It is not identical to either:

- `snap_hit(...)` forces contact onto the grid plane
- `snap_point_preserving_plane_offset(...)` preserves height but does not
  re-resolve contact with scene geometry

For a flat voxel top surface, the first useful version can snap grid-local X/Z
while retaining the hit height and hit surface normal. Arbitrary curved or
non-coplanar targets need an explicit projection/re-raycast policy later.

## UX use cases

### Transform gizmo

- Move a one-cell cube by one or more grid cells while its edges remain on lines.
- Move a multi-cell block without changing its established grid phase.
- Move an object along grid X without changing grid Z.
- Move an object along the grid normal without lateral snapping.
- Use a translated or rotated grid and see the gizmo snap to the displayed lines.
- Disable or deselect a grid and get continuous, unsnapped movement.
- Select a different grid and immediately use that grid's frame and spacing.

### Free Draw

- Click once to place one asset, or deliberately document that click placement is
  unavailable and show a drag affordance. The recommended behavior is one click =
  one placement.
- Drag within one surface and keep a stable preview.
- Drag across adjacent terrain cubes without the preview freezing.
- Cross from one coplanar renderable to another without losing the stroke.
- Snap against the selected grid while raycasting the terrain, not the grid
  visual.
- Keep the asset in contact with the hit surface while its in-plane anchor snaps.
- Make preview pose exactly match committed pose.
- Show an explicit inactive reason for missing asset, unsupported tool, or lost
  paint focus.

### Line

- First endpoint starts a line preview.
- Pointer movement updates the second endpoint.
- Commit places a deterministic sequence of assets at grid steps.
- Cancel removes the preview and places nothing.
- Endpoints snap in the selected grid frame.
- Duplicate endpoint cells are emitted once.
- Crossing adjacent renderables does not interrupt a valid coplanar line.
- A surface policy is explicit:
  - planar line on the initial surface for MVP, or
  - per-sample projection onto scene geometry as a later mode.

Recommended Line MVP:

- drag start = endpoint A
- drag move = preview endpoint B
- drag end = commit
- selected grid required
- sample the dominant grid-local segment with a 2D integer line algorithm
- place one asset per unique grid cell
- use the initial valid surface frame for orientation/contact

## Prioritized dependency plan

Priority is based on dependency and verification value, not only severity. A
later wave should not start until its listed gate is satisfied, except for the
explicit parallel lanes.

```text
baseline + policy
        |
        v
shared grid/snap contract ------------------+
        |                                   |
        v                                   v
visual grid agreement                 paint lifecycle stability
        |                                   |
        v                                   v
gizmo anchor + axis behavior          selected-grid paint snap
        |                                   |
        +----------------+------------------+
                         v
                  end-to-end demo gate
                         |
                         v
                    Line tool MVP
```

### Wave 0: preserve a baseline and stop presenting known no-ops

These tasks have no implementation prerequisites.

| Order | Tasks | Depends on | Exit gate |
|---|---|---|---|
| 0.1 | Run and record `M-01` through `M-17` as `TEST-00` | none | Every row has a baseline result and reproduction notes |
| 0.2 | `UX-04`: visibly disable Line while it is a no-op | none | Line cannot be mistaken for a working tool |
| 0.3 | Prepare regression tests for `A-01` through `A-04`, `A-08`, `A-10`, and `A-12` | `TEST-00` for expected behavior | Each reported issue has a test to land with its fix or an explicitly documented test gap |
| 0.4 | [x] `TERRAIN-01`: make whole-unit cell boundaries explicit in the prefab | none | Odd and even terrain dimensions retain whole-unit X/Z cell boundaries |
| 0.5 | `GIZMO-SIZE-01`: trace the desktop constant-angular-size regression | none | Calculation, propagation, or renderer-cache failure is identified |

Do not “fix” the existing Line no-op test in this wave. Keep it as an accurate
description of current runtime behavior until the Line implementation wave.

### Wave 1: lock policy decisions

These are short design decisions that unblock the shared API and prevent each
tool from inventing different snapping semantics.

| Order | Tasks | Depends on | Exit gate |
|---|---|---|---|
| 1.1 | `GRID-01`: choose the default gizmo snap anchor | baseline | Bounds anchor is accepted or replaced by a recorded alternative |
| 1.2 | `GRID-03`: define selected, enabled, and visible grid responsibilities | baseline | One state table covers gizmo, paint, rendering, and raycasting |
| 1.3 | `GRID-04`: define active-grid quantization plus scene-surface contact | `GRID-03` | Flat, rotated-planar, and non-planar behavior is specified |
| 1.4 | `PAINT-00`: define stroke continuity across renderables and surfaces | baseline | Same surface, coplanar neighbor, preview hit, and invalid target have explicit outcomes |
| 1.5 | `GRID-02`: decide whether incremental snapping is in the MVP | `GRID-01` | Recorded as MVP or explicitly deferred |

`GRID-02` must not block bounds-anchor snapping if incremental snapping is
deferred.

### Wave 2: introduce the shared snapping foundation and observability

| Order | Tasks | Depends on | Exit gate |
|---|---|---|---|
| 2.1 | `GRID-05`: expose active grid id, frame, spacing, snap mode/anchor, and last cell in debug output | `GRID-01`, `GRID-03` | Manual failures can be reported in grid-local coordinates |
| 2.2 | `GRID-06`: introduce a shared snap request/result carrying grid frame, operation constraint, and anchor | `GRID-01`, `GRID-03`, `GRID-04` | Gizmo and paint can call the same policy layer without sharing interaction code |
| 2.3 | Add pure math coverage `A-01` through `A-07` | `GRID-06` | Anchor, axis preservation, transform, and spacing cases pass |

`GRID-06` should keep surface contact outside the low-level quantizer. The
quantizer owns grid coordinates; paint placement owns scene contact and normals.

### Wave 3A: make visible grid lines authoritative

This lane can run in parallel with Wave 3B after Wave 2.

| Order | Tasks | Depends on | Exit gate |
|---|---|---|---|
| 3A.1 | `GRID-10`: use grid-local shader coordinates | `GRID-06` frame convention | The grid origin and rotation affect the line pattern |
| 3A.2 | `GRID-11`: pass `GridComponent.spacing` to the grid material | `GRID-06` spacing convention | Visual minor lines and snap steps share spacing |
| 3A.3 | `GRID-12`, `GRID-13`: verify translated and rotated grid rendering | `GRID-10`, `GRID-11` | Debug snap markers land on rendered lines |
| 3A.4 | `GRID-14`, `A-17`: retain visual regression coverage | `GRID-12`, `GRID-13` | Origin, translation, rotation, and non-unit spacing cases pass |

### Wave 3B: stabilize Free Draw before adding grid snapping

This lane deliberately fixes input lifecycle and continuity before changing
placement coordinates. Otherwise a snap change can mask or complicate the
intermittent-drag failures.

| Order | Tasks | Depends on | Exit gate |
|---|---|---|---|
| 3B.1 | `PAINT-05` + `UX-05`: make focus and inactive reasons deterministic and visible | `GRID-03` state policy | The UI always explains why a gesture will not paint |
| 3B.2 | `PAINT-01`: make click place once | baseline click characterization | `A-08` passes without producing a duplicate drag placement |
| 3B.3 | `PAINT-03`: keep previews out of stroke raycasts | `PAINT-00` | `A-11` passes |
| 3B.4 | `PAINT-02`: replace exact renderable identity with the continuity policy | `PAINT-00`, `PAINT-03` | `A-10` passes across adjacent voxel cubes |
| 3B.5 | `PAINT-07`, `PAINT-08`: lock preview/commit equality and voxel regression coverage | `PAINT-01` through `PAINT-03` | `A-09` through `A-11` pass |
| 3B.6 | `PAINT-06`: normalize desktop/XR gesture lifecycle | stable desktop lifecycle | Lifecycle parity is demonstrated before coordinate snapping is added |

### Wave 4A: fix gizmo snapping

This begins after the shared math exists. It may overlap late Free Draw work, but
its manual verification waits for the visual-grid gate in Wave 3A.

| Order | Tasks | Depends on | Exit gate |
|---|---|---|---|
| 4A.1 | `GIZMO-01` + `GIZMO-02`: apply the chosen anchor and preserve unmanipulated coordinates | Wave 2 | `A-01` through `A-04` pass through the gizmo path |
| 4A.2 | `GIZMO-03`: define grid-normal translation | `GIZMO-02` | `M-04` passes |
| 4A.3 | `GIZMO-06`: remove the first-move phase jump | `GIZMO-01`, `GIZMO-02` | `M-02`, `M-03`, and `M-05` pass |
| 4A.4 | `GIZMO-04`, `GIZMO-05`: cover coordinate spaces and parent transforms | Wave 3A, prior gizmo tasks | Rotated-grid and parented-target cases pass |

### Wave 4B: connect stable Free Draw to the selected grid

| Order | Tasks | Depends on | Exit gate |
|---|---|---|---|
| 4B.1 | `PAINT-04`: quantize from the selected active grid while retaining scene contact | `GRID-03`, `GRID-04`, `GRID-06`, Wave 3B | `A-12` through `A-14` pass |
| 4B.2 | Verify preview/commit after snapping | `PAINT-04`, `PAINT-07` | `M-13` and `M-14` pass |
| 4B.3 | `UX-01`, `UX-02`, `UX-03`: expose grid, snap anchor/mode, and state distinctions | stable grid and paint contracts | UI state agrees with runtime state in every matrix row |

### Wave 5: end-to-end gate before Line

Run the complete automated matrix except Line rows, then run:

- `M-01` through `M-16`
- `M-18` desktop/XR parity
- translated, rotated, hidden, disabled, and deselected grid variants

Do not start Line until:

- visible lines agree with snap coordinates
- gizmo cell alignment is stable
- Free Draw click, drag, continuity, snap, and commit behavior pass

This keeps Line as a consumer of proven primitives instead of a third custom
implementation.

### Wave 6: implement Line using the shared primitives

| Order | Tasks | Depends on | Exit gate |
|---|---|---|---|
| 6.1 | `LINE-01`, `LINE-02`, `LINE-05`: endpoint state, preview, cancel/invalid handling | Wave 5 | A line can preview and cancel without placement |
| 6.2 | `LINE-03`: snap endpoints through the shared selected-grid policy | `LINE-01`, `PAINT-04` | Endpoint cells match Free Draw coordinates |
| 6.3 | `LINE-04`: generate unique ordered cells | `LINE-03` | Pure line-cell tests cover horizontal, vertical, diagonal, reverse, and zero-length cases |
| 6.4 | `LINE-06`: commit samples as one editor operation | `LINE-02`, `LINE-04`, `LINE-05` | Commit count/order is deterministic and undo is atomic |
| 6.5 | `LINE-07`: replace the no-op test and enable the UI | all Line tasks | `A-15`, `A-16`, and `M-17` pass |

### Critical path

The shortest dependency chain to resolve the reported terrain-cube problem is:

```text
TEST-00
  -> GRID-01
  -> GRID-06
  -> GIZMO-01 + GIZMO-02
  -> GIZMO-06
  -> M-02 + M-03 + M-05
```

The shortest dependency chain to make grid-snapped Free Draw reliable is:

```text
TEST-00
  -> GRID-03 + GRID-04 + PAINT-00
  -> GRID-06
  -> PAINT-01 + PAINT-03 + PAINT-02
  -> PAINT-04
  -> M-10 through M-14
```

## Implementation tracker

### P0: establish and test the coordinate contract

- [ ] `GRID-01` Decide and record the default gizmo snap anchor.
- [ ] `GRID-02` Define whether incremental snap is a separate modifier/mode.
- [ ] `GRID-03` Define selected/enabled/visible grid responsibilities.
- [ ] `GRID-04` Define paint contact behavior away from the grid plane.
- [ ] `GRID-05` Add a small debug readout for active grid id, spacing, frame,
  snap mode, snap anchor, and last snapped cell.
- [ ] `GRID-06` Introduce a shared snap request/result carrying grid frame,
  operation constraint, and anchor semantics.
- [ ] `PAINT-00` Define continuity across same, adjacent, preview, and invalid
  renderable hits.
- [ ] `TEST-00` Record the initial manual-demo baseline.
- [x] `TERRAIN-01` Make each terrain cell's X/Z minimum corner an explicit
  whole-unit prefab-local coordinate and cover odd dimensions.
- [ ] `GIZMO-SIZE-01` Resolve
  `docs/bugs/transform-gizmo-screen-size-varies-with-camera-distance.md` before
  relying on manual gizmo hitbox/size verification.

### P0: fix visual/snap agreement

- [ ] `GRID-10` Feed grid-local coordinates to the grid shader.
- [ ] `GRID-11` Feed `GridComponent.spacing` to the material/shader.
- [ ] `GRID-12` Make translated grid lines move with the grid transform.
- [ ] `GRID-13` Make rotated grid lines lie in the rendered grid plane.
- [ ] `GRID-14` Add visual regression coverage for origin, translation,
  rotation, and spacing.

### P0: gizmo translation

- [ ] `GIZMO-01` Implement the chosen snap anchor instead of always snapping the
  transform origin.
- [ ] `GIZMO-02` Preserve unmanipulated coordinates.
- [ ] `GIZMO-03` Define and test grid-normal translation.
- [ ] `GIZMO-04` Test world-space and local-space gizmo axes against rotated
  grids.
- [ ] `GIZMO-05` Test parented, rotated, and scaled targets.
- [ ] `GIZMO-06` Make drag start and first snapped move free of discontinuous
  jumps.

### P0: stabilize Free Draw

- [ ] `PAINT-01` Make a click place one asset, or remove click affordance and
  clearly expose drag-only behavior.
- [ ] `PAINT-02` Replace exact `captured_renderable` equality with an explicit
  stroke surface-continuity policy.
- [ ] `PAINT-03` Ensure the preview cannot steal later stroke raycasts.
- [ ] `PAINT-04` Resolve snapping from the selected active grid while retaining
  scene-hit contact.
- [ ] `PAINT-05` Make focus/activation state visible and deterministic.
- [ ] `PAINT-06` Ensure pointer/XR drag thresholds generate the same lifecycle.
- [ ] `PAINT-07` Assert preview pose equals committed pose.
- [ ] `PAINT-08` Add regression coverage for dragging across voxel renderables.

### P1: implement Line

- [ ] `LINE-01` Add line interaction state and endpoint capture.
- [ ] `LINE-02` Add a non-raycastable line/asset preview.
- [ ] `LINE-03` Snap endpoints in selected grid-local space.
- [ ] `LINE-04` Generate unique grid cells along the segment.
- [ ] `LINE-05` Define cancellation and invalid-target behavior.
- [ ] `LINE-06` Commit all samples as one editor operation/undo unit.
- [ ] `LINE-07` Replace the existing Line no-op test with endpoint, preview, and
  commit tests.

### P1: feedback and controls

- [ ] `UX-01` Display active grid and spacing in the Grid/Paint UI.
- [ ] `UX-02` Display snap on/off and anchor mode.
- [ ] `UX-03` Distinguish `hidden`, `disabled`, and `not selected for snap`.
- [ ] `UX-04` Mark unimplemented tools disabled instead of selectable no-ops.
- [ ] `UX-05` Show why paint is inactive without relying on debug logs.

## Automated test matrix

| ID | Layer | Scenario | Required assertion |
|---|---|---|---|
| A-01 | unit | 1-unit centered cube, bounds snap | min/max X/Z are integer grid coordinates |
| A-02 | unit | 3-unit centered cube, bounds snap | min/max X/Z remain on integer lines |
| A-03 | unit | grid-local X gizmo drag | Z and plane offset are unchanged |
| A-04 | unit | grid-normal gizmo drag | both in-plane coordinates are unchanged |
| A-05 | unit | translated grid | snapped world point maps to integer grid-local coordinates |
| A-06 | unit | rotated grid | snapped world point maps to integer grid-local coordinates |
| A-07 | unit | spacing `0.5`, `1`, `3` | visual/snap cell indices use the same spacing |
| A-08 | integration | Free Draw click | exactly one committed asset |
| A-09 | integration | Free Draw drag on one renderable | preview updates and one asset commits |
| A-10 | integration | drag across two coplanar renderables | preview continues and commits at final pose |
| A-11 | integration | preview becomes ray candidate | preview does not interrupt captured stroke |
| A-12 | integration | active grid + ordinary terrain hit | in-plane placement is snapped |
| A-13 | integration | hidden active grid | behavior matches the documented visibility policy |
| A-14 | integration | disabled active grid | placement is unsnapped |
| A-15 | integration | Line A to B | expected unique ordered grid cells are emitted |
| A-16 | integration | Line cancel | no assets remain |
| A-17 | visual | translated/rotated/non-unit grid | rendered lines coincide with debug snap markers |
| A-18 | parity | desktop mouse vs XR pointer | equivalent gestures produce equivalent placements |

## `bisket-vr-demo` manual verification matrix

### Setup

1. Launch `bisket-vr-demo`.
2. Open the Grid panel.
3. Show and select the initial/default grid.
4. Confirm its spacing and transform in the inspector/debug readout.
5. Use the voxel terrain cubes as the primary alignment reference.
6. Repeat interaction rows with desktop pointer and XR controller where
   applicable.

Record `Pass`, `Fail`, or `Blocked`, plus the observed world/grid-local
coordinates when a row fails.

| ID | Workflow | Action | Expected result | Baseline on 2026-07-28 |
|---|---|---|---|---|
| M-01 | visual | Inspect untouched terrain cube edges | Edges coincide with visible grid lines | Pass: visually confirmed after prefab cleanup |
| M-02 | gizmo X | Move one terrain cube along X | X edges stay on lines; Z does not change | Confirmed fail: first snapped move changes to half-cell X/Z phase |
| M-03 | gizmo Z | Move one terrain cube along Z | Z edges stay on lines; X does not change | Confirmed fail: first snapped move changes to half-cell X/Z phase |
| M-04 | gizmo Y | Move cube vertically | X/Z do not change | At risk: helper rounds both grid-plane axes |
| M-05 | gizmo first move | Begin drag from an aligned cube | No half-cell jump at drag start | Known fail for cell-centered origin |
| M-06 | no active grid | Clear active grid, repeat move | Smooth unsnapped translation | Verify |
| M-07 | disabled grid | Disable selected grid, repeat move | Smooth unsnapped translation | Verify |
| M-08 | translated grid | Translate grid, then move cube | Cube aligns to displayed translated lines | Known risk: shader uses world XZ |
| M-09 | rotated grid | Rotate grid, then move cube | Cube follows displayed rotated frame | Known risk: shader uses world XZ |
| M-10 | Free Draw click | Select asset and click terrain once | One asset is placed | Known fail: click handler is no-op |
| M-11 | Free Draw short drag | Drag without leaving one cube | Stable preview commits at final pose | Verify |
| M-12 | Free Draw cross-cube | Drag across adjacent terrain cubes | Preview follows continuously | Known fail risk: exact renderable capture |
| M-13 | Free Draw + active grid | Drag on terrain while grid selected | Placement anchor quantizes to selected grid | Known fail: paint only snaps grid renderable hits |
| M-14 | Free Draw preview/commit | Release after moving preview | Committed pose exactly matches preview | Verify |
| M-15 | Free Draw focus | Focus another panel, then paint | UI clearly reports inactive state or tool retains deliberate capture | Current focus dependency is easy to miss |
| M-16 | Free Draw no asset | Clear asset selection and paint | No placement; clear “no asset selected” feedback | Verify |
| M-17 | Line | Select Line and drag A to B | Preview then one asset per unique grid cell | Known fail: tool is unimplemented |
| M-18 | parity | Repeat M-02, M-10–M-14 in XR | Same coordinate and lifecycle results | Verify |

## Definition of done

- A visible grid line and a snap boundary are the same coordinate for every grid
  transform and spacing.
- Cell-aligned cubes remain cell-aligned after gizmo translation.
- A one-axis gizmo operation cannot alter an unrelated coordinate.
- Free Draw click and drag behavior are both deliberate, visible, and tested.
- Free Draw continues across adjacent voxel renderables.
- Paint uses the selected grid without requiring the grid visual to intercept the
  pointer.
- Line is either implemented and enabled or visibly disabled as unavailable.
- Preview and commit agree.
- Desktop and XR results pass the same `bisket-vr-demo` matrix.

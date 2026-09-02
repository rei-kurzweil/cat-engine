# Editor grid and paint 0.8.0 release gate

Date: 2026-08-06

Status: active, untriaged `mittens-engine 0.8.0` release gate

Desktop planning entry: [Editor, grids, rasterized placement, and paint](../desktop/editor-grid-paint.md).

Live reproduction aid:
[Paint-stroke live diagnostics](../how_to/paint-stroke-live-diagnostics.md).
Desktop-only counterpart:
[Paint-grid stroke cell deduplication and desktop repro](paint-grid-stroke-cell-deduplication-and-desktop-repro.md).

## Purpose

Restore the editor's object-creation workflow before publishing
`mittens-engine 0.8.0`, and consolidate the release-critical parts of the
existing grid, cursor, gizmo, asset-selection, and paint investigations.

This is the short gate. The detailed coordinate decisions and test cases remain
in
[Grid + Gizmo + Paint end-to-end UX and test matrix](grid-gizmo-paint-end-to-end-ux-and-test-matrix.md).

## Current implementation checkpoint

- Grid creation, enumeration, selection, snapping helpers, surface placement,
  previews, Free Draw, Grid Tool, Spray Can, and Erase all have implementation
  slices and focused tests.
- The running application reportedly no longer completes the basic
  asset-selection -> paint-state -> visible Free Draw workflow.
- In `vtuber-mirror-example`, most temple geometry was not raycastable; only
  the three emissive test cubes could produce paint/color scene hits. The main
  floor and two placement platforms are now explicit paint targets.
- Grid rendering, active-grid selection, paint snapping, and gizmo snapping do
  not yet share one authoritative frame/spacing/anchor contract.
- Several grid panel, cursor routing, orientation, preview, and placement bugs
  remain open across overlapping task and bug documents.
- `PaintTool::Line` is exposed by the panel. General paint activation can accept
  it, the status path calls it unsupported, and its effect/preview handlers are
  no-ops; the current test asserts that Line places nothing.
- Color selection is shared paint state. Placement tools use it for new assets;
  the Color tool applies a renderable-local tint to only the raycast-hit
  primitive. `Fill` remains a distinct, unsupported tool and is not exposed in
  the 0.8 Paint panel.
- The engine library suite currently contains deterministic catalog drift plus
  broader editor/paint failures, so a compiling workspace is not sufficient
  release evidence.

## User journey gate

The following sequence must work in a normal desktop session and in the
supported XR editor path:

1. open or focus an editor;
2. select an asset in the Assets panel;
3. focus Paint and choose Free Draw or Line;
4. select or create a grid when the chosen tool requires one;
5. point/click/drag on valid scene geometry;
6. see a stable preview and a committed, selectable object at the same pose;
7. inspect or manipulate the result without losing the active grid or paint
   state unexpectedly.

The UI must show why painting is inactive: missing panel focus, asset, grid,
editor, valid hit, or supported tool. A silent no-op is a release failure.

## Gate A: restore paint activation

Tracked in detail by
[Asset selection and paint runtime regression](asset-selection-paint-runtime-regression-followup.md).

- [ ] Reproduce the current failure in the running application and record the
      exact click/focus sequence.
- [x] Make the intended `vtuber-mirror-example` floor/platform targets
      raycastable; a non-raycastable surface cannot start a paint gesture.
- [ ] Trace asset row `Option -> Data(asset_key)` through shared paint state,
      template lookup, stroke start, spawn, attach, and render registration.
- [ ] Make a selected asset remain visibly selected and available to the paint
      system when the Paint panel receives focus.
- [ ] Make one Free Draw click place exactly one object.
      Test this through `GestureSystem` (`DragStart -> DragEnd -> Click`); the
      terminal `Click` handler must not create a duplicate.
- [ ] Make a Free Draw drag remain active across adjacent compatible
      renderables instead of keying continuity to one raw renderable ID.
- [ ] Keep preview geometry out of subsequent stroke raycasts.
- [ ] Ensure preview pose and committed pose are identical and preview opacity
      is removed on commit.
- [ ] Add an integration-style test that crosses selection, focus, paint state,
      template resolution, placement, and visible/render registration.

## Gate A2: make object coloring explicit

The Color panel selects an RGBA operand; it does not perform an action by
itself. The selected color has two consumers:

1. Free Draw, Line, and Spray Can apply it while creating a new asset; and
2. the Color tool updates or attaches the immediate `ColorComponent` on the
   raycast-hit `RenderableComponent`.

- [x] Expose `Color` as a Paint-panel tool with an RGB icon; retain `Fill` as a
      distinct hidden tool value.
- [x] Do not require an asset selection to activate Color.
- [x] Do not require an active grid to use Color.
- [x] Limit Color to the hit renderable so sibling primitives are unchanged.
- [x] Attach a renderable-local tint when the hit has no immediate color.
- [x] Re-register changed color components so the visible render state updates.
- [x] Cover recoloring an existing hit without placing a new asset in a focused
      system test.
- [ ] Verify desktop and XR Color clicks in the running application.
- [ ] Decide a separate material/texture-paint design for textured GLTF assets;
      the 0.8 object-color tool does not rewrite textures or material graphs.

## Deferred Fill contract

Implement Line before beginning Fill. Fill also depends on an investigation
and implementation of an accelerated spatial-query/index facility that can
answer occupancy at grid-quantized world positions without persisting a second
grid-sized occupancy map.

Fill requires a selected asset, snapping enabled, and a selected enabled grid.
A click begins at one grid-local cell and expands through four-connected empty
cells. Occupied cells are walls, while grid edges do not count as enclosure.
The implementation advances one Manhattan-distance frontier layer per
`FrameTick`, keeping only transient frontier and visited state so the fill is
visibly animated.

Each explored cell queries current spatial occupancy, naturally reflecting
moved and deleted objects, and spawns a preview asset. If the frontier escapes
the enclosure or exceeds a safety limit, every preview is removed and nothing
is committed. If the region is enclosed, all previews are committed together
as one editor operation.

## Gate B: make grids authoritative and usable

- [ ] Establish one active-grid record containing grid identity, transform
      frame, plane axes, spacing, enabled/visible state, and snap anchor/mode.
- [ ] Render minor/major lines in grid-local coordinates using the same origin,
      rotation, and spacing consumed by snapping.
- [ ] Make Add Grid and Grid Tool create the intended horizontal or
      surface-aligned plane instead of relying on mismatched authored/render
      axes.
- [ ] Make committed Grid Tool previews ordinary registered grids and rerender
      the Grid panel immediately.
- [ ] Make grid row selection target the owning transform and activate the
      normal transform gizmo without leaving the grid as the only selectable
      scene target.
- [ ] Make show/hide, enable/disable, delete, selected state, and snap
      participation distinct and visible.
- [x] Persist an object's chosen grid with `GridBinding`, prefer it over the
      workspace-selected grid, and remove dependent bindings when a grid is
      deleted.
- [x] Keep hidden enabled grids snap-active, preserve grid selection across
      hide/disable, and pause authoritative bindings while disabled.
- [x] Retain the exact grid used by a snapped paint preview on the committed
      object's manipulation wrapper.
- [ ] Make the 3D cursor and grid placement resolve the correct editor root and
      work without first selecting a GLTF bone marker.
- [ ] Quantize only the degrees of freedom manipulated by a gizmo or paint
      operation; preserve untouched coordinates and established object phase.
- [ ] Verify translated, rotated, non-unit-spacing, horizontal, and vertical
      grids.

## Gate C: selected-grid paint snapping

Use three explicit inputs:

1. selected grid frame and spacing;
2. operation constraint or surface contact; and
3. the object's snap anchor.

- [ ] A grid visual does not have to win the scene raycast for the selected
      grid to quantize a scene-surface hit.
- [ ] Decide whether Paint addresses grid-line intersections, cell centers, or
      an address plus explicit anchor phase; use the resulting grid-local
      address as the durable deduplication unit.
- [ ] Paint retains the scene hit's contact/normal policy while quantizing only
      the selected grid's in-plane coordinates.
- [ ] Make asset placement follow the selected paint-lattice/anchor policy;
      do not silently mix the current intersection-based `GridStep.cell` with
      half-cell-centered placement.
- [ ] A placement cannot emit two committed objects for the same grid/cell key
      within one stroke.
- [ ] Snapping behavior and inactive reasons are visible in Paint status/UI.

## Gate D: Line tool MVP

Line is a consumer of the stable grid and paint primitives. Do not implement a
third private snapping model.

The lifecycle and cell-generation details below are a working hypothesis, not
an accepted architecture. Validate them alongside Free Draw and Spray Can in
[Grid-aware paint stroke interaction model](grid-aware-paint-stroke-interaction-model.md)
before implementation. In particular, first decide whether grid endpoints and
paths are tool-local gesture state, a shared transient grid-gesture model, or a
general editor grid-subselection model.

### Interaction lifecycle

- `DragStart`: require an active editor, selected asset, selected enabled grid,
  and valid start hit; capture grid identity/frame/spacing and integer start
  cell; begin a preview set.
- `DragMove`: resolve the current endpoint in the captured grid frame,
  quantize it to one integer cell, generate the ordered cells between start and
  end, and reconcile the preview set without committing duplicates.
- `DragEnd`: perform one final reconciliation and commit exactly one asset for
  every unique generated cell.
- cancellation, focus loss, grid deletion/disable, invalid handle, or an
  unrecoverable hit transition removes previews and commits nothing.

The MVP keeps the grid captured at `DragStart`; moving over a different grid
does not silently switch coordinate systems mid-stroke.

### Cell generation

- [ ] Include both endpoints; a zero-length line produces one cell.
- [ ] Use a deterministic integer 2D line/supercover policy in the grid plane.
- [ ] Cover horizontal, vertical, diagonal, shallow, steep, reversed, and
      zero-length segments.
- [ ] Deduplicate by `(grid identity, cell_u, cell_v)` while preserving stable
      start-to-end order.
- [ ] Place each asset using the paint lattice/anchor policy selected by the
      discovery task, then transform the resulting grid-local point to
      world/parent space.
- [ ] Use the initial valid surface frame for MVP orientation/contact;
      per-sample reprojection onto arbitrary curved geometry is deferred.
- [ ] Preview and committed assets use the same generated cell set and pose
      calculation.
- [ ] Commit the stroke as one editor operation so future undo can remain
      atomic.

### Line acceptance tests

- [ ] Replace the Line no-op test with Line lifecycle tests.
- [ ] Assert no duplicate grid cell or duplicate committed translation.
- [ ] Assert reverse drags produce the same cell set in reverse order.
- [ ] Assert repeated `DragMove` for the same endpoint does not respawn or
      accumulate previews.
- [ ] Assert cancellation and lost/disabled grid commit nothing.
- [ ] Assert desktop and XR gesture streams produce the same grid cells.

## Release verification

- [ ] Run focused grid, gizmo, cursor, selection, placement-preview, and paint
      tests without relying on shared global state from unrelated tests.
- [ ] Run the complete engine library suite and classify/fix any remaining
      failures.
- [ ] Exercise the matrix in `examples/bisket-vr-demo.mms` and at least one
      simpler editor scene.
- [ ] Verify both desktop pointer and XR pointer interaction.
- [ ] Confirm created objects serialize as authored scene content and exclude
      editor/preview helpers.

## Related issue inventory

Release-critical trackers:

- [Grid + Gizmo + Paint end-to-end UX](grid-gizmo-paint-end-to-end-ux-and-test-matrix.md)
- [Asset selection and paint runtime regression](asset-selection-paint-runtime-regression-followup.md)
- [Grid Tool and surface placement follow-ups](grid-tool-and-surface-placement-followups.md)
- [Grid panel select/delete/hide/gizmo](grid-panel-select-delete-hide-and-gizmo.md)
- [Unified grid snap mode](unified-grid-snap-mode-mms-gizmo-and-paint.md)
- [Grid-aware paint stroke interaction model](grid-aware-paint-stroke-interaction-model.md)

Focused bugs:

- [Editor cursor, GLTF, and grid alignment](../bugs/editor-cursor-3d-gltf-and-grid-alignment.md)
- [Free Draw does not snap to grid](../bugs/free-draw-paint-does-not-snap-to-grid-while-grid-tool-placement-does.md)
- [Grid Tool leaves grid as only selectable target](../bugs/grid-tool-leaves-grid-as-only-selectable-target-and-grid-drags-rotate-gizmo.md)
- [Grid panel does not refresh](../bugs/grid-panel-does-not-refresh-after-grid-tool-placement.md)
- [Paint asset selection not synced](../bugs/paint-panel-asset-selection-not-synced.md)

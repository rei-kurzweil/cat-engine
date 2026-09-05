# Free Draw cannot start a stroke on an empty grid's analytic plane

## Summary

`Free Draw` can project an already-active stroke onto a grid, but it cannot begin that stroke by
clicking or pressing the trigger on an otherwise empty part of the grid. A stroke only starts when
the initial `DragStart` ray hits an existing renderable, such as an object painted by an earlier
stroke. This makes painting an empty grid depend on prior painted scene geometry.

## Reproduction

1. Open an editor scene with a visible grid and select `Free Draw` plus a paintable asset.
2. Ensure there are no renderables over the part of the grid to be painted.
3. Click and drag with the desktop pointer, or press and move the XR trigger, beginning on that
   empty grid area.
4. Observe that no paint stroke begins.
5. Paint or otherwise place a renderable on the grid, then begin the drag on that renderable.
6. Observe that painting starts and can continue across the grid while the pointer/trigger remains
   held.

## Current behavior

- The grid's analytic-plane intersection is available for continuation of a captured paint drag.
- It does not provide a hit that can initiate the initial `DragStart` on an empty grid area.
- The initial press therefore needs a raycastable scene renderable under it; an existing painted
  object happens to satisfy that requirement.
- Releasing the pointer/trigger ends the stroke as expected, but the next stroke has the same
  renderable-at-start requirement.

## Expected behavior

- With an active, visible paint grid, users can begin a Free Draw stroke anywhere inside that
  grid's finite bounds, even when no scene renderables exist there.
- The same grid plane should supply both the initial stroke point and later drag-projection points.
- Starting a paint stroke from a grid surface must not select, move, rotate, or otherwise treat the
  grid as a gizmo target.
- Outside the finite grid bounds, normal no-surface behavior should remain unchanged.

## Likely investigation

The gesture pipeline appears to require a raycast hit before it emits `DragStart`, while the grid
analytic-plane calculation is consulted only after a drag has already been captured. Confirm the
ordering and ownership of:

- pointer activation and first-hit resolution in `gesture_system.rs`;
- `DragStart` reduction in `editor_paint_system.rs`;
- the finite analytic-plane hit in `grid_system.rs`.

Paint is now intended to be a first-class `EditorInteractionMode`, mutually exclusive with Select,
3D Cursor, and Select + Cursor. Use that mode as the policy boundary for grid drag eligibility:

- reuse the grid's existing live renderable as the BVH broad-phase candidate;
- enable its drag eligibility only for the selected grid while Paint mode owns scene input;
- use the existing finite analytic-plane helper for the exact hit and continued projection; and
- keep the grid non-selectable and the transform gizmo hidden/inert in Paint mode.

Do not add an invisible renderable or transform solely to catch the initial pointer activation. See
[Paint as a first-class editor interaction mode](../task/paint-as-first-class-editor-interaction-mode.md)
for the complete routing and mode-transition work.

## Acceptance criteria

- A fresh empty grid accepts an initial desktop click-drag and XR trigger-drag for Free Draw.
- The first painted item is placed at the grid-plane intersection, using the same grid coordinate
  and snapping rules as subsequent stroke points.
- A continuous stroke remains constrained/projected to the grid while held.
- Grid interaction does not regress into the known behavior where a grid drag arms a gizmo or
  monopolizes selection.
- The fix is active only under `EditorInteractionMode::Paint`; Paint gestures cannot retarget or
  manipulate a transform gizmo.
- A regression test covers a `DragStart` whose ray intersects the active grid's existing live
  renderable but no other raycastable scene renderable.

## Related trackers

- [Free Draw paint placement does not snap to grid while Grid Tool placement does](./free-draw-paint-does-not-snap-to-grid-while-grid-tool-placement-does.md)
- [Grid tool can leave a grid as the only selectable target, and dragging the grid rotates the gizmo](./grid-tool-leaves-grid-as-only-selectable-target-and-grid-drags-rotate-gizmo.md)
- [Editor 3D Cursor GLTF Coverage and Grid Alignment](./editor-cursor-3d-gltf-and-grid-alignment.md)
- [Paint as a first-class editor interaction mode](../task/paint-as-first-class-editor-interaction-mode.md)

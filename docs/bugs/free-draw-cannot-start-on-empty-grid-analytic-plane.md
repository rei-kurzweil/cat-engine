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

- While the editor is in Paint mode, users can begin a Free Draw stroke anywhere inside any
  enabled, visible grid's finite bounds, even when no scene renderables exist there.
- The same grid plane should supply both the initial stroke point and later drag-projection points.
- Starting a paint stroke from a grid surface must not select, move, rotate, or otherwise treat the
  grid as a gizmo target.
- Selecting a grid explicitly from `grid_panel` is a separate command path: it selects the grid's
  owning transform and attaches the normal transform gizmo even when the editor is currently in
  Paint or 3D Cursor mode. It does not make ordinary grid-surface paint hits select the grid.
- Outside the finite grid bounds, normal no-surface behavior should remain unchanged.

## Likely investigation

The gesture pipeline appears to require a raycast hit before it emits `DragStart`, while the grid
analytic-plane calculation is consulted only after a drag has already been captured. Confirm the
ordering and ownership of:

- pointer activation and first-hit resolution in `gesture_system.rs`;
- `DragStart` reduction in `editor_paint_system.rs`;
- the finite analytic-plane hit in `grid_system.rs`.

Paint is now intended to be a first-class `EditorInteractionMode`, mutually exclusive with Select,
3D Cursor, and Select + Cursor. Use entering and exiting that mode as the policy boundary for grid
raycast/BVH participation:

- on entry to Paint mode, consult `GridSystem`, iterate every enabled, visible live grid it manages,
  and register each grid's existing live renderable as a raycast/BVH broad-phase candidate;
- while Paint mode remains active, keep that membership synchronized when grids are created,
  deleted, enabled, disabled, shown, or hidden;
- on exit from Paint mode, consult `GridSystem` again and unregister every managed grid renderable
  that was made raycastable for Paint;
- make the transition idempotent so repeated mode synchronization cannot duplicate registration or
  leave stale BVH entries;
- use the existing finite analytic-plane helper for the exact hit, grid choice, and continued
  projection after broad-phase candidate discovery; and
- keep grid paint surfaces non-selectable. A grid-surface hit initiates Paint; it is not a scene
  selection or transform-gizmo hit.

The `grid_panel` selection action is an explicit exception to mode-based scene selection routing.
It should resolve and select the grid's owning transform and attach the normal transform gizmo
without first forcing the editor into Select mode. This applies in Paint and 3D Cursor modes as
well as Select modes. Only a deliberate panel selection or a dedicated gizmo handle may target the
grid transform; clicking or dragging the grid's BVH/analytic paint surface must never do so.

Do not add an invisible renderable or transform solely to catch the initial pointer activation. See
[Paint as a first-class editor interaction mode](../task/paint-as-first-class-editor-interaction-mode.md)
for the complete routing and mode-transition work. That broader task's blanket rule that Paint
hides the gizmo needs to preserve the explicit `grid_panel` selection exception documented here.

## Acceptance criteria

- Fresh empty grids accept an initial desktop click-drag and XR trigger-drag for Free Draw while
  Paint mode is active.
- The first painted item is placed at the grid-plane intersection, using the same grid coordinate
  and snapping rules as subsequent stroke points.
- A continuous stroke remains constrained/projected to the grid while held.
- Entering Paint registers all enabled, visible managed grids for raycast/BVH broad-phase testing;
  exiting Paint removes that Paint-only participation from every managed grid.
- Creating, deleting, enabling, disabling, showing, or hiding a grid while Paint is active updates
  BVH participation without leaving a stale or duplicate entry.
- Outside Paint mode, grid renderables do not participate in raycasting merely because they are
  visible or managed by `GridSystem`.
- Grid interaction does not regress into the known behavior where a grid drag arms a gizmo or
  monopolizes selection.
- Selecting a grid row selects its owning transform and attaches the gizmo without changing an
  existing Paint or 3D Cursor interaction mode.
- A Paint gesture cannot retarget or manipulate the grid gizmo unless its initial hit is a dedicated
  gizmo handle.
- Regression tests cover a `DragStart` whose ray intersects a managed grid's existing live
  renderable but no other raycastable scene renderable, Paint entry/exit with multiple grids, and
  grid-panel selection while Paint and 3D Cursor modes are active.

## Related trackers

- [Free Draw paint placement does not snap to grid while Grid Tool placement does](./free-draw-paint-does-not-snap-to-grid-while-grid-tool-placement-does.md)
- [Grid tool can leave a grid as the only selectable target, and dragging the grid rotates the gizmo](./grid-tool-leaves-grid-as-only-selectable-target-and-grid-drags-rotate-gizmo.md)
- [Editor 3D Cursor GLTF Coverage and Grid Alignment](./editor-cursor-3d-gltf-and-grid-alignment.md)
- [Paint as a first-class editor interaction mode](../task/paint-as-first-class-editor-interaction-mode.md)

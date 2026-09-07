# Interaction priorities: empty-grid painting and grabbing

Date: 2026-09-05

Status: active meta tracker. These are the most recent high-priority interaction
items, not a declaration that every item blocks a release. Existing release
checklists retain their own scope and gate decisions.

[Back to the desktop workbench](README.md)

## Priority 1: restore empty-grid Free Draw

- [ ] [Free Draw cannot start on an empty grid's analytic plane](../bugs/free-draw-cannot-start-on-empty-grid-analytic-plane.md)

The user revalidated after the attempted grid raycast/BVH registration change:
painting still does not start without backing scene geometry. Investigate this
before the grab epic. Keep the existing bug as the canonical diagnosis and
acceptance record; do not create a duplicate bug.

A headless lifecycle probe of the supplied `paint-grids-desktop.mms` confirms
that its authored grid has no live renderable/raycast marker after Paint sync.
Disabling/re-enabling it creates the missing runtime. The registration test
only covers editor-spawned grids, which already have that runtime. The exact
analytic plane helper also remains unconnected to production hit resolution.

The user has now confirmed that toggling the grid does **not** fix empty-grid
stroke startup. Live traces show empty hit lists on failed presses and backing
square captures on successful strokes. The lifecycle finding does not close
this bug; it also informs the separate startup visibility issue below.

Next milestone: inspect the visible, toggled grid's live Paint eligibility,
raycast marker, BVH entry/bounds, and exact-hit filtering to find where the grid
candidate disappears. Validate a stroke on the exposed grid area with no
backing geometry before closing this item.

Related work: [Editor/grid/paint workbench](editor-grid-paint.md),
[Paint interaction mode](../task/paint-as-first-class-editor-interaction-mode.md),
and [editor grid/paint release gate](../task/editor-grid-paint-0.8.0-release-gate.md).

## Priority 2: grid startup visibility — closed

- [x] [Grid startup visibility and UI state](../bugs/default-grid-visibility-ui-state-out-of-sync.md)

The user clarified on 2026-09-06 that the initial grid should stay hidden,
with a matching Hidden label and one toggle to show it. The desktop example
now authors that state; generated editor defaults already do. A regression
covers initial panel state and first-show runtime opacity for both paths.
After additional fixes, the user validated visibility in
`examples/vtuber-mirror-example.mms` on 2026-09-06 and requested closure for now.
Reopen if the default-grid visibility/UI mismatch recurs.

## Priority 3: grabbing, poses, and interaction zones

- [ ] [Grabbing, poses, and release zones epic](../task/epic/grabbing-poses-and-release-zones.md)

The epic owns these three tasks and their dependencies:

1. [Hand-relative, bounds-aware grab placement](../task/grab-hand-relative-bounds-placement.md),
   including live levitation-distance adjustment with the desktop scroll wheel
   and XR stick not assigned to locomotion, down to zero anchor clearance.
   This is the next implementation slice within the epic.
   Extend `InputXRGamepad` with builder options; no new
   automatic-controller-behaviors component is needed.
2. [Grab animation and reusable pose transitions](../task/grab-animation-and-pose-transitions.md),
   including untracked humanoid arm IK reaching and shared component-transform
   pose blending. Camera-only desktop uses a hold anchor without hand animation.
3. [Interaction zones, sockets, and vehicle mounting](../task/release-zones-sockets-and-vehicle-mounting.md),
   including mouth props, release-time avatar-to-broom attachment, explicitly
   enabled proximity mounting, and car-handle/steering-wheel activation.

Next milestone after the paint investigation: define shared grab state and hand
anchors, then progress the placement and pose contracts using the existing pose
layer work. The [E2 broom first slice](../task/e2-broom-mounting-first-slice.md)
is the focused attachment test: pickup, release near the legs to mount, and
dismount. It uses desktop distance adjustment as needed and precedes full
vehicle physics; mouth sockets are not a prerequisite.

## Tracking rules

Update detailed findings in the canonical bug/task, and update this page when
priority or completion changes. Close the paint item only after end-to-end
validation. Close the epic item when its three child tasks meet their acceptance
criteria. Assign release-blocker status explicitly in the relevant release gate;
high priority alone does not imply it.

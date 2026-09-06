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

The source audit confirms Paint-mode registration is present, but the finite
analytic grid-plane helper has no production callers. Gesture start still
requires a qualifying raycast hit. The reason the existing grid box hit also
failed in the reported run remains unconfirmed.

Next milestone: trace the first empty-grid press through eligibility, BVH,
raycast, gesture delivery, and Paint startup, identify the first failing stage,
and validate a repair with an actual empty-grid stroke. Registration-only tests
are insufficient to close this issue.

Related work: [Editor/grid/paint workbench](editor-grid-paint.md),
[Paint interaction mode](../task/paint-as-first-class-editor-interaction-mode.md),
and [editor grid/paint release gate](../task/editor-grid-paint-0.8.0-release-gate.md).

## Priority 2: grabbing, poses, and release zones

- [ ] [Grabbing, poses, and release zones epic](../task/epic/grabbing-poses-and-release-zones.md)

The epic owns these three tasks and their dependencies:

1. [Hand-relative, bounds-aware grab placement](../task/grab-hand-relative-bounds-placement.md),
   including live minimum levitation-distance adjustment using the stick not
   assigned to locomotion. Extend `InputXRGamepad` with builder options; no new
   automatic-controller-behaviors component is needed.
2. [Grab animation and reusable pose transitions](../task/grab-animation-and-pose-transitions.md),
   including untracked desktop hands and shared component-transform pose blending.
3. [Release zones for sockets and vehicle mounting](../task/release-zones-sockets-and-vehicle-mounting.md),
   including mouth props and the release-time avatar-to-broom attachment handoff.

Next milestone after the paint investigation: define shared grab state and hand
anchors, then progress the placement and pose contracts using the existing pose
layer work. Ordinary socket attachment precedes the broom mounting handoff.

## Tracking rules

Update detailed findings in the canonical bug/task, and update this page when
priority or completion changes. Close the paint item only after end-to-end
validation. Close the epic item when its three child tasks meet their acceptance
criteria. Assign release-blocker status explicitly in the relevant release gate;
high priority alone does not imply it.

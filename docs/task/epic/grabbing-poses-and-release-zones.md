# Tracker: grabbing, poses, and release zones

## Status

Planning, 2026-09-05. This records the design discussion; no runtime changes
are part of this work. Component and system names proposed below are provisional.

## Conclusions

Mittens operates on component trees. A grabbable object means a component
tree with a resolved movable transform, not a separate entity abstraction.
Grabbing temporarily attaches that tree to the nearest ancestor transform
of the `Pointer` that initiated the grab gesture.

Pointing and grabbing can bring a distant tree into reach. The desired resting
placement puts its bounding-box boundary flush with the grabbing hand, with
configurable clearance, in XR and desktop. Desktop without tracking needs an
explicit synthetic hand anchor and an authored pose that transitions into
holding. Webcam/MediaPipe integration is future work, not a prerequisite.

A `grab_animation_system` is a reasonable consumer of grab state: it selects
and transitions the appropriate pose. General pose interpolation belongs in
shared pose infrastructure, and attachment/placement belongs in grab handling.
Pose evaluation should update resolved component transforms; skinning is a
consumer of those transforms, not a requirement for interpolation.

Release zones share spatial eligibility and release arbitration, but their
attachment outcomes differ. A mouth socket attaches the released prop to the
avatar. A broom mount attaches the avatar to the released broom. Entering a
zone while holding only establishes eligibility; releasing commits the action.

Automatic XR controller behaviors should share one configurable component
surface covering locomotion and interaction. `InputXRGamepad` already provides
automatic locomotion, enabled by default, with builder configuration. Extend or
consolidate that surface instead of adding a component for every behavior.
While grabbing, the stick not assigned to locomotion should adjust effective
`min_grab_distance` continuously using its vertical axis (normally right stick;
up farther, down closer). Idle input must not change distance. The placement
ticket records mapping, live destination updates, and remaining default/lifetime
choices; this is proposed behavior, not an existing capability.

## Tickets

- [ ] [Hand-relative, bounds-aware grab placement](../grab-hand-relative-bounds-placement.md)
- [ ] [Grab poses and reusable pose transitions](../grab-animation-and-pose-transitions.md)
- [ ] [Release zones for sockets and vehicle mounting](../release-zones-sockets-and-vehicle-mounting.md)

## Existing foundations and delivery order

The current `GrabbableSystem` already reparents to the pointer's nearest
ancestor transform, levitates toward a destination derived from subtree
bounds, and restores the original parent on release while preserving world
pose. `Pointer.min_grab_distance` already configures clearance; its defaults
are 0.05 m with a controller driver and 0.75 m otherwise. These are distances
from the pointer ray origin, not a complete hand-contact contract.

Start by agreeing on grab state, hand-anchor resolution, and release ownership.
Placement and pose work can then use the same active-grab state. Reuse the
[existing pose-layer ticket](../avatar-pose-transition-layers.md) rather than
creating another interpolation runtime. Implement ordinary socket release
before the broom's reversed attachment and physics handoff.

## Integrated acceptance

1. In XR, point at and grab a rigid component tree: it approaches the hand and
   settles at the configured boundary clearance.
2. On desktop without tracking, the chosen hand smoothly assumes a holding
   pose and the tree settles against its synthetic hand anchor.
3. Release a lollipop or cigarette near the mouth zone: it snaps to an authored
   mouth transform and follows the avatar; elsewhere it releases normally.
4. Hold the broom near the legs, outside the torso exclusion, then release:
   the broom stops being attached to the hand, the avatar snaps to its riding
   anchor, and broom physics becomes authoritative for vehicle motion.

## Remaining design choices

Exact public APIs, desktop hand selection and anchor placement, contact-point
selection, zone geometry and tie-breaking, and vehicle locomotion/dismount
policy remain to be specified in the linked tickets. The broom asset is not
yet in the repository. Accurate deformed humanoid bounds are follow-on work;
rigid-tree placement should not wait for them.

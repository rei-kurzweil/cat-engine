# Task: grab animation and reusable pose transitions

## Status and outcome

Planned. Part of [grabbing, poses, and release zones](epic/grabbing-poses-and-release-zones.md).

An avatar without live hand tracking smoothly enters an authored reaching or
holding pose when its pointer grabs a component tree, and blends out on
release. Desktop works before webcam/MediaPipe tracking exists.

## System boundary

A proposed `grab_animation_system` observes successful grab state and controls
pose selection/weights for the corresponding avatar and hand. It does not own
pointer gesture recognition, tree attachment, contact bounds, or physics.
Failed grab attempts must not leave a holding pose active.

Reuse [transitionable avatar pose layers](avatar-pose-transition-layers.md)
and the [pose-application transition draft](../draft/pose-application-transitions.md).
Existing one-shot `apply_blended` blends from rest; repeated calls alone do not
establish a persistent layer with interruption and ownership semantics.

The existing proposal binds poses to GLTF instances. Reconcile that binding
with a reusable core that resolves pose targets to component transforms:
translation/scale interpolate and rotations use quaternion interpolation.
It should also support a posed tree without a skinned mesh. GLTF joint mapping
can remain an adapter. Do not require a separate transition on every joint;
one pose-layer transition can drive all resolved transforms.

## Required behavior

- Map active grab state to avatar, left/right hand, and held component tree.
  Define desktop hand selection and its synthetic anchor explicitly.
- Transition from the current effective pose/weight, including interrupted
  transitions and rapid grab/release/regrab. Duration and easing are authorable.
- Permit sparse hand/arm poses. A minimum holding pose is sufficient initially;
  a separate reach phase and object-specific grip poses are extensions.
- Establish explicit precedence with base animation, IK, and live tracking.
  Tracked transforms remain authoritative; animate only unowned channels.
  Controller-driven wrists and tracked fingers may have different ownership.
- Release, cancellation, target removal, pointer removal, and tracking changes
  remove or fade the contribution and reveal the current underlying pose.
- Coordinate the synthetic hand anchor with placement so the animated hand and
  held object agree. Do not derive two mutually dependent targets.
- Publish a release result that permits socket/mount poses to replace holding
  without an unintended intermediate idle frame.

## Acceptance criteria

1. Untracked desktop grabbing blends the selected hand/arm into holding and
   back out; the object meets the same hand anchor used by placement.
2. Interrupting an in-flight blend remains continuous and reaches its new target.
3. Two hands and two avatar instances maintain independent pose state.
4. XR tracking stays authoritative for its owned transforms; losing/regaining
   tracking switches ownership without stale pose residue.
5. A non-skinned component tree can use the same transform-pose interpolation.
6. Removal and failed/cancelled grabs leave no stuck holding pose.

## Open decisions

Finalize whether the consumer is named `grab_animation_system`, the pose
binding API for ordinary component trees, and the initial desktop arm strategy
(authored arm pose, IK target, or a combination). MediaPipe transport, finger
contact solving, and full-body IK are outside this ticket.

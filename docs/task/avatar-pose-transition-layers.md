# Task: transitionable avatar pose layers

## Status

Planned.  The generic `TransitionComponent` / `TransitionSystem` groundwork
exists, but avatar poses cannot yet be bound as persistent, independently
weighted layers.

## Outcome

An authored pose can be bound to one avatar instance and smoothly blend from
its current weight to another weight.  This is the foundation for posture
changes such as standing → crouching → standing and later for locomotion
animation.  It must coexist predictably with AVC's tracked head, eyes, hands,
and other live bone drivers.

## Existing capability and gap

- `TransitionComponent` is authorable beneath a target component.
- The current transition runtime samples whole-TRS transforms and emissive
  intensity, with easing and replacement behavior.
- `PoseCapturePose.apply`, `overlay`, and `apply_blended` are supported, but
  each is a one-shot pose application.  `apply_blended(target, amount)` blends
  from imported rest pose; it does not retain an application instance or
  transition the amount over time.
- The existing Bisket example loops captured running poses by discrete
  keyframe overlays.  It is useful asset proof, not an avatar locomotion
  system.

## Design

Introduce a GLTF-instance-specific pose application/layer with a scalar
weight and directly nested transition policy.  The desired public shape is:

```mms
let crouch = crouch_pose.bind(avatar_gltf) {
    Transition.duration_beats(0.20).ease_in_out_sine() {}
}

crouch.set_blend_amount(1.0)
```

The final surface may use a differently named factory, but it must preserve
these semantics:

- the same pose asset may be bound independently to multiple GLTF instances;
- an instance owns its current weight and transition policy;
- changing its weight captures from its current effective value and replaces
  its in-flight transition deterministically;
- each sampled weight reapplies the pose to that instance's targets;
- a sparse pose has explicit replace/overlay/rest-blend semantics rather than
  silently mixing them; and
- removing an instance releases its contribution and restores the applicable
  lower-priority/base pose.

Do not install a `TransitionComponent` under every bone.  The pose layer owns
one scalar transition; pose evaluation fans that result out to its resolved
joints.

## Driver ownership

Define and test ordering/ownership before enabling live VR posture use:

- tracked head and eye-bone drivers win for the bones they own;
- controller/hand retargeting wins for its mapped arm/hand bones;
- a crouch or locomotion pose controls the remaining intended body bones;
- releasing a tracking source or pose layer restores the correct lower layer,
  never an arbitrary stale transform.

The initial vertical slice may exclude head, eye, and tracked arm joints from
the authored posture poses.  That is preferable to an implicit and unstable
blend with tracking.

## Implementation and validation

1. Reconcile the stale transition checklist with the existing transform and
   emissive runtime, then add the scalar pose-layer channel it lacks.
2. Resolve pose joints atomically at bind time and retain the target mapping
   per application instance.
3. Implement weight changes, easing, replacement, removal, and exact final
   sample behavior.
4. Add a Bisket scene/test that binds a captured pose, transitions 0 → 1 → 0,
   interrupts it midway, and confirms no snapping or residual transforms.
5. Verify the same pose asset can drive two Bisket instances independently.
6. Verify an active AVC head/eye/hand driver remains authoritative on its
   mapped bones.

## Related work

- [Transitionable pose applications](../draft/pose-application-transitions.md)
- [TransitionSystem checklist](../spec/transition-system-checklist.md)
- [Pose capture and applying poses](../how_to/pose_capture_and_applying_poses.md)
- [VR avatar embodiment epic](epic/vr-avatar-embodiment.md)


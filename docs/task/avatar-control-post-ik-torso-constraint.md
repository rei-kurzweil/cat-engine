# Task: Avatar-control post-IK torso constraint

## Status

Planned.  This is a new torso-orientation constraint for `AvatarControl`, not
a replacement for tracked-hand IK or the existing body-yaw follow policy.

## Observed need

With hand tracking, a user can move an arm far enough around the torso that
the resolved upper-arm pose is anatomically implausible unless the torso also
turns.  The current body-yaw follow responds only to head yaw, so it cannot
assist in this case.

## Outcome

After hand tracking and arm IK resolve, AVC may turn the torso a limited amount
when the resolved upper-arm pose indicates that doing so would restore a valid
shoulder/torso relationship.  The turn must remain compatible with the
head-facing constraint and compose with ordinary body-yaw follow.

## Ordering and ownership

1. Apply tracked hand targets and solve arm IK.
2. Evaluate this torso constraint from the resolved upper-arm poses.
3. Resolve its requested torso yaw with at least the authority of body-yaw
   follow, subject to the head-facing and anatomical limits below.
4. Recompute the affected arm-chain world transforms and IK result after the
   torso adjustment.

The constraint must run after IK; using the pre-IK controller target alone
would infer torso pressure from a pose that may not be the pose the avatar
actually reaches.

## Constraint contract

- Use both resolved upper-arm orientations/shoulder directions, not just hand
  positions, as the input signal.
- Apply only the minimum torso yaw that brings an offending arm back toward its
  configured valid region.
- Never turn the torso if that would put the head-facing direction outside a
  configured compatible range.
- Resolve simultaneous left/right requests together; opposing requests must
  cancel or be bounded rather than oscillating frame-to-frame.
- Add thresholds, hysteresis, rate limits, and smoothing so natural hand
  jitter does not cause torso vibration.
- Preserve head, eye, and hand tracking as authoritative inputs.  The torso
  moves to support the arm pose; it must not rewrite tracker samples.
- Keep this policy optional until it is validated on representative rigs.

## Relationship to body-yaw follow

The existing body-yaw follow is a head-driven baseline.  This constraint is an
additional post-IK request.  The resolver needs an explicit precedence or
weighted merge rather than two systems independently writing the same torso
rotation.  In particular, arm-driven turning should be allowed to match or
exceed body-yaw-follow authority only while the upper-arm validity constraint
is active.

## Non-goals

- full-body IK, foot placement, or locomotion steering;
- rotating the torso merely because a hand is far away;
- replacing authored upper-body animation; or
- changing the current eye-tracking freeze experiment.  The latter remains
  available for separate headset investigation.

## Acceptance scenarios

1. Moving one tracked hand behind the torso causes a smooth, bounded torso
   assist only after the arm pose crosses its configured threshold.
2. Returning the hand to a normal range smoothly releases the assist without a
   snap or residual torso drift.
3. Opposing left/right arm requests remain stable and do not oscillate.
4. Looking in a direction incompatible with the requested torso turn prevents
   or bounds the assist according to the head-facing limit.
5. Existing head-driven body yaw still works when neither arm requires assist.
6. Tracker loss, missing humanoid arm mappings, and IK failure leave the
   current torso behavior unchanged.

## Related code

- `src/engine/ecs/system/avatar_control_system.rs`
- `src/engine/ecs/component/avatar_body_yaw.rs`
- `src/engine/ecs/system/avatar_body_yaw_system.rs`
- `src/engine/ecs/system/ik_system.rs`

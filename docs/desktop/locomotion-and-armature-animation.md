# Locomotion and whole-armature animation

Date: 2026-09-02

Status: planned. Generic transform transitions exist, but transitionable,
weighted pose layers are the main missing prerequisite for locomotion animation.

[Back to the desktop workbench](README.md)

## Outcome

Animate and transition a complete multi-bone armature as one coordinated pose
operation, then use that capability for smooth idle/walk/run locomotion without
overwriting bones owned by live head, eye, hand, or controller tracking.

## Canonical trackers

- [Transitionable avatar pose layers](../task/avatar-pose-transition-layers.md)
- [Bisket walking locomotion animation](../task/bisket-walking-locomotion-animation.md)
- [Transitionable pose applications](../draft/pose-application-transitions.md)
- [Transition system checklist](../spec/transition-system-checklist.md)
- [Pose capture and applying poses](../how_to/pose_capture_and_applying_poses.md)
- [Animation keyframe interpolation design record](../spec/animation-keyframe-interpolation.md)

The transition checklist and interpolation spec contain stale Action-era parts;
use the pose-layer tracker as the current product direction and reconcile those
documents before implementing from their unchecked lists.

## Work order

### 1. Whole-armature transition proof

- [ ] Bind one captured pose to one GLTF armature instance with a scalar weight.
- [ ] Resolve and retain the target joint set once per binding rather than
      installing one transition component per bone.
- [ ] Tween 0 → 1 → 0, interrupt midway, and replace the destination without a
      snap or stale final transform.
- [ ] Prove two armature instances can use the same pose asset independently.
- [ ] Remove the pose layer and restore the correct lower/base pose.

### 2. Bone ownership and composition

- [ ] Define masks/priorities for base pose, posture, locomotion, secondary
      motion, and live tracked drivers.
- [ ] Ensure tracked head/eyes/hands remain authoritative on their mapped bones.
- [ ] Define how sparse poses and rest-pose blending compose.
- [ ] Make diagnostics show which layer/driver currently owns a joint.

### 3. Locomotion policy

- [ ] Drive gait from the chosen locomotion/root displacement, not raw HMD
      wobble or head rotation.
- [ ] Define idle/walk/run thresholds, hysteresis, stride phase, and in-place
      versus root-motion policy.
- [ ] Audit Bisket's existing relaxed/running captures and capture missing walk,
      idle, contact, or transition poses.
- [ ] Compose gait with the documented crouch/posture policy.
- [ ] Add desktop and VR scenes with visible speed, state, phase, active layer,
      and bone-owner diagnostics.

## Acceptance path

1. A button or timer transitions an entire test armature between two obviously
   different poses and can reverse halfway smoothly.
2. The same test runs on two avatar instances without cross-talk.
3. A tracked/mock-tracked head and hands remain stable while the body pose
   transitions.
4. Bisket starts, changes speed, stops, and crouch-walks without snapping,
   frozen partial strides, threshold chatter, or residual bone transforms.

## Input connection

Locomotion animation consumes semantic movement chosen by locomotion policy; it
must not depend directly on a particular keyboard, desktop gamepad, or XR
controller backend. See [Keyboard and regular gamepad events](keyboard-and-gamepad-input.md)
for the input-provider layer.

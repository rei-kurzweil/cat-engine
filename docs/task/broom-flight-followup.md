# Task: broom flight follow-up and vehicle example

Status: planned, 2026-09-07. Documentation only; no example or engine changes yet.
Follows [E2 broom attachment](e2-broom-mounting-first-slice.md).

## Scope and dependencies

Build a small script-controlled flight example on the mounting handoff, with
two enabling tasks:

1. [Scriptable velocity pose driver](scriptable-velocity-pose-driver.md):
   input-independent descendant motion, reference-based MMS updates, and
   orientation-to-direction helpers.
2. [MMS keyboard and regular gamepad events](mms-keyboard-and-gamepad-events.md):
   event sources usable independently of automatic locomotion.

There is no third task to change `Input` / `I {}` automatic keyboard behavior.
The new example will omit that component and own its controls in MMS.

## Vehicle example

Create `examples/broom-flight.mms` during implementation. Copy the house/room,
lights, Bisket avatar, broom, and pre-roll from [E2](../../examples/e2.mms).
Omit the giant estradiol tablet and its display plinth. Retain the mirror for
mount alignment inspection. Preserve E2 as the attachment acceptance scene.

Copy the avatar's visual setup and first-person camera attachment, but replace
its `I {}`-based driving topology with explicit scripted movement/pose ownership.
Define how the avatar remains positioned and how camera look works without
`I {}`; simply deleting the input wrapper is not a complete replacement.
Provide enough unmounted positioning/look control to reach and mount the broom.

While mounted, MMS turns keyboard/gamepad state into a direction and speed,
then updates the broom's referenced `Velocity` component. Resolve a deliberate
steering orientation (broom or camera, documented in the example) and its
forward axis. Recompute direction while steering changes even if no new key
event arrives. The broom drives motion; its mounted rider and camera follow.
The pre-roll remains an ordinary grabbable prop to check interaction coexistence.

First flight policy: kinematic directional motion, no gravity or inertia, and
zero commanded velocity when movement controls are released. Document bindings
for steering, forward/backward, ascent/descent, stop, and dismount. Dismount,
focus loss, and active-controller disconnect stop flight and clear held input.
Disable vehicle driving while the broom is held or unmounted. Do not let an
avatar driver also integrate the mounted rider's translation.

## Acceptance and later physics

- Load the new MMS example, mount, steer, fly, stop, and dismount repeatedly
  using keyboard and, separately, a regular non-XR gamepad.
- Check predictable distance over time, orientation-based steering, rider
  following, control restoration, and no continued motion after focus loss.
- Existing E2 pickup/mount tests and the new example's pre-roll pickup still work.

This establishes flight control and integration, not full rigid-body physics.
Gravity/lift, acceleration, drag, momentum/velocity inheritance, and collision
response are later policies. The initial example must state that collisions
do not yet constrain flight if that remains the implementation. Later collision
integration must share motion authority rather than apply displacement twice.

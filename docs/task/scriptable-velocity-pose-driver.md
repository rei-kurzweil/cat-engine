# Task: scriptable Velocity pose driver

Status: planned, 2026-09-07. Supports [broom flight](broom-flight-followup.md).

## Contract

Introduce `Velocity` as a pose driver that does not require user input. It
integrates a configured linear velocity in world units per second using frame
elapsed time and applies the resulting motion to descendant transforms through
the existing pose/transform pipeline. Define the driven transform boundary using
the established pose-driver conventions; descendants inherit motion once, not
once per ancestor. Preserve authored offsets, rotation, and scale.

Expose construction and live get/set through an MMS component reference.
Illustrative syntax, not a committed or implemented API:

```text
let flight = Velocity { velocity(0, 0, 0) }
// Place the vehicle transform beneath this driver in the authored tree.
flight.set_velocity([0, 0, 2])
flight.set_velocity([0, 0, 0])
```

Updates change the live component, not a copied value or just its construction
configuration. Specify update ordering and whether a handler's update applies
this tick or next. Default to zero velocity; provide explicit enable/disable
semantics, finite-value validation, and cleanup when the driver is removed.
Serialize authored initial configuration separately from transient scripted state.

Start with explicitly world-space velocity. Correctly convert displacement
through the effective parent basis; parent rotation/scale must not silently
change commanded world speed. Document handling of singular parent transforms.
If local-space velocity is added, expose its space rather than infer it.

## Orientation helpers

MMS needs a reusable way to rotate a chosen local axis by a quaternion and
obtain a direction vector. This is not truncating a vec4 to a vec3:

```text
direction = rotate_vector(orientation_xyzw, local_forward_axis)
velocity = direction * speed
```

Names above are provisional. Normalize valid quaternions, reject invalid/zero
quaternions, and document `xyzw` ordering and the selected forward-axis convention.
Allow reading an object's local or world orientation through existing transform
accessors. A transform/matrix direction convenience should ignore translation
and yield a unit direction without scale changing speed; specify behavior for
shear, mirrored, and degenerate bases. Prefer reusing
[transform accessors](../draft/transform-component-accessors-engine-api.md).

Provide an easy direction-and-speed setter or a helper composition from MMS;
avoid requiring authors to implement quaternion math themselves. A setter using
an orientation snapshots it unless explicitly documented as a live binding.
Continuous steering must refresh the velocity when orientation changes.

## Motion authority and existing draft

The older [velocity-components WIP](wip/velocity-components.md) proposes storage,
history, and derived motion observations, explicitly not integration. This task
defines the requested active driver. Reconcile naming/shared storage during
implementation without making history, angular velocity, or collision-system
migration prerequisites. Observed velocity and commanded velocity must not be
confused or fed back into two integrators.

Exactly one owner integrates a driven transform. For the broom, enable the
driver only after the mount handoff has removed the pointer attachment and
established independent vehicle motion. Dismount disables/zeros it according
to the flight example's policy.

## Acceptance

- An input-free MMS scene moves a descendant at the configured speed.
- Changing the component by reference changes motion; zero stops it.
- Nested descendants inherit displacement once, including rotated/scaled parents.
- Equal elapsed time at different frame rates yields equivalent displacement.
- Identity and quarter-turn orientations produce the documented directions;
  invalid values fail clearly, and scale does not change commanded speed.
- Disable/removal and mount/dismount leave no stale motion or competing driver.

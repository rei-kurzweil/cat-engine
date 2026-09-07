# Task: MMS keyboard and regular gamepad events

Status: planned, 2026-09-07. Supports [broom flight](broom-flight-followup.md).

Expose keyboard and non-XR gamepad input to MMS event handlers, independently
of `Input` / `I {}` automatic movement. Use the existing XR controller event
surface as the model for component references, subscription, and payloads.
Review `InputXRGamepadSystem` and its button/axis events before adding a parallel
path; reuse event infrastructure where possible.

## Public surface

`Keyboard {}` and `Gamepad {}` are proposed event-source components, not pose
drivers. An existing scene/global subscription mechanism may replace the need
for `Keyboard` if it provides equivalent scoping, focus, and cleanup. Settle
that after inspecting MMS's event facilities; do not invent a global singleton
requirement or make `I {}` a prerequisite.

- Keyboard: key down/up, stable key identity, modifier state, repeat information,
  and documented physical-key versus text/layout semantics. Text entry remains
  distinct from gameplay controls.
- Regular gamepad: button down/up, analog sticks/triggers with documented ranges
  and deadzones, device identity/selection, connection and disconnection events.
- Handlers can maintain held state and update other components by reference,
  including `Velocity`; input components themselves do not move transforms.

Reuse MMS `on(component, event, handler)` if components are chosen. Exact event
names and payloads remain open. Multiple listeners must have defined routing;
do not accidentally broadcast one selected controller into unrelated vehicles.

## Lifecycle and ownership

Define interaction with focused editor controls and text entry so typing does
not also fly the broom. Preserve releases or publish a state reset when focus
or capture changes; never leave a held movement key latched. Clear state on
window focus loss, component disable/removal, controller disconnection, and
scene teardown. Repeats must not become additional independent key presses.

Continuous movement is held state integrated over time, not OS key-repeat
frequency. Define simultaneous opposing keys and keyboard/gamepad arbitration.
No changes to `I {}` automatic keyboard behavior are required: the flight
example deliberately omits it and scripts its own controls.

## Acceptance

- MMS receives keyboard edges and regular gamepad button/axis changes without
  any `I {}` or XR session, and uses them to update a referenced component.
- Holding/releasing controls produces stable start/stop behavior; repeated key
  events do not multiply speed, and analog magnitude controls speed predictably.
- Text focus, window blur, disconnection, and scene reload leave no stuck input
  or stale handlers; selecting one gamepad does not consume another's identity.
- Existing XR event subscriptions continue to work.

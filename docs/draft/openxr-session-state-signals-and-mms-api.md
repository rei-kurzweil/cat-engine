# OpenXR session-state signals and MMS runtime API

Status: draft; not implemented.

## Purpose

Expose OpenXR lifecycle and active interaction-profile state to ordinary engine event consumers
and MMS.

The engine already receives OpenXR session-state and interaction-profile events, but currently
uses them only to update private renderer state or print debug logs. Authored scenes cannot tell
when an XR session becomes visible, loses focus, stops, or changes controller profile. They also
cannot inspect the runtime and OpenXR system information already available during initialization.

This draft adds:

- one exact `XrSessionStateChanged` event instead of derived “started presenting” and “stopped
  presenting” events;
- one `XrInteractionProfileChanged` event containing the active profile paths chosen by the
  runtime for the left and right hands;
- read-only MMS methods on the live `XR` component for the current session state, runtime/system
  information, and current interaction profiles.

## Current behavior

`OpenXRSystem::pump_events` receives `openxr::Event::SessionStateChanged`, stores the new
`openxr::SessionState`, begins the session on `READY`, ends it on `STOPPING`, and logs the
transition. It does not emit an `EventSignal`.

The controller path periodically calls `current_interaction_profile` for:

- `/user/hand/left`
- `/user/hand/right`

It converts the selected OpenXR paths to strings such as
`/interaction_profiles/valve/index_controller`, but only prints changes for debugging.
`openxr::Event::InteractionProfileChanged` is not currently handled.

MMS exposes `XrButtonDown`, `XrButtonUp`, `XrButtonChanged`, and `XrAxisChanged`. It has no
session-lifecycle signal or read-only XR metadata API. `XR` itself contains only the authored
`enabled` flag.

## Session-state event

Add an engine-facing state enum that mirrors the complete core OpenXR session-state set:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrSessionState {
    Unknown,
    Idle,
    Ready,
    Synchronized,
    Visible,
    Focused,
    Stopping,
    LossPending,
    Exiting,
}
```

Convert from `openxr::SessionState` at the OpenXR boundary. Keep the engine enum independent of
the OpenXR crate so the ECS signal contract does not expose a backend-library type. The mapping
must be exhaustive for every state supported by the linked OpenXR crate; an unknown future raw
value maps to `Unknown` rather than being mistaken for `Idle`.

Add:

```rust
EventSignal::XrSessionStateChanged {
    xr: ComponentId,
    previous: XrSessionState,
    state: XrSessionState,
}

SignalKind::XrSessionStateChanged
```

This event reports the actual session transition. Do not add separate
`XrStartedPresenting`/`XrStoppedPresenting` events in v1. Those names hide useful distinctions:

- `READY` means the application should begin the session.
- `SYNCHRONIZED` means frame synchronization is active but application content is not necessarily
  visible.
- `VISIBLE` means submitted content may be visible.
- `FOCUSED` means visible with input focus.
- `STOPPING` requires ending the session.
- `LOSS_PENDING` and `EXITING` are materially different shutdown reasons.

Authors can derive the policy they need from the exact state. For example, an application may
treat both `VISIBLE` and `FOCUSED` as presenting, while an input-heavy experience may require
`FOCUSED`.

### MMS payload

The state fields use the canonical uppercase OpenXR names:

```mms
let xr = XR.on()

on(xr, "XrSessionStateChanged", fn(event) {
    print("XR state: " + event.previous + " -> " + event.state)

    if event.state == "VISIBLE" || event.state == "FOCUSED" {
        print("XR content may now be presented")
    } else if event.state == "STOPPING" ||
              event.state == "LOSS_PENDING" ||
              event.state == "EXITING" {
        print("XR presentation is stopping")
    }
})
```

The MMS event map is:

```text
{
  xr:       <live XR component>,
  previous: "IDLE" | "READY" | "SYNCHRONIZED" | "VISIBLE" | "FOCUSED" |
            "STOPPING" | "LOSS_PENDING" | "EXITING" | "UNKNOWN",
  state:    same value set,
}
```

Use uppercase names because they correspond directly to the OpenXR constants and are unambiguous
in logs and authored comparisons. Do not emit Rust `Debug` formatting without a test that pins the
MMS spelling.

## Interaction-profile event

Handle `openxr::Event::InteractionProfileChanged` by querying both hand user paths and comparing
the resulting profile snapshot with the last published snapshot.

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XrInteractionProfiles {
    pub left: Option<String>,
    pub right: Option<String>,
}

EventSignal::XrInteractionProfileChanged {
    xr: ComponentId,
    previous: XrInteractionProfiles,
    profiles: XrInteractionProfiles,
}

SignalKind::XrInteractionProfileChanged
```

The strings are the actual OpenXR paths returned by `current_interaction_profile`, for example:

```text
/interaction_profiles/khr/simple_controller
/interaction_profiles/oculus/touch_controller
/interaction_profiles/htc/vive_controller
/interaction_profiles/valve/index_controller
/interaction_profiles/microsoft/motion_controller
/interaction_profiles/ext/hand_interaction_ext
```

`None` means that the runtime reports `Path::NULL`, the profile query failed, or no active profile
has been selected for that hand. Do not infer a controller or headset model from a profile path.
OpenXR allows mixed devices and different profiles per hand.

The existing periodic polling may remain as a defensive fallback for runtimes that fail to emit
the profile-changed event, but it should call the same compare-and-publish helper. A profile
snapshot is emitted only when its value changes.

### MMS payload

```mms
on(xr, "XrInteractionProfileChanged", fn(event) {
    print("left profile: " + event.left)
    print("right profile: " + event.right)
})
```

The MMS event map is deliberately flat:

```text
{
  xr:             <live XR component>,
  previous_left:  string | null,
  previous_right: string | null,
  left:           string | null,
  right:          string | null,
}
```

Flat fields match the existing MMS event-map convention and avoid requiring authored record
literals. Rust may retain the grouped `XrInteractionProfiles` value internally.

## Event scope and delivery

Both events are scoped to the live authored `XR` component:

```mms
let xr = XR.on()
on(xr, "XrSessionStateChanged", fn(event) { ... })
on(xr, "XrInteractionProfileChanged", fn(event) { ... })
```

`OpenXRSystem` is singleton state, but more than one enabled `XR` component may exist in authored
topology. Track registered XR component IDs and publish the same transition to each currently
enabled live component. `XrComponent` currently has no cleanup intent, so implementation must add
`RemoveXr` and remove registered IDs when components are cleaned up. Do not choose an arbitrary
“first” XR component as the event owner.

Global handlers may observe the new `SignalKind`s through the existing global subscription
mechanism, but scoped handlers are the primary authored API.

Change `pump_events` to receive a `SignalEmitter`; `tick_with_queue` already has one. State
mutation, `session.begin`, or `session.end` happens first, then the fact describing the accepted
transition is emitted. If `session.begin` or `session.end` fails, retain the exact runtime state
in the event and expose the operational failure through the existing error path; do not fabricate
a different session state.

## Read-only MMS API

Add these methods to a live `XR` component:

```mms
xr.session_state()
xr.runtime_info()
xr.interaction_profiles()
```

### `xr.session_state()`

Returns the current canonical state string. It returns `"UNKNOWN"` before a session exists or
after runtime state is unavailable.

### `xr.interaction_profiles()`

Returns:

```text
{
  left:  string | null,
  right: string | null,
}
```

This is the current chosen profile snapshot. It is not a list of every profile for which the
engine has suggested bindings.

### `xr.runtime_info()`

Returns a read-only MMS map:

```text
{
  available:                    bool,
  runtime_name:                 string | null,
  runtime_version:              string | null,
  system_name:                  string | null,
  vendor_id:                    number | null,
  orientation_tracking:         bool | null,
  position_tracking:            bool | null,
  max_swapchain_image_width:    number | null,
  max_swapchain_image_height:   number | null,
  max_layer_count:              number | null,
  gpu_device_name:              string | null,
  view_configuration:           "PRIMARY_STEREO" | null,
  render_extent:                [width, height] | null,
  session_state:                string,
  left_interaction_profile:     string | null,
  right_interaction_profile:    string | null,
}
```

Populate:

- `runtime_name` and `runtime_version` from `Instance::properties`;
- `system_name`, `vendor_id`, tracking flags, and graphics limits from
  `Instance::system_properties`;
- `gpu_device_name` from the selected Vulkano physical device;
- render extent from the active XR swapchain;
- current state and profiles from the cached OpenXR system snapshot.

Cache this immutable or slowly changing metadata in `OpenXRSystem`. MMS queries must not call
OpenXR functions or lock the graphics device directly.

## About the “headset name”

OpenXR can provide `SystemProperties::system_name`. The engine does not currently query it, and it
should be added to `runtime_info`.

`system_name` is the best available OpenXR system label, but it is not guaranteed to be the retail
headset model. A streaming or compatibility runtime may report its own virtual-system name.
Likewise:

- `runtime_name` identifies SteamVR, Monado, ALVR, or another runtime, not necessarily the HMD;
- the Vulkan GPU name identifies the rendering adapter, not the HMD;
- interaction-profile paths identify the active controller/hand binding profile and may differ
  between hands;
- a controller profile must not be used to infer the headset model.

Expose these facts independently and let diagnostic UI show them verbatim.

## Supported versus chosen profiles

OpenXR does not provide one universal “all profiles supported by this physical headset” list.
Keep the API terminology precise:

- **suggested binding profiles** are profiles for which the engine submitted accepted binding
  suggestions during initialization;
- **active/chosen interaction profiles** are the paths currently returned by
  `current_interaction_profile` for each user hand.

The v1 MMS API exposes the active/chosen paths because those describe what the runtime is actually
using. A later diagnostics API may expose `suggested_binding_profiles()` as an array, but it must
not label that list as detected hardware support.

## Implementation outline

1. Add `XrSessionState`, `XrInteractionProfiles`, both `EventSignal` variants, and both
   `SignalKind` variants.
2. Add the two new names to the MMS signal-kind parsers used by the host and world evaluator.
3. Add event-to-MMS map conversion in the scripting runner.
4. Retain registered live `XR` component IDs in `OpenXRSystem`; add `XrComponent::cleanup`,
   `IntentValue::RemoveXr`, and its `SystemWorld` handler.
5. Pass the command queue into `pump_events` and emit state changes after internal state handling.
6. Handle `openxr::Event::InteractionProfileChanged`; share its query/compare path with periodic
   fallback polling.
7. Cache instance properties, system properties, selected GPU name, swapchain extent, current
   session state, and current profiles.
8. Add live `XR` component method dispatch for `session_state`, `runtime_info`, and
   `interaction_profiles`.
9. Update the XR input specification and add one MMS example showing lifecycle and profile
   diagnostics.

## Tests

- Every OpenXR core session state maps to the exact MMS uppercase name.
- A state transition emits once with the correct previous/current values.
- Repeating the same state does not emit a duplicate transition.
- `READY` begins the session before its event is dispatched.
- `STOPPING` ends the session before its event is dispatched.
- `LOSS_PENDING` and `EXITING` remain distinguishable.
- Profile-change handling queries both hands and emits only when the snapshot changes.
- `Path::NULL` becomes MMS `null`.
- Left and right may hold different active profiles.
- Scoped handlers receive events on every enabled live `XR` component.
- Removed and disabled XR components receive no events.
- MMS handlers can compare `event.state == "FOCUSED"` and read every profile field.
- `runtime_info` succeeds before session creation with unavailable fields represented by `null`.
- System name, runtime name, and GPU name remain separate fields.
- Runtime metadata queries use cached state and make no OpenXR or Vulkan calls from MMS execution.

## Non-goals

- Inferring a retail headset model from controller profiles, runtime name, USB IDs, or GPU name.
- Serializing runtime metadata into scene files.
- Allowing MMS to force an OpenXR session state.
- Replacing `XrButton*` or `XrAxisChanged` input events.
- Defining application-specific pause/resume policy.
- Adding synthetic presentation-started/stopped events before exact session-state usage shows they
  are necessary.

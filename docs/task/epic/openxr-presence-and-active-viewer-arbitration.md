# OpenXR presence, runtime-state signals, and active viewer arbitration

Status: active planning epic

## Purpose

Expose trustworthy OpenXR runtime and device-presence state through the engine signal model, then
use that state together with resolved pointer interaction to choose whether desktop or XR is the
authoritative viewer for camera-dependent editor affordances.

The immediate user-facing regression is transform-gizmo scale in mixed desktop/XR applications.
`bisket-vr-demo` can render a desktop `Camera3D` while SteamVR and ALVR continue presenting to a
`CameraXR`. The current rule makes XR globally authoritative whenever it has published eye views,
even when nobody is wearing or interacting through the headset.

This epic joins:

- [OpenXR session-state signals and MMS runtime API](../../draft/openxr-session-state-signals-and-mms-api.md)
- [Transform gizmo active viewer family](../transform-gizmo-active-viewer-family.md)

These should be implemented and validated together because viewer arbitration needs explicit
runtime capability/presence state, while the OpenXR event work needs a concrete engine consumer
that proves the distinction between presentation, focus, presence, and interaction.

## Core distinctions

Keep these states separate:

- **Camera available:** a usable window or XR camera has published the data needed for rendering.
- **Session running/presenting:** OpenXR is synchronized or visible and may be receiving frames.
- **Session focused:** the OpenXR application is eligible to receive focused input.
- **User present:** the runtime reports that a user is present through
  `XR_EXT_user_presence`.
- **User interacting:** a desktop or XR pointer/input source performed meaningful interaction.
- **Active viewer family:** the one camera family selected by editor policy while the transform
  architecture has one effective world matrix.

SteamVR or ALVR presenting frames proves camera availability and session activity. It does not
prove that the headset is being worn and must not, by itself, select XR as the active viewer.

## Execution order

### Phase 1: publish OpenXR runtime state

Implement the existing session-state and interaction-profile design:

1. Map all core OpenXR session states into engine-owned values.
2. Emit exact session-state transitions on every enabled live `XR` component.
3. Handle interaction-profile changes and publish left/right active profiles.
4. Cache runtime and system metadata for read-only MMS inspection.
5. Add cleanup for registered `XR` components.

Do not derive user-presence or application focus from `SessionState::FOCUSED`.

### Phase 2: add user-presence capability and events

Extend the same OpenXR state/event boundary with `XR_EXT_user_presence`:

1. Detect whether the runtime advertises `XR_EXT_user_presence`.
2. Enable it when advertised.
3. Query `XrSystemUserPresencePropertiesEXT::supportsUserPresence`.
4. Retain capability and current presence separately:

   ```text
   user_presence_supported: bool
   user_present: Option<bool>
   ```

   `None` means unsupported or not yet reported. It must not be treated as either present or
   absent.
5. Handle `XrEventDataUserPresenceChangedEXT`.
6. Emit an engine event scoped to each enabled live `XR` component.
7. Expose cached presence support/state through the read-only XR runtime API.
8. Emit/log the initial state reported after session begin as well as later changes.

Suggested engine event:

```rust
EventSignal::XrUserPresenceChanged {
    xr: ComponentId,
    previous: Option<bool>,
    present: bool,
}
```

Suggested MMS payload:

```text
{
  xr:       <live XR component>,
  previous: bool | null,
  present:  bool,
}
```

The exact name should remain aligned with OpenXR (`XrUserPresenceChanged`) rather than using an
inferred name such as `HeadsetPutOn`.

### Phase 3: choose the active gizmo viewer

Replace the implicit “XR eyes exist, therefore XR wins” rule with the policy in
[Transform gizmo active viewer family](../transform-gizmo-active-viewer-family.md):

1. Automatically select the only usable family when only desktop or only XR is available.
2. Default to desktop when both are available and no meaningful interaction/presence history
   exists.
3. Treat `user_present: false -> true` as strong evidence that XR became active.
4. Let resolved pointer selection and direct manipulation outrank passive platform focus.
5. Let desktop input return ownership to the desktop family.
6. Lock the chosen family for the duration of a gizmo drag.
7. Pass one resolved family to both depth calculation and `TransformCameraSpecific` selection.

If presence is unsupported or unreliable, XR controller/hand interaction remains the fallback.
Passive head pose updates and frame presentation are never fallback activation signals.

## Viewer arbitration policy

When both camera families are usable, apply this priority:

1. Family locked by the active gizmo drag.
2. Family of the pointer that most recently selected or manipulated the target/gizmo.
3. Most recent meaningful desktop or XR interaction.
4. A supported `false -> true` XR user-presence transition.
5. Monoscopic default.

Presence is evidence that XR became relevant, not permanent ownership. A later desktop click or
mouse-driven selection must switch ownership back to desktop even while the headset remains worn.

An XR `true -> false` transition releases presence-derived XR preference. It does not interrupt a
drag without an explicit cancellation/fallback policy.

## Focus 3 through ALVR validation gate

The first hardware validation target is a VIVE Focus 3 streamed through ALVR to SteamVR.

Validate every hop independently:

```text
VIVE Focus 3 proximity/user-presence state
  -> ALVR Android client
  -> ALVR SteamVR driver proximity state
  -> SteamVR OpenXR XR_EXT_user_presence
  -> OpenXRSystem event
  -> engine/MMS signal
  -> active gizmo viewer policy
```

Record:

- VIVE Focus 3 firmware/ROM version
- ALVR client and streamer versions
- SteamVR version and whether stable or beta
- OpenXR runtime name/version
- whether `XR_EXT_user_presence` is advertised
- whether `supportsUserPresence` is true
- the initial presence event after session begin
- events observed when covering/uncovering the proximity sensor
- events observed when putting on/removing the headset

If SteamVR advertises the extension but reports `supportsUserPresence == false`, or no changes
arrive from ALVR, retain the interaction fallback and record the failed hop. Do not synthesize
presence from head movement or session focus.

## Tests

### OpenXR state boundary

- Complete session-state mapping and exact transition payloads.
- Interaction-profile snapshots emit only on change.
- Presence extension is enabled only when advertised.
- Unsupported presence remains `None` and emits no fabricated state.
- Initial and subsequent presence events update cached state and emit once per change.
- Multiple enabled live `XR` components receive equivalent scoped events.
- Removed or disabled `XR` components receive no events.
- MMS exposes session state, interaction profiles, presence support, and current presence without
  making live OpenXR calls.

### Viewer policy

- Desktop-only and XR-only configurations require no manual focus.
- Passive XR presentation cannot override active desktop editing.
- Supported user-presence transition can activate XR when both families exist.
- XR pointer interaction works when presence is unsupported.
- Desktop interaction can reclaim ownership while XR remains present.
- A drag keeps its starting family until end/cancellation.
- Depth calculation and transform-stream family selection always agree.

### Integration

- A synthetic OpenXR presence event can drive the viewer policy without hardware.
- A live gizmo test exercises camera movement, effective transform propagation, VisualWorld model
  updates, and BVH refitting.
- Focus 3/ALVR/SteamVR manual validation covers sensor transitions and both desktop/XR editing.

## Non-goals

- Producing simultaneous monoscopic and stereoscopic model matrices for one renderable.
- Inferring headset presence from head motion.
- Treating OpenXR session focus as equivalent to headset-worn state.
- Requiring user presence support for XR interaction.
- Inferring a retail headset model from runtime or interaction-profile strings.
- Applying camera-relative transforms to ordinary renderables.

## Completion criteria

- OpenXR session, interaction-profile, and user-presence changes have engine signals and cached
  read-only state.
- Presence capability and presence value remain distinguishable.
- Focus 3 through ALVR/SteamVR is probed and the result is recorded.
- An unused but presenting XR session no longer overrides desktop gizmo scaling.
- Putting on the headset activates XR automatically when presence forwarding works.
- XR interaction activates XR when presence forwarding is unavailable.
- Single-family camera configurations require no focus action.
- Viewer ownership is stable throughout a gizmo drag.
- Rendering, BVH bounds, and raycasting observe one coherent selected gizmo scale.

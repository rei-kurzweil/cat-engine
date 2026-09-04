# AVC runtime eye-tracking selection panel

Date: 2026-09-04

Status: planned — lifecycle seam audit complete; no runtime switching UI yet.

## Outcome

In `examples/vtuber-microphone-speaking-xr-eye-tracking.mms`, add a world-space
`info_panel` matching the existing audio-input panel. It lets the user choose
the one eye tracking transport attached directly beneath the avatar's `AVC`:

- `VRChat OSC`
- `HTC Wave Sdk\n(experimental)`

MediaPipe is a planned third option, not part of the first implementation.
Selecting an option must remove the currently active eye-tracking component,
attach and initialize the selected replacement, and make AVC consume only that
replacement. This is a transport selector, not a policy to blend all available
trackers.

Until the selector lands, the transport-neutral base scene starts with VRChat
OSC. Its HTC companion remains a static transport-specific acceptance scene:

- `examples/vtuber-microphone-speaking-xr-eye-tracking.mms` uses
  `XREyeTracking.on()` (VRChat OSC; default UDP port `9000`).
- `examples/vtuber-microphone-speaking-xr-eye-tracking-htc.mms` uses
  `XREyeTrackingHTC.on()` (HTC Wave SDK / ALVR binary UDP; default port
  `9002`).

## What works today

MMS already has the primitives needed to express the structural part of a
swap:

1. A `let`-bound component expression materializes as a detached live
   component object.
2. `avc.attach(tracker)` emits `IntentValue::Attach` and runs the deferred
   initialization walk when the mutation executes.
3. `tracker.remove_subtree()` emits `IntentValue::RemoveSubtree`.
4. `SystemWorld::remove_subtree_immediate` deletes the subtree and removes
   scoped signal handlers rooted within it.

The desired authored shape is therefore feasible without re-creating the
avatar or `AVC`:

```mms
let active_tracker = XREyeTracking.on()
avc.attach(active_tracker)

// on a selector-row click, after the runtime has a way to retain/reassign
// live component handles:
active_tracker.remove_subtree()
let replacement = XREyeTrackingHTC.on()
avc.attach(replacement)
active_tracker = replacement
```

The exact reassignment/selection-state syntax should follow the retained MMS
component-handle work; this snippet states the required topology and ordering,
not a claimed complete program.

## AVC seam

`AvatarControlSystem` already treats `XREyeTrackingComponent` and
`XREyeTrackingHtcComponent` as the same normalized input category. Each frame
it scans only the **direct children** of the AVC and selects the newest valid
gaze and closure sample independently for each eye. Its global receive
sequence is shared by both transports.

This is the good seam: attach the replacement as a direct AVC child and remove
the old direct child. No new AVC field, tracker enum, or source-specific pose
path is required for the first two choices.

On the next avatar-control update, absence of valid gaze releases the old
eye-bone ownership to the GLTF rest rotation; absent closure removes the morph
driver and restores its base value. Because a new tracker has no samples when
it is created, the avatar can briefly return to rest between source handoff
and the first replacement packet. That is correct and safer than applying a
stale source's expression indefinitely.

Do not leave both trackers as AVC children and merely hide or disable one.
Today AVC intentionally arbitrates every direct child by newest sample, so a
background source can take control again. A true selector must retain exactly
one active direct tracker child.

## Transport-system lifecycle seam

`XREyeTrackingSystem` owns UDP sockets and cached decode state keyed by
component ID:

- standard OSC: `sockets`, `standard_samples`, and failed-bind IDs;
- HTC: `htc_sockets` and failed-bind IDs.

It prunes those maps at the start of its next tick by scanning the current
world, so removal does eventually close/drop the old socket state. It does not
currently expose a per-component `remove`/`component_removed` hook, and
`SystemWorld::remove_subtree_immediate` does not notify it. A switch can
therefore leave the old socket registered until the next eye-tracking tick.

Before exposing the selector, add an explicit
`XREyeTrackingSystem::component_removed(ComponentId)` (or a shared component
lifecycle hook) and call it during authoritative subtree removal. It must
remove the ID from both socket maps, sample cache, and failed-bind sets so the
socket closes immediately. This is also a concrete consumer of the broader
[unified subtree-removal lifecycle](unify-subtree-removal-and-component-system-cleanup.md)
task.

This hook makes fast A → B → A selection deterministic and prevents a removed
component's UDP binding from surviving long enough to conflict with a fresh
component configured for the same port.

## Menu design

Place a second ordinary-scene `info_panel` beside the existing microphone
panel in the base scene. Reuse its title chrome, drag behavior, accordion
minimize/restore behavior, raycast rows, and styling; do not build a parallel
editor panel.

Suggested initial content:

```text
eye tracking

[ VRChat OSC ]
[ HTC Wave Sdk
  (experimental) ]
```

Each option row should set a small status text such as `selected VRChat OSC`.
The active row needs a selected visual treatment. The two labels are stable UI
names; they must map to component factories, not directly to socket/port
details. MediaPipe later adds one factory and one row without changing AVC.

## First implementation plan

1. Finish the removal-lifecycle seam above and add focused tests that removal
   immediately clears both standard and HTC socket/cache registrations.
2. Add a minimal retained selection holder to MMS/runtime code: active tracker
   handle plus selected transport key. Its replacement operation must serialize
   `remove_subtree(old)` before attaching the new direct child.
3. Add `eye_tracking_option` / `eye_tracking_options` helpers in the base
   microphone scene, following its `audio_input_option` pattern.
4. Create the `info_panel` with exactly the two labels above and route clicks
   through the selection holder/factories.
5. Give each factory the exact current defaults and common eye rotation limits:
   `XREyeTracking.on()` for OSC and `XREyeTrackingHTC.on()` for HTC.
6. Test initial materialization, OSC → HTC → OSC switching, no stale direct
   child, rest-pose release before first replacement sample, and handler/socket
   cleanup after every switch.
7. Validate live with VRChat OSC and an HTC Wave SDK/ALVR feed. Confirm that
   the inactive port is no longer held and that blinking and per-eye gaze both
   transfer correctly.

## Non-goals and open questions

- No simultaneous-source blending or freshness policy beyond the existing
  direct-child arbitration; the selector eliminates that ambiguity.
- No automatic source detection, persisted selection, hot-port rebinding UI,
  or MediaPipe implementation in this slice.
- Decide whether switching should preserve per-transport settings (host, port,
  head-rotation compensation, limits) in scene-authored factories or in a
  session configuration object. Start scene-authored; add persistence only
  when a stable user-settings owner exists.
- Define behavior for a failed new bind. The preferred first policy is to keep
  the selected replacement attached, show its failed/unavailable state in the
  panel, and allow an explicit row click to retry; do not silently resurrect
  the removed tracker.
- If runtime code cannot safely hold/reassign a component handle yet, implement
  that narrow capability first. Rebuilding the entire AVC/avatar subtree is
  explicitly not an acceptable substitute.

## Completion criteria

- The menu contains precisely `VRChat OSC` and `HTC Wave Sdk\n(experimental)`.
- A click leaves exactly one direct eye-tracking child beneath AVC.
- Old tracker UDP/socket/cache state and its scoped handlers are gone at
  removal, not merely discovered by a later full-world scan.
- AVC releases old gaze/blink ownership safely and accepts the selected
  transport's first valid samples without avatar or GLTF reinitialization.
- The transport-neutral base and the HTC companion scenes remain valid; before
  the selector implementation the base uses OSC, and afterward it exposes both
  supported choices.

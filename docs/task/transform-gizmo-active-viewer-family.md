# Task: Choose transform gizmo camera family from active interaction

Date: 2026-07-29

Status: open

Epic: [OpenXR presence, runtime-state signals, and active viewer arbitration](epic/openxr-presence-and-active-viewer-arbitration.md)

Related:

- `docs/draft/openxr-session-state-signals-and-mms-api.md`
- `docs/bugs/transform-gizmo-screen-size-varies-with-camera-distance.md`
- `docs/spec/transform-camera-specific.md`
- `src/engine/ecs/system/gizmo_system.rs`
- `src/engine/ecs/system/transform_stream_system.rs`
- `src/engine/ecs/system/system_world.rs`

## Problem

`TransformCameraSpecific` currently produces one effective ECS world matrix. When an active XR
camera has published eyes, the transform stream selects the stereoscopic settings globally.

That policy is wrong for a mixed-camera application such as `bisket-vr-demo`, which has both:

- a `Camera3D` driving the desktop window
- a `CameraXR` driving the headset

An XR runtime may continue presenting through SteamVR or ALVR while nobody is using the headset.
Merely having valid XR eye views must not make XR authoritative for transform-gizmo scale. When
the user edits through the desktop window, the gizmo can otherwise be scaled from the XR camera's
depth and visibly grow or shrink in the wrong direction relative to the desktop camera.

This task is a policy fix for the current single-effective-matrix architecture. It does not add
simultaneous per-render-view world matrices.

## Goal

Choose the camera family used for transform-gizmo depth compensation from the viewer/pointer
family the user is actively interacting through.

The chosen family controls:

- which camera depth updates the gizmo settings scale
- which `TransformCameraSpecific` settings transform becomes effective
- the one world-space gizmo size used by rendering, BVH bounds, and raycasting

## Automatic single-family behavior

Manual focus or prior interaction must not be required when only one usable camera family exists.

- If at least one active `Camera3D`/window camera is available and no usable `CameraXR` is
  available, select monoscopic automatically.
- If a usable `CameraXR` is available and no active `Camera3D`/window camera is available, select
  stereoscopic automatically.
- Re-evaluate this automatically when cameras are enabled, disabled, registered, removed, or lose
  valid published camera data.

The interaction arbitration described below applies only while both camera families are usable.

## Mixed-family arbitration

When both desktop and XR cameras are usable, retain an active gizmo viewer family.

Priority:

1. The pointer that owns an active gizmo drag.
2. The pointer that most recently selected or directly interacted with the gizmo/target.
3. The family with the most recent meaningful user input.
4. Monoscopic as the initial fallback when no interaction history exists.

Meaningful desktop activity includes:

- mouse press/release
- mouse movement or wheel input
- relevant keyboard input
- window focus gained

Meaningful XR activity includes:

- controller, hand, or gaze pointer selection
- trigger/grip press or release
- beginning a gizmo or scene interaction
- an OpenXR focus transition when it reliably represents the headset becoming actively used

The following are not sufficient by themselves to switch ownership to XR:

- an OpenXR session existing
- SteamVR or ALVR presenting frames
- valid XR eye matrices being published
- an XR camera being registered or active
- passive headset pose updates with no evidence of user interaction

Window and OpenXR focus may both remain nominally active depending on the runtime and compositor.
Explicit pointer interaction therefore outranks platform focus state.

When supported, an `XR_EXT_user_presence` transition from absent to present is meaningful XR
activity and may select XR automatically. Presence capability and current presence must come from
the OpenXR state work tracked by the epic; do not infer either from session focus or eye
publication.

## Drag stability

Lock the active viewer family for the lifetime of a gizmo drag.

Unrelated mouse movement, XR pose publication, window focus events, or controller noise must not
change the effective gizmo scale during that drag. Release the lock on drag end or cancellation,
then reconsider the most recent meaningful interaction.

If the locked camera family disappears during a drag, cancel the drag or deliberately fall back
to the remaining usable family; do not continue applying transforms from stale camera data.

## Suggested implementation shape

Introduce a small state value owned by the appropriate editor/gizmo input coordination layer:

```text
GizmoViewerFamily {
    Monoscopic,
    Stereoscopic,
}
```

Track:

```text
active_family
last_desktop_interaction
last_xr_interaction
drag_locked_family
```

Prefer a monotonically increasing interaction sequence over wall-clock timestamps. Update the
sequence at the point where pointer/input ownership is already resolved so that ordering is
deterministic within a frame.

Pass the resolved family explicitly into:

- `TransformGizmoSystem::update_camera_scales`
- `TransformStreamSystem` camera-specific selection

Remove the current implicit rule that XR wins solely because an active XR camera has published
eyes.

## Scope

This policy applies to transform gizmos and their camera-specific visual anchor. It must not make
ordinary renderables camera-relative or change general camera activation/render submission.

It may later become a reusable selection policy for other editor affordances, but that
generalization is not required for this task.

## Tests

Add focused coverage for:

- desktop-only camera availability automatically selects monoscopic without interaction
- XR-only camera availability automatically selects stereoscopic without interaction
- both cameras present with no interaction defaults to monoscopic
- passive XR presentation does not steal ownership from desktop
- desktop pointer selection switches to monoscopic
- XR pointer selection switches to stereoscopic
- the most recent meaningful interaction wins when both families remain present
- an active drag keeps its starting family despite activity from the other family
- ending/cancelling a drag releases the family lock
- removal or invalidation of one family automatically selects the remaining family
- the selected family supplies the depth used by `update_camera_scales`
- the transform stream selects the same family used for depth calculation

Exercise at least one live gizmo topology rather than testing only the arbitration helper.

## Manual validation

In `bisket-vr-demo`:

1. Start SteamVR/ALVR and allow XR frames to present without putting on or interacting through the
   headset.
2. Select and move a cube using the desktop pointer.
3. Confirm the gizmo remains approximately constant in desktop screen size as its depth changes.
4. Interact with a scene object or gizmo through an XR controller.
5. Confirm XR becomes authoritative and the gizmo remains approximately constant in angular size
   in the headset.
6. Begin a drag in either family and generate input in the other family.
7. Confirm the gizmo does not change family or pop in scale until the drag ends.

Also validate desktop-only and XR-only examples without performing any manual focus action.

## Completion criteria

- An unused but presenting SteamVR/ALVR session no longer overrides desktop gizmo scaling.
- Camera-family selection follows resolved interaction ownership when both families coexist.
- Desktop-only and XR-only configurations select their sole usable family automatically.
- Viewer-family selection cannot change during an active gizmo drag.
- Depth calculation and `TransformCameraSpecific` selection use the same resolved family.
- Rendering, BVH bounds, and raycasting continue to observe one coherent effective gizmo scale.

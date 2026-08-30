# Task: Authored active-camera position panel

Date: 2026-08-30

Status: focused tracker; runtime pose bridge and asset not implemented

## Outcome and stop condition

Provide a reusable MMS asset that can be added alongside a camera and displays
the effective world-space position of the active desktop and XR cameras. The
panel is ordinary authored scene content, not editor-owned UI and not managed
by `EditorSystem`.

Stop when the asset updates while a desktop camera moves, updates from the
physical headset pose in XR, handles either or both active camera types, and
the focused evaluator/system tests pass. Do not build a general data-binding
framework, camera inspector, transform editor, or saved settings model in this
slice.

## Requested panel

Create `assets/components/camera_position_panel.mms`, with a small exported
constructor that authors the panel shell and its value text. Its visual
language should match the world settings panel, but with a smaller footprint
and a larger coordinate font.

For each available active camera type, render one group:

- heading: `Desktop` or `XR`;
- three block-like value elements labeled `x`, `y`, and `z`;
- values in world units, quantized to six decimal places; and
- a stable width/sign layout so movement does not constantly reflow the panel.

If only one camera type is active, render one group. If both are active, render
both groups. If neither is active, keep the panel shell and show a concise
`no active camera` state.

The asset must be usable without an `Editor` subtree. A camera scene should be
able to import and author it explicitly; exact attachment syntax can be chosen
with the runtime API below, but it must not require editor registration.

## What already works

The suspected gap is not array access:

- MMS can call `transform.translation()` and receives `[x, y, z]`;
- MMS can access those channels as `position[0]`, `position[1]`, and
  `position[2]`;
- `Text.set_text(...)` can update an existing value label; and
- `Math.round(...)` is sufficient for numeric quantization.

Those transform values are local authored TRS. They are not, by themselves,
the effective active-camera pose required by this panel.

## Actual runtime gap

There is no MMS API to:

1. identify the active window camera and active XR camera;
2. read the current effective world position for either camera type; or
3. receive or poll those values as the camera moves.

`CameraSystem` already tracks `active_window_camera_component()` and
`active_xr_camera_component()` in Rust. A desktop camera position can be
derived from its effective world transform. XR is different: the authored
`CameraXR` transform represents the rig origin, while the visible headset pose
comes from OpenXR views and the `InputXR`-driven transform. Reading
`CameraXR`'s authored parent transform would therefore show the wrong value as
the user moves their head.

## Narrow runtime contract

Add one read-only camera-pose bridge owned by the camera/OpenXR runtime. It
should expose snapshots equivalent to:

```text
active_camera_positions() -> {
    desktop: Option<[f32; 3]>,
    xr: Option<[f32; 3]>,
}
```

Semantics:

- `desktop` is the world translation of the active window `Camera3D`.
  `Camera2D` is out of scope for the first panel.
- `xr` is the world-space center-head pose used for the active XR rig, not one
  eye and not merely the rig origin.
- an entry is `None` when that type has no enabled active camera, or when XR
  has no valid current headset pose;
- a frame must never publish stale XR tracking data as current; and
- values are raw finite world units. Six-decimal quantization is presentation
  behavior in the asset, not storage behavior in the camera system.

Prefer one purpose-built snapshot/read operation over adding active-camera
selection state to `TransformComponent`. The bridge may be surfaced to MMS as
a small runtime component method or host read. Keep camera selection and XR
pose ownership in their existing systems.

## Refresh model

Use the smallest existing delayed-execution mechanism that can resample live
state safely. A short looping `Animation` callback in the MMS asset is
acceptable for the first slice if component reads inside keyframes observe the
current world rather than a captured load-time value. Do not rematerialize the
panel tree every frame; update the existing `Text` components only.

If the current evaluator cannot provide the camera-pose bridge to delayed MMS
callbacks, add that focused host context. Do not introduce a general reactive
graph as part of this task.

Quantize for display with the equivalent of:

```mms
fn quantize_6(value) {
    return Math.round(value * 1000000.0) / 1000000.0
}
```

The implementation must also choose a formatting rule that preserves six
decimal places (including trailing zeroes). `Math.round` alone does not do
that, so a narrow numeric formatting helper may be required.

## Authored and runtime ownership

Authored/serialized:

- panel root, layout, labels, colors, and text placeholders;
- explicit asset invocation/attachment in the camera scene; and
- presentation choices such as update interval and decimal precision.

Runtime-only:

- active camera identities;
- latest valid desktop and center-head XR positions;
- update scheduling state; and
- any cached last-displayed values used to avoid redundant `SetText` intents.

None of that runtime state should serialize into MMS.

## Focused implementation order

1. Add Rust tests for active desktop world position and XR center-head
   position, including nested rig transforms and invalid/stale XR pose.
2. Expose the paired optional snapshot to delayed MMS execution.
3. Add numeric fixed-decimal formatting only if the existing string surface
   cannot preserve trailing zeroes.
4. Build `assets/components/camera_position_panel.mms` from existing authored
   panel/layout primitives.
5. Add a minimal example scene with desktop movement and an XR-capable branch.

## Acceptance checks

- Moving a nested desktop camera changes the displayed world coordinates.
- Moving the XR headset changes the XR values while the rig origin remains
  fixed.
- Moving the XR rig origin offsets the reported headset world position.
- Switching or disabling an active camera updates/removes the matching group.
- Invalid XR tracking never displays a stale pose as current.
- Negative values and values near a six-decimal rounding boundary format
  consistently without `-0.000000`.
- The asset runs outside the editor and round-trips as ordinary authored MMS.
- Removing the panel releases callbacks/handles and leaves no runtime-owned
  scene nodes or text updates behind.

## Deferred

- rotation, scale, view/projection matrices, and camera settings;
- Camera2D;
- editing or teleporting cameras from the panel;
- generalized ECS queries or arbitrary component-property subscriptions;
- a reusable reactive UI/data-binding framework; and
- editor discovery, docking, persistence, and settings integration.

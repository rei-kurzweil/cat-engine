# vtuber-desktop avatar moves backward when W is pressed

## Status

Open bug / example-convention investigation.

## Symptom

In `examples/vtuber-desktop.mms`, W moves the avatar backward relative to its
visible facing direction and S moves it forward. This is observed after running
the Rust `vtuber-desktop` example, which embeds the MMS scene and successfully
opens the window.

## Repro

1. Run `cargo run --release --example vtuber-desktop`.
2. Observe the avatar's visible forward direction.
3. Press W, then S, without rotating the input rig.

## Expected behavior

W moves the avatar in the direction it visibly faces; S moves it opposite that
direction.

## Current evidence

`InputTransformMode.forward_z()` maps W to local `-Z` and S to local `+Z` in
`InputSystem`. The same scene configures its AVC with both
`forward_plus_z()` and `initial_yaw(0.0)`. Those choices describe the avatar
body as +Z-forward while the input rig's W direction is -Z-forward, yielding a
180-degree convention disagreement.

## Investigation / fix options

- Verify the imported PC-Rei GLTF's authored forward direction at rest.
- Compare this scene with the current desktop and VR AVC conventions.
- If the asset is +Z-forward, update the example's input/body configuration so
  both use that convention (or restore the appropriate initial yaw).
- If the conventions are already intended to agree, add a focused regression
  test covering W/S world displacement versus model forward.

## Relevant code

- `examples/vtuber-desktop.mms`
- `src/engine/ecs/system/input_system.rs`
- `src/engine/ecs/system/avatar_control_system.rs`
- `src/engine/ecs/component/transform_temporal_filter.rs`


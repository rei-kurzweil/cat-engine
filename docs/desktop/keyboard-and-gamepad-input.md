# Keyboard and regular gamepad events

Date: 2026-09-02

Status: proposed. Raw desktop keyboard edges already exist; a general authored
handler surface and non-XR gamepad provider do not yet have an implementation
tracker, so this page owns that gap.

[Back to the desktop workbench](README.md)

## Outcome

Let Rust systems and MMS components respond to ordinary computer keyboards and
regular desktop gamepads through a consistent event API, while preserving
focus, hotplug, repeat, dead-zone, and per-device semantics.

`InputXRGamepad` is related prior art, not the desktop gamepad abstraction. An
Xbox/PlayStation-style controller connected to the computer must work without
an OpenXR session or XR interaction profile.

## Existing pieces and gaps

- `src/engine/user_input.rs` already stores keyboard `down`, `pressed`, and
  `released` state from winit, plus a separate text-input event stream.
- [Editor focus and mode shortcuts](../draft/editor-input-focus-and-mode-shortcuts.md)
  explains why raw keyboard state is not enough: text-entry focus, editor
  shortcut focus, and panel routing are distinct.
- [Input → intent → data flow](../spec/input-intent-data-flow.md) documents the
  current raw-input and gesture pipeline.
- [MMS event payloads and runtime attach](../task/mms-event-payloads-and-runtime-attach.md)
  tracks the general event-payload bridge needed for useful authored handlers.
- XR buttons/axes have existing components and events, but no general desktop
  gamepad acquisition/state path was found.

## Proposed layering

```text
winit keyboard        desktop gamepad backend       OpenXR actions
       \                       |                         /
        device-specific acquisition and identity
                         |
          normalized buttons / axes / key edges
                         |
       focus + routing + dead-zone/repeat policies
                         |
           Rust handlers and MMS event payloads
```

Normalization should allow higher-level actions such as `move`, `jump`, or
`menu_back` later, while still exposing raw named controls for diagnostics and
authoring.

## Contract decisions

- Distinguish key identity used for shortcuts from produced text. Do not
  reconstruct text input from key-down events.
- Define whether keyboard handlers subscribe globally, to a focus scope, or to
  an explicit input component. Text fields must be able to consume/suppress
  conflicting shortcuts.
- Define key-repeat semantics separately from the first `down` edge.
- Give each gamepad a stable runtime identity plus connection/disconnection
  events; do not silently merge multiple pads.
- Normalize common face buttons/sticks where possible while retaining backend
  control identifiers for unsupported layouts.
- Apply configurable dead zones and emit axis changes only beyond a documented
  epsilon to avoid per-frame noise.
- Keep low-level input events observational. Gameplay/editor policies emit
  higher-level intents rather than mutating unrelated components directly.

## Milestones

- [ ] Inventory every current consumer of `InputState` and every MMS/Rust input
      event kind so the new path does not duplicate shortcut or movement logic.
- [ ] Specify keyboard payloads and names for down, up, and optional repeat,
      including modifiers and logical/physical identity.
- [ ] Add an engine-owned keyboard event producer with deterministic frame
      ordering and focused-routing tests.
- [ ] Bind keyboard payloads into MMS handlers and add a small key dashboard
      example that is independent of movement controls.
- [ ] Select a cross-platform desktop gamepad backend and document platform,
      hotplug, and build implications before adding the dependency.
- [ ] Add per-device button/axis snapshots and edge/change events.
- [ ] Bind gamepad events into the same Rust/MMS event layer without routing
      them through OpenXR types.
- [ ] Add a regular-gamepad dashboard and a locomotion/action-mapping example.
- [ ] Test keyboard focus suppression, held/repeated keys, two simultaneous
      gamepads, disconnect/reconnect, dead zones, and headless/no-device runs.

## Suggested first slice

Keyboard handlers first: the acquisition and edge state already exist, so this
proves event production, focus routing, payload conversion, and MMS authoring.
Then add the desktop gamepad provider behind the same event boundary.

## Related XR work

- [OpenXR controller actions and stick locomotion](../task/openxr-controller-actions-and-default-stick-locomotion.md)
- [XR gamepad and hand input refactor](../task/xr-gamepad-and-hand-input-refactor.md)

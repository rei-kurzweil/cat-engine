# Task: Authored transform info panel

Date: 2026-08-30

Status: focused tracker; asset and fixed-decimal formatter not implemented

## Outcome and stop condition

Provide a reusable MMS asset that displays the local translation of an
explicitly supplied `TransformComponent`. The panel is ordinary authored scene
content, not editor-owned UI and not managed by `EditorSystem`.

Stop when a caller can pass a transform to the asset and its x/y/z values
update at the requested cadence without rebuilding the panel tree. Do not
build active-camera discovery, an XR pose bridge, a general data-binding
framework, a transform editor, or a saved settings model in this slice.

## Requested panel

Create `assets/components/ui/transform_info_panel.mms`, with a small exported
constructor that accepts the transform to inspect:

```mms
transform_info_panel(target)
```

It authors the panel shell and three persistent value `Text` components. Its
first block element is the literal title `telemetry`, followed by one line for
each local translation channel:

- `x: 999.99999`
- `y: 999.99999`
- `z: 999.99999`

The final presentation uses exactly five decimal places and a stable
width/sign layout. The asset must be usable without an `Editor` subtree; any
scene may import it and pass any transform explicitly.

## What already works

- MMS can call `transform.translation()` and receives `[x, y, z]`;
- MMS can access those channels as `position[0]`, `position[1]`, and
  `position[2]`;
- `Text.set_text(...)` can update an existing value label; and
- `Math.round(...)` is sufficient for numeric quantization.

Those transform values are the local authored TRS, which is the intentional
coordinate space for this panel.

## Refresh model

The existing global `FrameTick` signal is the refresh mechanism. The panel
subscribes with `on_global("FrameTick", fn(event) { ... })`, accumulates
`event.dt_sec` in a heap-backed local state table, and samples
`target.translation()` every 0.1 seconds (10 Hz). It updates the existing
`Text` components only; it never rematerializes its authored subtree.

`FrameTick` is emitted once per rendered frame while a global subscriber
exists, so no MMS sleep, timeout, animation, polling API, or new scheduler is
needed for this asset.

## Fixed-decimal formatting TODO

The requested output has exactly five fractional digits. `Math.round` is
sufficient for quantization, but MMS currently stringifies numbers with their
ordinary minimal representation. A narrow fixed-decimal formatting helper is
therefore required before the panel can reliably show `999.00000`.

The helper should:

- round to five fractional digits;
- preserve trailing zeroes;
- normalize negative zero to `0.00000`; and
- avoid changing unrelated numeric or string behavior.

This helper is intentionally deferred from the tracker-first slice; it is not
a reason to add camera, OpenXR, or world-transform APIs.

## Authored and runtime ownership

Authored/serialized:

- panel root, layout, labels, colors, and text placeholders;
- explicit asset invocation and its target transform; and
- presentation choices such as update interval and decimal precision.

Runtime-only:

- the frame-update accumulator; and
- any cached last-displayed values used to avoid redundant `SetText` intents.

None of that runtime state should serialize into MMS.

## Focused implementation order

1. Add the fixed-decimal MMS formatting helper and focused tests.
2. Build `assets/components/ui/transform_info_panel.mms` from existing
   authored panel/layout primitives.
3. Add an evaluator/example test that passes a moving transform and observes
   10 Hz text updates.

## Acceptance checks

- Passing any transform shows that transform's local x/y/z translation.
- Moving the passed transform updates the three values at roughly 10 Hz.
- The title is `telemetry`, followed by x, y, and z lines with five fixed
  decimal places once the formatter TODO lands.
- Negative values and values near a five-decimal rounding boundary format
  consistently without `-0.00000`.
- The asset runs outside the editor and round-trips as ordinary authored MMS.
- The asset updates its existing text nodes only and releases its global
  handler when removed.

## Deferred

- world-space position, rotation, scale, view/projection matrices, and camera
  settings;
- active-camera discovery and XR headset pose inspection;
- editing or teleporting transforms from the panel;
- generalized ECS queries or arbitrary component-property subscriptions;
- a reusable reactive UI/data-binding framework; and
- editor discovery, docking, persistence, and settings integration.

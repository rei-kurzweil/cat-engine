# Bisket desktop Pose panel clips the Apply action

## Status

Open visual/layout investigation. No source changes made.

## Symptom

In `bisket-desktop-demo`, the Pose panel appears too narrow for its captured
pose rows. The visible row has no usable **Apply** button: it may be clipped
past the inline edge rather than absent from the component tree.

## Evidence so far

The pose-row renderer explicitly creates `pose_apply_action` with the label
`Apply`, and its focused unit test asserts that the action exists. This makes a
missing action less likely than a width, flex, clipping, or hit-target layout
failure.

The authored Pose-panel shell has a fixed `POSE_PANEL_WIDTH_GU = 29.5`, while
the dynamic row uses a flex container containing editable name/capture controls
and an Apply action. The layout must be inspected at runtime to determine
whether the row itself overflows, the panel is assigned less width than its
authored shell expects, or the button exists but is outside the scroll/clip
region.

## EditorUI configuration gap

`EditorUI.panels([...])` currently chooses which panels are materialized. Its
only typed per-panel configuration is for Settings visibility toggles; it has
no supported `width`, `height`, placement, or per-panel layout configuration.

That means the desired authoring contract does not yet exist: an example can
request the Pose panel, but cannot declare the width it needs through
`EditorUI`. This should be designed deliberately rather than relying on
hard-coded panel MMS constants or an accidental runtime layout width.

## Repro

1. Run `cargo run --release --example bisket-desktop-demo`.
2. Open the Pose panel and select a Bisket pose library with one or more poses.
3. Inspect a pose row at the panel's right edge.
4. Compare its visible width and clickability with the live
   `#pose_apply_action` node in the component tree.

## Required measurements

- Pose-panel shell resolved width and clip/scroll bounds.
- Pose-row flex container resolved width.
- Name field, Capture/Reset/Save controls, and `pose_apply_action` local boxes.
- Whether `pose_apply_action` is rendered, clipped, or loses raycast hits.
- The panel-layout allocation supplied to Pose versus its authored
  `POSE_PANEL_WIDTH_GU` request.

## Likely related work

- `docs/bugs/layoutroot-available-width-does-not-constrain-explicit-panel-widths.md`
- `docs/bugs/pose-panel-captured-pose-text-overlap-and-slot-routing.md`
- `assets/components/internal/panels.mms`
- `src/engine/ecs/system/editor/pose_panel.rs`
- `src/engine/ecs/component/editor_ui.rs`

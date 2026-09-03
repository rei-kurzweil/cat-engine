# Bisket desktop Pose panel clips the Apply action

## Status

Closed as misdiagnosed. No source changes made.

## Symptom

The original observation was made while the pose library had no loaded poses.
That made the panel look incomplete, but there was no pose row on which to
render an **Apply** action. Once the library's poses are present, the row has
both editable pose name and Apply controls as intended.

## Evidence so far

The pose-row renderer explicitly creates `pose_apply_action` with the label
`Apply`, and its focused unit test asserts that the action exists. Visual use
with loaded poses confirms that this is not an Apply-button clipping bug.

There is still a separate header-layout problem: the editable pose-library name
can outgrow its white input background and overlap following controls. That is
not evidence that the Pose panel should receive a larger global fixed width.
It is tracked in
`docs/bugs/text-input-intrinsic-size-in-layoutroot.md`.

## Follow-up: EditorUI configuration gap

`EditorUI.panels([...])` currently chooses which panels are materialized. Its
only typed per-panel configuration is for Settings visibility toggles; it has
no supported `width`, `height`, placement, or per-panel layout configuration.

That means a future request for per-panel sizing needs a deliberate authoring
contract. It is not required to fix the original Apply observation, and should
not be conflated with intrinsic sizing of the editable library-name input.

## Related work

- `docs/bugs/layoutroot-available-width-does-not-constrain-explicit-panel-widths.md`
- `docs/bugs/pose-panel-captured-pose-text-overlap-and-slot-routing.md`
- `assets/components/internal/panels.mms`
- `src/engine/ecs/system/editor/pose_panel.rs`
- `src/engine/ecs/component/editor_ui.rs`

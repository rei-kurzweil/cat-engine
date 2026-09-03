# Pose panel cannot delete or recapture an existing pose

## Status

Open product and editor-action tracker. No source changes made.

## Symptom

Once a pose library is loaded, its rows correctly expose an editable name and
an **Apply** button. They do not offer a way to delete a captured pose or to
replace that pose's stored joint entries with a newly captured version.

The header-level **Capture** action always captures a new pose. It does not
target the selected/existing row for replacement.

## Current contract and evidence

The documented implemented pose-library contract intentionally left rename,
delete, reorder, dirty tracking, per-pose save, and import UI out of scope.
The current `PosePanelActionKind` has `Capture`, `RenamePose`, `Select`, and
`Apply`, but no `Delete` or `Recapture` action. The renderer correspondingly
creates only the row body/name input and Apply button.

Capture uses the library's current count to name a newly appended pose when no
name is specified. It produces a new `PoseCapturePoseComponent`; it does not
mutate an existing pose component.

## Required behavior

- Each existing pose row provides a deliberate delete action.
- Each existing pose row provides a deliberate recapture/replace action that
  snapshots the current target into that same pose identity and preserves its
  user-facing name unless the user changes it.
- Both actions identify the target, library, and pose explicitly in their
  payload; neither may depend on global editor selection accidentally pointing
  to the correct row.
- Deleting or recapturing marks the owning `PoseCaptureComponent` unsaved,
  refreshes the data-rendered row model, and keeps local pose selection valid
  (select a documented neighbor or clear it if the deleted row was selected).
- **Save** remains the persistence boundary: after a successful save, the
  manifest and numbered generated pose modules match the remaining ordered
  library children. The existing stale-generated-file cleanup should remove a
  deleted pose's obsolete module at that time.
- Destructive delete must be clearly labelled and should have an explicit
  confirmation policy before it becomes one-click in a live editor.

## Design questions

- Should recapture be an immediate replacement, or first open/enter an
  overwrite confirmation state? It is destructive to the old joint data but
  easy to reverse before Save.
- Does Delete remove the runtime pose immediately and defer filesystem changes
  to Save, or should the panel offer a separate discard/reload path too?
- Where should the controls sit so the row remains compact: explicit text
  buttons, an overflow menu, or a selected-row action strip?
- Should a library-level delete action be a separate future scope? This tracker
  is limited to individual `PoseCapturePoseComponent` rows.

## Acceptance coverage

1. Capture two poses, delete one, save, and verify the reloaded manifest
   contains only the surviving pose and stale generated module is removed.
2. Recapture a named pose after changing joints; Apply uses the new entries and
   the row name/order is unchanged.
3. Delete and recapture both rerender the active Pose panel without changing
   `EditorContextState.selected_component` unexpectedly.
4. Failed capture or failed save leaves the prior pose data and persisted files
   intact, with a visible status message.

## Related

- `docs/task/pose-library-row-actions-and-manifest-saving.md`
- `src/engine/ecs/system/editor/pose_panel.rs`
- `src/engine/ecs/system/pose_capture_system.rs`
- `src/engine/ecs/component/pose_capture.rs`

# Editor UI live MMS panel validation slices

Date: 2026-09-05

Status: planned; no implementation or validation completed

## Goal

Implement [live MMS panel instantiation and side effects](./editor-ui-live-mms-panel-instantiation-and-side-effects.md)
in small behavioral slices, then migrate the shared editor shell/layout path and verify the real
paint/assets workflow. Prove callback delivery and ownership before changing every panel.

## Current code constraints

- `src/scripting/runner.rs`: the existing live module factory helper calls
  `call_mms_module_fn` without evaluator channels. It can produce live component handles, but
  `on(...)` in `src/scripting/world_evaluator.rs` only registers handlers when channels exist.
  Replacing materialization with that spawn helper alone does not establish live panel semantics.
- `RuntimeSpecSession` already has behavioral tests for retained callbacks, component mutation,
  captured table state, session isolation, and disabling delivery on close. Use this retained
  execution model as the starting point. Closing a session makes registrations inert; physical
  registration cleanup still needs an explicit owner.
- `src/engine/ecs/system/panel_system.rs` exposes individual panel spawning separately from editor
  layout assembly. Its existing 100-cycle body restoration test checks component counts and
  preserved transforms, but not callbacks or captured state.
- Production `spawn_editor_panel_layout_tree` still assembles panel shells into a
  `MaterializedCE` layout tree. Changing `spawn_panel_instance` alone does not migrate the editor.

These observations are from code inspection, not a fresh test run.

## Slice 1: Install one live panel fixture

Introduce the smallest panel installation seam that evaluates an imported MMS factory through
retained live execution, attaches its returned shell under an editor root, discovers its slots,
and retains the execution owner. Establish where the owning runtime services callbacks.

The fixture captures a text or style component and updates it from a click handler.

Acceptance:

- [ ] The factory returns a live shell attached to the intended editor mount.
- [ ] Dispatch a click after installation returns, using the engine event path and callback service.
- [ ] The captured handle updates the actual attached component, with no callback errors.
- [ ] Initialization and attachment do not duplicate the shell or its handlers.

Keep this fixture small; it does not require migrating all production panels.

## Slice 2: Instance isolation and removal

Extend the factory to capture a mutable counter and install two instances.

Acceptance:

- [ ] Multiple events preserve each instance's counter across dispatches.
- [ ] Clicking one instance changes neither the other instance's state nor its components.
- [ ] Removing an instance releases or invalidates its callbacks and retained state.
- [ ] A callback queued before removal cannot act on removed components afterward.
- [ ] The surviving instance continues to work.
- [ ] Registration cleanup has an explicit lifecycle owner; repeated creation/removal does not
  accumulate active handlers or retained sessions.

Slices 1–2 are the first implementation chunk: one installation seam with working callback
delivery and teardown. A successful component spawn by itself is not sufficient.

## Slice 3: Replace dynamic rows beneath a live shell

Populate the fixture's content slot through the existing dynamic projection machinery. Attach,
replace, and remove rows while continuing to dispatch the shell's event handler.

Acceptance:

- [ ] Shell, stable slot, and captured control identities remain unchanged.
- [ ] Shell callback state persists through row updates.
- [ ] Old rows disappear and replacement rows attach beneath the intended slot.
- [ ] Row refreshes neither reevaluate the shell factory nor duplicate shell handlers.

Do not redesign all row generation as part of this slice.

## Slice 4: Collapse and restore with coherent ownership

Choose and document the callback lifetime boundary for shell and removable body. A callback
capturing a body component must not outlive that component as a callable handler.

Extend the existing repeated restoration fixture with body-owned callbacks and captured state.

Acceptance:

- [ ] Collapse removes the body and invalidates its callbacks, including pending delivery.
- [ ] Restoration evaluates the replacement body through live execution and rediscovers its slots.
- [ ] Restored handlers capture new body identities and fresh body-local state.
- [ ] Shell transform and shell-owned state remain stable where the shell survives collapse.
- [ ] Repeated cycles produce one response per event, stable component counts, and no growing
  collection of retained body sessions or handlers.

## Shared editor installation cutover

After the fixture slices pass, migrate the shared shell/layout installation path together:

- [ ] Create the live layout mount, then install panel shells beneath it.
- [ ] Replace ordinary shell CE decoration with authored structure or live component operations.
- [ ] Return discovered `PanelInstance`s to editor orchestration.
- [ ] Route body restoration through the same live ownership facilities.
- [ ] Preserve dynamic slot projection independently of shell creation.

Explicit template consumers may keep materialization. This task does not require removing every
`MaterializedCE` use or fully decomposing the stopgap adapter.

## Slice 5: Color panel and paint/assets integration gate

Move the active-color-well update into the color panel's MMS selection handler. Preserve Rust's
observation of the same `SelectionChanged` event for editor paint state.

Acceptance:

- [ ] Palette selection updates the active well through MMS without recoloring palette swatches.
- [ ] The same event reaches the ancestor Rust observer and updates paint state once.
- [ ] Selecting an asset still bootstraps the expected selection payload and enables placement.
- [ ] Painting uses the selected asset and applicable color state.
- [ ] Collapse/restore, select again, and paint again: controls and placement continue to work.
- [ ] Multiple editor instances retain correct event routing and instance ownership.
- [ ] Visually verify panel layout, palette feedback, asset selection, and painted output.

Reuse the integration fixtures in `src/engine/ecs/system/editor_paint_system.rs`, including
`asset_selection_bootstraps_to_data_payload`,
`palette_selection_updates_active_well_without_recoloring_swatches`, and
`paint_places_when_focused_and_asset_selected`. Adapt them to assert the new MMS ownership,
and retain relevant existing routing regressions.

The paint/assets workflow is the final integration gate. It does not replace the focused tests
for shared closure state, queued callbacks, stale targets, and handler accumulation.

## Related

- [Editor UI live MMS panel instantiation and side effects](./editor-ui-live-mms-panel-instantiation-and-side-effects.md)
- [Live panel factory spawn and stopgap adapter seams](./live-panel-factory-spawn-and-stopgap-adapter-seams.md)

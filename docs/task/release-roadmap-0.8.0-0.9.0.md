# Mittens release roadmap: 0.8.0 and 0.9.0

Date: 2026-08-06

Status: active canonical release roadmap

## Purpose

Track the work from the currently published `mittens-engine 0.7.0` to two
focused releases:

- `0.8.0` ships the completed humanoid-map/API replacement and makes the
  editor's grid, paint, and minimized-panel restoration paths dependable; and
- `0.9.0` completes the Meow Meow Script ownership cutover and deletes the
  duplicated engine-side MMS runtime.

There will be no `mittens-engine 0.7.1` or `0.7.2` publication. Work previously
planned for those versions is either included in `0.8.0` when listed below or
retargeted to `0.9.0`.

Do not bump manifests at the start of a milestone. Bump, update the lockfile,
package, and publish only after that milestone's gates pass.

## Release sequence

```text
published 0.7.0
   |
   v
0.8.0: humanoid-map API + reliable grid/paint + reliable panel restoration
   |
   v
0.9.0: one MMS evaluator/runtime owner; engine duplication removed
```

## 0.8.0: editor reliability and avatar API cleanup

`0.8.0` is intentionally a focused engine release. The public AVC bone-name
surface has been replaced rather than retained as a compatibility layer, so
the next release uses a new pre-1.0 minor version.

### Humanoid mapping and AVC cleanup

The implementation is tracked by
[Humanoid bone map, conservative automapping, and MMS presets](humanoid-bone-map-automapping-and-mms-presets.md).

- [x] Replace the old `BoneMappingSystem` with the GLTF-scoped
      `HumanoidBoneMapSystem`.
- [x] Add deterministic explicit, absent, preset, and conservative automapping
      behavior with diagnostics.
- [x] Make AVC consume the resolved semantic map rather than its old per-bone
      name fields and builder calls.
- [x] Remove the old AVC bone-name MMS and Rust configuration surface.
- [x] Update the maintained avatar examples and mapping presets.
- [x] Record the deliberate AVC API break and migration path in the 0.8 release
      notes.
- [ ] Complete any final desktop/headset smoke checks selected for the release.

The deferred humanoid-map editor UI is not a 0.8 release gate unless separately
promoted into this roadmap.

### Grid and paint reliability

The short gate is
[Editor grid and paint 0.8 release gate](editor-grid-paint-0.8.0-release-gate.md),
with the detailed dependency order and acceptance matrix in
[Grid + Gizmo + Paint end-to-end UX](grid-gizmo-paint-end-to-end-ux-and-test-matrix.md).

- [ ] Triage the current real-application grid and paint behavior and record a
      reproducible baseline.
- [ ] Make rendered grid lines, the selected grid frame, spacing, and snap math
      agree.
- [ ] Resolve active-grid selection and translated/rotated/non-unit grid
      behavior.
- [ ] Restore deterministic asset selection, Free Draw activation, preview,
      placement, and cross-renderable stroke behavior.
- [ ] Make paint quantize through the selected active grid without requiring
      the grid visual itself to win the raycast.
- [ ] Implement Line on top of the stabilized grid/paint primitives, or visibly
      disable it and explicitly defer it before release.
- [ ] Pass the focused automated coverage and desktop/XR manual matrix selected
      by the grid/paint gate.

### Accordion panel restoration regressions

The original lifecycle is documented in
[Editor panel minimize and render suspension](editor-panel-minimize-and-render-suspension.md).
The open regression is tracked by
[Accordion panel restoration loses content and corrupts backgrounds](../bugs/accordion-panel-restore-content-and-background-corruption.md).

- [ ] Reproduce Asset-panel minimize/restore and confirm why the restored body
      content does not render.
- [ ] Reproduce the oversized bright emissive-yellow background quad after
      restoring the World and Asset panels.
- [ ] Determine whether the background symptom is a duplicate renderable,
      stale renderer/layout registration, incorrect restored topology, or an
      existing background resized from stale layout state.
- [ ] Rebind every restored body-owned slot/control/render registration from
      fresh component IDs and refresh once from current model state.
- [ ] Verify that repeated minimize/restore cycles do not accumulate nodes,
      renderables, backgrounds, handlers, slot registrations, or layout state.
- [ ] Add focused regression coverage for Asset and World panel restoration.

### 0.8 stabilization and publication

- [ ] Update README/version-visible strings and prepare 0.8 release notes.
- [ ] Run the focused humanoid, grid, paint, accordion, layout, renderer, and
      editor tests.
- [ ] Run the complete crate/workspace test baseline and classify any remaining
      failures.
- [ ] Complete selected desktop and XR smoke tests.
- [ ] Bump `mittens-engine` to `0.8.0` and update the lockfile atomically.
- [ ] Run `cargo package -p mittens-engine` and inspect the packaged file list.
- [ ] Publish `mittens-engine 0.8.0` only after all selected gates pass.

`meow-meow-script` does not need a release solely because `mittens-engine`
ships 0.8. Publish it only if its packaged API or behavior changes as part of
the selected work.

## 0.9.0: MMS ownership cutover

`0.9.0` is the architectural MMS release. Its gate is deletion of the
duplicated engine-side evaluator/runtime model, not the first appearance of a
builder or standalone crate example.

The canonical phase plan is
[Mittens MMS ownership cutover for 0.9](mittens-mms-ownership-cutover-and-0.9-release.md),
and the implementation checklist is
[MMS evaluator deduplication](mms-evaluator-deduplication.md).

- [ ] Complete one crate-owned strict `RuntimeSpec` and opaque Mittens host
      bindings; remove parallel vocabulary/capability catalogs.
- [ ] Add the host-independent persistent session/client boundary and cut
      ordinary source, file, world, asset, and example evaluation over.
- [ ] Move modules, imports, exports, factories, and shared heap identity into
      crate-owned sessions.
- [ ] Replace stored engine closures with opaque callback handles and migrate
      handlers, keyframes, animation callbacks, and audio lookahead.
- [ ] Move the programmatic REPL and worker protocol into the crate boundary.
- [ ] Delete `src/scripting/world_evaluator.rs` and engine-local MMS values,
      heaps, closures, module state, and legacy conversion/protocol paths.
- [ ] Pass parity, specification, host, module, callback, worker, runner, REPL,
      example, package, compile-fixture, and workspace suites.
- [ ] Publish direct-embedder migration notes and document deliberate Mittens
      Rust API changes.
- [ ] Version and publish `meow-meow-script` as required, then bump and publish
      `mittens-engine 0.9.0` with dependency requirements and lockfile updated
      atomically.

Fixed-width MMS numerics, intrinsic collection/string/numeric methods, typed
binding syntax, inference, static checking, transpilation, and language-server
work do not gate 0.9 unless they deliberately change the supported runtime
specification or boundary protocol.

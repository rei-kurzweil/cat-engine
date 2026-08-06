# Mittens release roadmap: 0.7.1, 0.7.2, and 0.8.0

Date: 2026-08-06

Status: active release roadmap

## Purpose

Define smaller, user-visible releases while the larger Meow Meow Script
ownership cutover continues. This document is the release-level index; focused
tasks retain their detailed implementation and test checklists.

The governing compatibility decision is:

- ordinary Mittens applications and authored MMS scenes should remain
  behavior-compatible through the scripting migration;
- custom script-interpreter configuration was not a supported Mittens feature,
  so replacing internal registries with an equivalent crate-owned catalog is
  not considered a breaking Mittens user experience;
- `meow-meow-script` is pre-1.0 and its direct embedder API may change when the
  configuration/session surface is made real; and
- `mittens-engine` reaches `0.8.0` when the duplicated engine-side MMS
  evaluator/runtime model is removed, because that is the honest architectural
  boundary even when ordinary scene behavior is preserved.

Do not bump manifests at the start of a milestone. Bump, update the lockfile,
package, and publish only after that milestone's gates pass.

## Release sequence

```text
0.7.1: avatar setup + editor creation tools are dependable
   |
   v
0.7.2: first supported MMS host-catalog/configuration foundation
   |
   v
0.8.0: one MMS evaluator/runtime owner; engine duplication removed
```

## 0.7.1: avatar setup and editor creation reliability

This is the next crates.io release. It deliberately does not wait for the MMS
ownership cutover.

### Humanoid mapping and VR hand setup

The release slice is tracked by
[Shared humanoid bone map, conservative automapping, and MMS presets](humanoid-bone-map-automapping-and-mms-presets.md).

- [ ] Add one GLTF-instance-scoped humanoid semantic map with inspectable
      provenance and diagnostics.
- [ ] Define merge semantics as an ordered union:
  1. explicit per-slot `Absent` or reference decisions win;
  2. existing explicit AVC fields override map/preset decisions during the
     compatibility period;
  3. embedded metadata and convention presets fill only unspecified slots;
  4. conservative name/topology/rest-geometry inference fills only remaining
     unspecified slots; and
  5. ambiguous or invalid candidates remain unresolved rather than silently
     replacing an explicit decision.
- [ ] Make `Legacy`, `ExplicitOnly`, and `Auto` behavior explicit and tested.
- [ ] Minimize scene boilerplate for a normal humanoid while retaining exact
      per-model MMS presets as the escape hatch.
- [ ] Let AVC consume validated head, neck, hand, and arm mappings without
      repeated world-wide name lookup.
- [ ] Derive VR hand orientation from mapped rest-geometry landmarks through
      `JointRetargetBasis`/`RestAttachment`, with documented weaker fallbacks
      when full palm landmarks are unavailable.
- [ ] Provide an inventory/dry-run report that explains selected, overridden,
      ambiguous, missing, and intentionally absent slots before AVC mutates the
      armature.
- [ ] Validate at least Bisket, PC-Rei, a helper/twist-bone rig, and an
      intentionally incomplete/nonhumanoid rig.
- [ ] Complete headset verification for controller and articulated-hand paths.

Local LLM proposals are not a `0.7.1` gate. They may consume the same inventory
and validation pipeline later, but cannot override authored decisions or bypass
deterministic validation.

### Grid, paint, and Line

The release gate is summarized in
[Editor grid and paint 0.7.1 release gate](editor-grid-paint-0.7.1-release-gate.md),
with the detailed dependency matrix in
[Grid + Gizmo + Paint end-to-end UX](grid-gizmo-paint-end-to-end-ux-and-test-matrix.md).

- [ ] Restore the real-app asset-selection -> paint-state -> visible placement
      path.
- [ ] Make the selected grid's rendered lines, frame, spacing, and snap math
      agree.
- [ ] Fix grid creation/orientation, panel refresh, selection, visibility, and
      gizmo alignment regressions required by painting.
- [ ] Make Free Draw click/drag lifecycle, cross-renderable continuity,
      preview, commit, and snapping deterministic.
- [ ] Implement Line as a grid-cell stroke driven by `DragStart`, `DragMove`,
      and `DragEnd`, with one object per unique cell and cell-centered
      translation.
- [ ] Run the desktop/XR manual matrix and retain focused automated coverage.

### General release quality

- [x] Editor panels can be minimized and suspend their body subtree.
- [ ] Resolve or deliberately quarantine the current parallel engine library
      test failures; deterministic catalog/documentation failures must be fixed.
- [ ] Finish the relevant example and headset smoke tests.
- [ ] Update release notes and package `mittens-engine 0.7.1`.

`meow-meow-script` does not need a release solely because `mittens-engine`
ships `0.7.1`; publish it only if this milestone changes its packaged API or
behavior.

## 0.7.2: MMS runtime specification and host configuration foundation

This release ships the start of the supported configuration/catalog work
without claiming that the ownership cutover is complete.

- [ ] Refresh direct-embedder API fixtures and decide the supported initial
      configuration surface.
- [ ] Replace the transitional flat catalog with the crate-owned nested
      `RuntimeSpec`/builder model, or document a deliberately smaller stable
      first slice if nesting is staged.
- [ ] Cover components, aliases, constructors, builder calls, properties,
      positionals, methods, builtins, signals, namespaces, and host APIs needed
      by the first supported slice.
- [ ] Make strict registered-name and signature validation deterministic.
- [ ] Bind effectful operations through opaque IDs so configuration metadata
      and host implementation cannot become independent catalogs.
- [ ] Add catalog consistency, duplicate/conflict, missing-binding, strict-name,
      signature, and custom-host unit tests.
- [ ] Assemble the Mittens catalog from the same API and verify that it
      describes the existing script-visible surface without changing ordinary
      authored-scene behavior.
- [ ] Keep the legacy engine evaluator operational behind the new catalog until
      the later session/cutover milestones are ready.
- [ ] Publish configuration guidance that distinguishes implemented APIs from
      the `SessionClient`/REPL target architecture.

Version policy for this milestone:

- bump `mittens-engine` to `0.7.2` if it ships the catalog integration;
- bump `meow-meow-script` to the appropriate next pre-1.0 version for its
  actual direct-embedder API change; align it with the Mittens version when
  practical, otherwise increment from its latest published version; and
- update the Mittens dependency requirement and lockfile atomically whenever
  the MMS crate version changes.

The remaining session, module, callback, and REPL work is not required to call
the catalog foundation useful, but documentation must not imply the legacy
evaluator has already been removed.

## 0.8.0: MMS ownership cutover

`mittens-engine 0.8.0` is the architectural release. Its gate is deletion of
the duplicated engine-side MMS implementation, not the first appearance of the
builder.

### Chunk A: specification and Mittens bindings

- [ ] Finish any catalog coverage deferred from `0.7.2`.
- [ ] Remove parallel `HostCapabilities`, supported-name, parser-name, signal,
      and string method-support catalogs.
- [ ] Ensure every declared effectful operation has exactly one binding.

### Chunk B: ordinary evaluation

- [ ] Add the host-independent persistent session/client boundary.
- [ ] Cut ordinary source, file, world, render-asset, and example evaluation
      over to the crate runner/session.

### Chunk C: modules and delayed behavior

- [ ] Move modules, imports, exports, template/live factories, and shared heap
      identity into crate-owned sessions.
- [ ] Replace stored engine closures with opaque callback handles.
- [ ] Migrate handlers, keyframes, animation callbacks, and audio lookahead.

### Chunk D: REPL and deletion

- [ ] Move the programmatic REPL and worker protocol into the crate boundary.
- [ ] Delete `src/scripting/world_evaluator.rs`, engine-local MMS values/heaps,
      module state, closure bodies, and legacy conversion bridges.

### Chunk E: stabilization and release

- [ ] Pass pure parity, specification consistency, host integration, module,
      callback lifetime, worker, runner, REPL, example, package, and workspace
      suites.
- [ ] Publish direct-embedder migration notes for `meow-meow-script`.
- [ ] Document any deliberate Rust API changes in Mittens while preserving the
      intended ordinary application/MMS authoring experience.
- [ ] Bump `mittens-engine` to `0.8.0`; bump `meow-meow-script` as required by
      its latest published version and cutover API changes; update dependency
      requirements and the lockfile together.

## Related documents

- [Mittens MMS ownership cutover](mittens-mms-ownership-cutover-and-0.8-release.md)
- [MMS evaluator deduplication checklist](mms-evaluator-deduplication.md)
- [Mittens/MMS resume checkpoint](mittens-mms-cutover-resume-checkpoint.md)
- [XR controller/hand pose basis and laser alignment](xr-controller-hand-pose-basis-and-laser-alignment.md)
- [Editor panel minimize and render suspension](editor-panel-minimize-and-render-suspension.md)

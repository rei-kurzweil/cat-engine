# Mittens/MMS cutover resume checkpoint

Date: 2026-08-06

Status: active `0.9.0` re-entry checkpoint; `0.8.0` proceeds independently

## Purpose

Record where to resume the MMS ownership cutover after the focused
`mittens-engine 0.8.0` editor/avatar release. The release sequence is
[Mittens 0.8.0 and 0.9.0](release-roadmap-0.8.0-0.9.0.md), the canonical
cutover plan is
[Mittens MMS ownership cutover for 0.9](mittens-mms-ownership-cutover-and-0.9-release.md),
and the detailed implementation checklist is
[MMS evaluator deduplication](mms-evaluator-deduplication.md).

## Current conclusion

The boundary change is not finished. The first standalone
`meow-meow-script` slices work, but Mittens still owns and calls the legacy MMS
evaluator, heap, closure, module, callback, and REPL paths. Do not describe the
ownership cutover as complete from this checkpoint.

`mittens-engine 0.8.0` does not include MMS deduplication. It ships the new
humanoid-map/AVC surface plus grid, paint, and accordion-restoration reliability.
`mittens-engine 0.9.0` is gated on the complete crate-owned catalog/session
cutover and deletion of the duplicated engine runtime.

## What works now

- `meow-meow-script` owns its syntax pipeline and public host protocol types.
- A preliminary runtime/catalog builder and `Session<H>` exist in the crate.
- `StandardHost` can collect emitted and registered roots and resolve local
  attachment relationships into a queryable forest.
- `Runner::standard()` provides a convenient standalone path.
- Crate examples cover a standard host, event-stream host, and JSON-lines host.
- Mittens contains a provisional `MittensHost` adapter and re-exports parts of
  the crate syntax surface.
- Editor accordion minimization removes panel bodies, but Asset and World
  restoration regressions remain open and gate 0.8.
- The GLTF-scoped `HumanoidBoneMapSystem` and AVC API replacement are
  implemented for 0.8.

The last recorded standalone smoke evidence was 44 passing
`meow-meow-script` library tests, successful crate host examples, and a passing
`cargo check -p mittens-engine --lib` with pre-existing warnings. This proves
the standalone slice, not the engine cutover.

## What still crosses the wrong boundary

- `src/scripting/world_evaluator.rs` remains the engine evaluator.
- `src/scripting/runner.rs` still calls the legacy evaluator and owns
  engine-local loaded-module state.
- Engine-local `Value`, `MaterializedCE`, `RuntimeClosure`, heap, and related
  object types remain in use.
- Modules, factories, handlers, keyframes, animations, and audio callbacks
  still depend on engine-owned evaluation or closure state.
- The engine REPL still uses the legacy evaluator handle and worker protocol.
- `MittensHost` still derives capabilities from a separate registry, routes
  operations by strings, and converts between crate and legacy runtime data.
- `HostCapabilities` and parallel vocabulary lists have not been replaced by
  one strict nested `RuntimeSpec` with opaque operation IDs.
- The crate has no host-independent `SessionClient` suitable for the engine to
  drive while borrowing short-lived ECS access.

## Resume MMS work here for 0.9

- [ ] Refresh the public API inventory and compile/behavior fixtures.
- [x] Record the release decision: the full MMS cutover targets 0.9 and is not
      a 0.8 gate.
- [ ] Finish the supported crate-owned `RuntimeSpec`, strict component policy,
      and opaque binding IDs.
- [ ] Make `MittensHost` implement every declared effect without consulting a
      second vocabulary or the legacy evaluator.
- [ ] Introduce the host-independent persistent session/client boundary and
      make the generic runner drive it.
- [ ] Cut ordinary Mittens source/file/world evaluation over first and rerun
      executable examples.
- [ ] Migrate loaded modules and template/live factories while preserving
      session identity.
- [ ] Replace stored closures with opaque callback handles and migrate
      handlers, keyframes, animations, and audio lookahead.
- [ ] Move REPL and live-inspection integration onto the crate protocol.
- [ ] Delete engine-owned evaluator/runtime state only after all callers move.
- [ ] Pass the complete cutover matrix, version `meow-meow-script` from its
      actual published history, and publish `mittens-engine 0.9.0` only after
      the legacy runtime is gone.

## Independent 0.8 work

- [Humanoid bone map and AVC cleanup](humanoid-bone-map-automapping-and-mms-presets.md)
- [Editor grid and paint release gate](editor-grid-paint-0.8.0-release-gate.md)
- [Accordion restoration regression](../bugs/accordion-panel-restore-content-and-background-corruption.md)

## First smoke tests after resuming MMS work

```sh
cargo test -p meow-meow-script
cargo run -p meow-meow-script --example standard_runtime
cargo run -p meow-meow-script --example event_stream_host
cargo run -p meow-meow-script --example json_lines_host
cargo check -p mittens-engine --lib
```

After each engine-facing slice, also run the smallest Mittens example using
that path. A crate example passing does not establish parity for modules,
callbacks, REPL, or live-ECS behavior.

## Re-entry guardrails

- Do not add new behavior to `world_evaluator.rs` unless required to preserve
  a regression fixture during migration.
- Do not treat `Runner::standard()` as proof that Mittens uses the crate
  runner.
- Do not retain a compatibility facade if it requires a second heap,
  evaluator, closure representation, or mutable module state.
- Keep attachment graph materialization in the crate `StandardHost`; it is a
  standalone host result, not an engine scene-tree implementation.

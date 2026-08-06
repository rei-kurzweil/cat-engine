# Mittens/MMS cutover resume checkpoint

Date: 2026-08-06

Status: resumed and reframed; catalog foundation targets `0.7.2`, legacy
deletion targets `0.8.0`

## Purpose

Record the current state of the Mittens/MMS ownership cutover after the
UI-performance and first XR-pose implementation slices landed. This is the
short re-entry document; the release sequence is
[Mittens 0.7.1, 0.7.2, and 0.8.0](release-roadmap-0.7.1-0.7.2-0.8.0.md), the
canonical cutover plan remains
[Mittens MMS ownership cutover](mittens-mms-ownership-cutover-and-0.8-release.md),
and the detailed checklist remains
[MMS evaluator deduplication](mms-evaluator-deduplication.md).

## Current conclusion

The boundary change is **not finished**. The first standalone
`meow-meow-script` slices work, but Mittens still owns and calls the legacy MMS
evaluator, heap, closure, module, callback, and REPL paths. The remaining work
is therefore not merely internal cleanup inside `meow-meow-script`.

Do not describe the ownership cutover as complete from this checkpoint.
`mittens-engine 0.7.1` may ship its smaller avatar/editor reliability scope;
`0.7.2` may ship the supported catalog foundation; `mittens-engine 0.8.0`
remains gated on deletion of the duplicated engine-side MMS runtime.

## What works now

- `meow-meow-script` owns its syntax pipeline and public host protocol types.
- A preliminary runtime/catalog builder and `Session<H>` exist in the crate.
- `StandardHost` can collect emitted and registered roots and resolve local
  attachment relationships into a queryable forest.
- `Runner::standard()` provides a convenient standalone path.
- The crate examples exercise a standard host, an event-stream host, and a
  JSON-lines host.
- Mittens contains a provisional `MittensHost` adapter and re-exports parts of
  the crate syntax surface.
- MMS user guides are being centralized under
  `crates/meow-meow-script/docs/`; historical tasks and analysis remain in the
  repository documentation tree.
- Editor panel accordion/minimization and subtree suspension are implemented.
- The controller Grip/Aim and joint-basis retargeting direction has landed;
  headset acceptance and the shared humanoid-map ergonomics remain open.

The last recorded smoke-test evidence was:

- all 44 `meow-meow-script` library tests passed;
- `standard_runtime`, `event_stream_host`, and `json_lines_host` ran; and
- `cargo check -p mittens-engine --lib` passed with pre-existing warnings.

This evidence proves the standalone slice, not the engine cutover.

## What still crosses the wrong boundary

- `src/scripting/world_evaluator.rs` remains the engine evaluator.
- `src/scripting/runner.rs` still calls the legacy evaluator and owns
  engine-local loaded-module state.
- engine-local `Value`, `MaterializedCE`, `RuntimeClosure`, heap, and related
  object types remain in use.
- modules, factories, handlers, keyframes, animations, and audio callbacks
  still depend on engine-owned evaluation or closure state.
- the engine REPL still uses the legacy evaluator handle and worker protocol.
- `MittensHost` still derives capabilities from a separate registry, routes
  operations by strings, and converts between crate and legacy runtime data.
- `HostCapabilities` and parallel vocabulary lists have not been replaced by
  one strict, nested `RuntimeSpec` with opaque operation IDs.
- the crate has no host-independent `SessionClient` suitable for the engine to
  drive while borrowing short-lived ECS access.

## Resume MMS work here

Work in this order unless new evidence changes a dependency:

- [ ] Refresh the public API inventory and compile/behavior fixtures from
      Phase 0 of the canonical plan.
- [x] Record the revised compatibility/release decision: ordinary Mittens
      behavior is preserved, `0.7.2` introduces the supported catalog, and
      engine-side legacy deletion gates `0.8.0`.
- [ ] Finish the supported crate-owned `RuntimeSpec`, strict component policy,
      and opaque binding IDs for the `0.7.2` foundation.
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
- [ ] Delete engine-owned evaluator/runtime state only after all callers have
      moved.
- [ ] Delete duplicated engine runtime state, pass the cutover matrix, then
      bump `mittens-engine` to `0.8.0`; version `meow-meow-script` from its
      actual published history and API changes.

The active `0.7.1` work proceeds independently:

- [Shared humanoid bone map and AVC ergonomics](humanoid-bone-map-automapping-and-mms-presets.md)
- [Editor grid and paint release gate](editor-grid-paint-0.7.1-release-gate.md)

## First smoke tests after resuming

```sh
cargo test -p meow-meow-script
cargo run -p meow-meow-script --example standard_runtime
cargo run -p meow-meow-script --example event_stream_host
cargo run -p meow-meow-script --example json_lines_host
cargo check -p mittens-engine --lib
```

After each engine-facing slice, also run the smallest Mittens example that
uses that path. A crate example passing does not establish parity for module,
callback, REPL, or live-ECS behavior.

## Re-entry guardrails

- Do not add new behavior to `world_evaluator.rs` unless it is required to
  preserve a fixture during migration.
- Do not treat `Runner::standard()` as proof that Mittens uses the crate
  runner.
- Do not retain a compatibility facade if it requires a second heap,
  evaluator, closure representation, or mutable module state.
- Keep attachment graph materialization in the crate `StandardHost`; it is a
  standalone host result, not an engine scene-tree implementation.

## Related documents

- [Mittens MMS ownership cutover](mittens-mms-ownership-cutover-and-0.8-release.md)
- [MMS evaluator deduplication](mms-evaluator-deduplication.md)
- [Mittens host and MMS runtime boundary](../meow_meow/spec/mittens-host-and-runtime-boundary.md)
- [Move MMS documentation into the crate](move-meow-meow-documentation-into-crate.md)

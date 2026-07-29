# Deduplicate MMS evaluation into `meow-meow-script`

Date: 2026-07-29

Status: planning

## Goal

Make `crates/meow-meow-script` the only implementation of MMS language and
runtime semantics.

Mittens should embed that evaluator through `MittensHost`; it should not carry
a second interpreter in `src/scripting/world_evaluator.rs`.

The completed architecture should have:

- one evaluator
- one runtime `Value` representation
- one `ObjectWorld` / heap / closure model
- one implementation of control flow, indexing, field access, builtins,
  modules, and component-expression evaluation
- engine-specific behavior behind the host boundary

## Motivation

The workspace split already moved the tokenizer, parser, AST, transforms,
runtime values, pure evaluator, and public host contract into
`crates/meow-meow-script`.

The engine still runs most examples and live scripts through a separate
interpreter:

- `src/scripting/world_evaluator.rs`
- `src/scripting/object.rs`
- the correlated `EvalRequest` / `EvalResponse` protocol serviced by
  `src/scripting/runner.rs`

`src/scripting/host.rs` already defines `MittensHost`, but the main
engine-facing `MeowMeowRunner` does not yet use the standalone evaluator with
that host.

This leaves two implementations of core language behavior.

The terrain palette config exposed a concrete parity failure:

- the standalone evaluator supported table indexing
- a missing table key evaluated to `null`
- the engine-local evaluator only accepted numeric array indexes
- partial config tables therefore behaved differently depending on which
  evaluator ran the script

That class of bug will recur while evaluator semantics can land in either
location.

## Ownership decision

### `crates/meow-meow-script` owns

- tokenization and parsing
- AST and lowering/transforms
- expression and statement evaluation
- lexical scopes, frames, heap objects, closures, and captured state
- runtime `Value`, `MaterializedCE`, and related script DTOs
- functions, loops, branching, reassignment, tables, arrays, and indexing
- builtin dispatch and argument validation
- module evaluation, exports, and import semantics
- template versus live component-expression evaluation semantics
- evaluator sessions and any transport-neutral worker protocol
- typed evaluation and host errors
- host capability negotiation

### `mittens-engine` owns

- `MittensHost`
- conversion between opaque `ComponentHandle` and ECS `ComponentId`
- ECS queries
- component construction and materialization into `World`
- component-method dispatch
- attachment and initialization of live component trees
- signal-handler registration in `RxWorld`
- engine mutations, audio operations, and render-asset access
- construction of the engine's component/API catalog
- engine-facing convenience runners and example launchers

The engine may decide whether evaluation happens synchronously or on a worker
thread. It must not implement MMS expression semantics in order to do so.

## Current architecture

```text
MMS source
  │
  ▼
mittens_engine::scripting::MeowMeowRunner
  │
  ▼
src/scripting/world_evaluator.rs
  ├── parses and evaluates MMS
  ├── owns a second ObjectWorld / Value path
  ├── evaluates modules and runtime closures
  └── sends engine-specific HostCallKind messages
          │
          ▼
src/scripting/runner.rs / repl/backend.rs
          │
          ▼
component_registry + ECS World
```

The intended host-neutral path exists beside it:

```text
MMS source
  │
  ▼
crates/meow-meow-script::Evaluator
  │
  ▼
meow_meow_script::Host
  │
  ▼
mittens_engine::scripting::MittensHost
  │
  ▼
component_registry + ECS World
```

The task is to make the second path complete, migrate all callers, and delete
the first evaluator.

## Important boundary rules

### The host contract stays synchronous and host-neutral

`meow-meow-script::Host` is the canonical evaluator boundary.

No engine types should enter the language crate:

- no `World`
- no `ComponentId`
- no `IntentValue`
- no `RxWorld`
- no render-asset handles

The crate exposes opaque handles and transport-safe requests/responses.
`MittensHost` translates those into engine operations.

### The engine component registry is not an evaluator

`src/scripting/component_registry.rs` may:

- build an engine component from a validated component DTO
- apply constructors, properties, and child topology
- supply component/API schema when constructing the MMS runtime catalog

It should not independently evaluate MMS expressions. Transitional helpers
such as `expression_to_value(...)` must either disappear or consume values
already evaluated by `meow-meow-script`.

### Missing host features produce typed errors

If the standalone evaluator requests an engine operation that `MittensHost`
does not yet implement, evaluation should return a typed unsupported-host or
host-failure error.

Do not silently fall back to the old evaluator.

### No permanent dual-run production path

Running both evaluators is useful in parity tests during migration. Production
code should switch at explicit milestones and should never select an evaluator
based on individual language features.

## Migration inventory

Before switching the runner, account for every responsibility currently in
`world_evaluator.rs`.

### Ordinary script execution

- worldless `eval(...)`
- live `eval_with_world(...)`
- relative source paths and imports
- top-level component emission
- live registration of `let component = T { ... }`
- attachment and initialization of registered subtrees
- queries and component method calls

### Modules and factories

- `load_module_source(...)`
- `load_module_file(...)`
- named and sequence exports
- exported function calls
- template `MaterializedCE` factories
- live factory instantiation
- uninitialized live subtree spawning
- shared module heap/object identity across exported calls

Template versus live mode must remain explicit. This migration must not
collapse the distinction documented in:

- `docs/task/mms-module-component-materialization-vs-instantiation.md`
- `docs/task/live-mms-module-preview-components-vs-panel-materialization.md`

### Long-lived runtime closures

- signal handlers registered by `on(...)`
- named/global handlers
- keyframe runtime closures
- animation lookahead evaluation
- captured tables and component handles
- shared heap lifetime after initial script evaluation finishes

These are a critical gate. The engine currently stores legacy
`RuntimeClosure` values in ECS components and later calls
`world_evaluator::eval_runtime_closure(...)`.

The replacement must store or reference crate-owned closure/session state and
resume it through the crate evaluator.

### REPL and sessions

- persistent bindings and heap
- snippet evaluation
- navigation evaluation
- `cwd`
- query-backed `ls`, `cd`, `tree`, `cat`, and dump behavior
- reset and shutdown

The REPL backend should consume a crate-owned session/worker API. It should not
depend on engine-local `EvalRequest`, `EvalResponse`, `HostCallKind`, or
`HostValue` definitions.

### Builtins and engine APIs

Inventory all behavior currently dispatched directly by
`world_evaluator.rs`, including:

- `Math`
- `MusicNote`
- queries
- component methods
- handler registration
- audio clip instances
- engine mutations
- REPL operations

Pure builtins belong in the crate. Engine operations belong in the public host
request catalog and `MittensHost`.

## Required host and crate work

### Complete `MittensHost`

`MittensHost` currently covers much of the engine boundary, but it is not yet a
drop-in replacement for the live runner.

Audit and implement:

- component catalog/capabilities
- engine API catalog registration
- `CallApi` dispatch instead of the current blanket unsupported result
- module source loading or an explicit source-loader interface
- scoped and global handler registration
- closure invocation after initial evaluation
- audio and mutation parity
- render-asset-aware component spawning
- query result and error parity
- REPL inspection requests, if these remain host operations

### Complete crate sessions

The crate evaluator must support a long-lived session that preserves:

- frames/bindings
- object heap
- registered callbacks
- module state
- opaque live component handles

The session API must be usable for:

- initial scene evaluation
- REPL snippets
- event handlers
- animation/keyframe callbacks
- exported module function calls

Do not recreate an evaluator or heap per callback when authored state is
expected to be shared.

### Complete worker integration

`crates/meow-meow-script/src/worker.rs` currently defines transport-neutral
request/response shapes but is not a full replacement for the engine-local
correlated worker.

Either:

1. finish the crate worker/session protocol and adapt the engine runner to it,
   or
2. keep threading in the engine while invoking one crate-owned `Evaluator`
   session on that thread.

In both cases, evaluation logic remains in the crate.

## Migration phases

### Phase 0 — freeze and establish parity

- [ ] Treat `src/scripting/world_evaluator.rs` as migration-only.
- [ ] Add a comment directing new language semantics to
      `crates/meow-meow-script`.
- [ ] Inventory all public and crate-private callers of:
  - `MeowMeowEvaluator`
  - `eval_mms_fn`
  - `eval_runtime_closure`
  - `eval_module_source`
  - engine-local `Value`, `MaterializedCE`, and `RuntimeClosure`
- [ ] Build a shared script corpus covering pure, live, module, callback, and
      REPL behavior.
- [ ] Run the pure subset through both evaluators and compare values and typed
      errors.
- [ ] Run the live subset through the old runner and the standalone evaluator
      with `MittensHost`, comparing world topology and emitted intents.

Exit criterion: known semantic differences are listed and protected by tests.

### Phase 1 — make `MittensHost` feature-complete

- [ ] Generate/register the engine component and API catalog without copying
      language rules into the engine.
- [ ] Implement every required host request.
- [ ] Remove legacy DTO conversion where the registry can consume crate-owned
      DTOs directly.
- [ ] Add focused host tests for spawn, register, attach, query, methods,
      handlers, audio, mutations, and failures.

Exit criterion: the parity corpus can run through the standalone evaluator and
`MittensHost` without unsupported operations.

### Phase 2 — migrate ordinary runner APIs

- [ ] Preserve the public engine-facing `MeowMeowRunner` convenience API where
      useful.
- [ ] Reimplement `eval`, `eval_with_path`, `eval_with_world`, and
      `eval_with_world_and_assets_at_path` on the crate evaluator.
- [ ] Preserve relative import behavior and error source locations.
- [ ] Switch examples and documentation tests to the new path.
- [ ] Remove ordinary script evaluation from `world_evaluator.rs`.

Exit criterion: all executable MMS examples evaluate through
`meow-meow-script::Evaluator`.

### Phase 3 — migrate modules and factories

- [ ] Move module loading/export state to crate-owned module/session types.
- [ ] Migrate generic exported function calls.
- [ ] Migrate template materialization.
- [ ] Migrate live and uninitialized factory instantiation.
- [ ] Preserve shared table/heap identity across exported calls.
- [ ] Update asset, panel, pose, paint, and world-panel callers.

Exit criterion: no module helper calls `eval_module_source` or `eval_mms_fn`
from `world_evaluator.rs`.

### Phase 4 — migrate callbacks and stored closures

- [ ] Replace engine-local `RuntimeClosure` storage with a crate-owned callback
      handle or closure/session value.
- [ ] Migrate signal handlers.
- [ ] Migrate keyframe and animation runtime evaluation.
- [ ] Preserve captured component handles and shared table identity.
- [ ] Verify handler and animation execution after the originating script call
      has returned.

Exit criterion: no engine system calls `eval_runtime_closure` or
`eval_mms_fn`.

### Phase 5 — migrate the REPL and worker protocol

- [ ] Move persistent evaluator state to the crate session.
- [ ] Replace engine-local evaluator request/response types.
- [ ] Preserve navigation commands and `cwd`.
- [ ] Keep ECS-specific formatting/query traversal in the host or REPL adapter,
      not the evaluator.
- [ ] Verify shutdown, reset, timeout, and error behavior.

Exit criterion: `src/scripting/repl` no longer imports
`src/scripting/world_evaluator`.

### Phase 6 — delete duplicate runtime code

- [ ] Delete `src/scripting/world_evaluator.rs`.
- [ ] Delete the engine-local evaluator thread protocol.
- [ ] Remove engine-local copies of `Value`, `ObjectWorld`, `MaterializedCE`,
      and runtime closure state where crate-owned types can be used directly.
- [ ] Remove legacy/external DTO conversion from `MittensHost`.
- [ ] Remove evaluator-like expression handling from
      `component_registry.rs`.
- [ ] Update stale documentation links that name `world_evaluator.rs` as the
      source of MMS semantics.

Exit criterion: searching outside `crates/meow-meow-script` finds no MMS
expression/statement evaluator implementation.

## Parity test matrix

The migration is not complete until these categories run through the canonical
crate evaluator.

| Category | Required coverage |
|---|---|
| Pure language | arithmetic, comparisons, dimensions, arrays, tables, missing optional keys, functions, closures, loops, reassignment, pipes |
| Errors | tokenize, parse, invalid types, unknown methods, unsupported host operations, source locations |
| Components | constructors, builders, properties, positionals, children, named components |
| Live identity | register, attach, queries, component methods, stale handles |
| Modules | relative imports, named exports, sequence exports, shared heap, exported calls |
| Factory modes | template materialization, live spawn, uninitialized spawn |
| Events | scoped handlers, named/global handlers, event payloads, captured state |
| Runtime callbacks | keyframes, animations, audio lookahead, post-load invocation |
| REPL | persistent variables, heap identity, navigation, reset, inspection |
| Engine integration | render assets, audio, mutations, editor panels, asset previews |

Add a regression fixture for every evaluator-parity bug found during the
migration. The partial terrain config/table-index case should be the first.

## Acceptance criteria

- [ ] `crates/meow-meow-script` contains the only MMS evaluator.
- [ ] `src/scripting/world_evaluator.rs` no longer exists.
- [ ] The engine runner evaluates through the crate evaluator and
      `MittensHost`.
- [ ] All MMS examples use the canonical path.
- [ ] Engine event handlers and animation callbacks resume crate-owned session
      state.
- [ ] Module template/live behavior remains explicit and tested.
- [ ] There is one public runtime `Value` and `MaterializedCE` representation.
- [ ] No engine registry/helper evaluates arbitrary MMS expressions.
- [ ] Pure-script compatibility tests compare hostless and Mittens-hosted
      execution.
- [ ] Live parity tests compare component topology, values, intents, and typed
      errors.
- [ ] Full workspace tests pass after the legacy evaluator is removed.

## Non-goals

- Redesigning MMS syntax.
- Replacing the ECS component registry.
- Moving engine components or systems into `meow-meow-script`.
- Exposing `World` through the host-neutral API.
- Preserving the legacy correlated ring-buffer protocol as public API.
- Adding a production fallback to the duplicate evaluator.
- Solving unrelated editor or renderer migration work.

## Risks

### Callback lifetime regressions

The highest risk is losing shared heap/session identity when handlers or
keyframes run later. Migrate these only with explicit aliasing and captured
state tests.

### Template/live factory regressions

Panel materialization and animated asset previews require different modes.
Keep the choice explicit at the caller.

### Catalog drift replacing evaluator drift

Moving evaluation into the crate is not enough if component/API schemas are
manually duplicated. Build the runtime catalog from one engine registry
description.

### Error behavior changes

The standalone evaluator uses typed host errors while the legacy runner often
collects strings. Preserve useful source context while moving engine APIs
toward typed errors.

### Large-bang migration

Do not delete the old evaluator before modules, callbacks, and REPL sessions
are proven. Migrate by API family with parity gates, but do not add new
language features to both implementations during that period.

## Related

- `docs/meow_meow/standalone-roadmap.md`
- `docs/meow_meow/spec/host-call-api.md`
- `docs/meow_meow/spec/script-runner.md`
- `docs/meow_meow/spec/eval-with-world.md`
- `docs/meow_meow/task/mms-objectworld-evaluator-wiring.md`
- `docs/task/mms-tables-as-heap-objects-only.md`
- `docs/task/mms-repl-navigation-and-cat-unification.md`
- `docs/task/mms-module-component-materialization-vs-instantiation.md`
- `docs/task/live-mms-module-preview-components-vs-panel-materialization.md`

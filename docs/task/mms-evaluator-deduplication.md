# MMS evaluator deduplication checklist

Date: 2026-07-29

Status: planned

Normative architecture:
[Mittens host and MMS runtime boundary](../meow_meow/spec/mittens-host-and-runtime-boundary.md).

This document tracks implementation only. Ownership, protocol invariants,
callback lifetime, catalog rules, and completion semantics are defined by the
normative specification and must not be redefined here.

## Scope guardrails

- [ ] Keep `src/scripting/world_evaluator.rs` migration-only; add no language
      semantics to it.
- [ ] Do not add a production fallback or feature-by-feature evaluator
      selection.
- [ ] Keep MMS syntax, ECS, renderer, and editor redesigns out of scope.
- [ ] Preserve current engine-facing `MeowMeowRunner` entry points where
      practical.
- [ ] Use temporary legacy-path re-exports only for crate-owned DTOs.
- [ ] Record every newly found parity defect as a regression fixture.

## Baseline and inventory

- [ ] Record the current workspace test baseline, including the 37 passing
      `meow-meow-script` library tests.
- [ ] Inventory every public and crate-private caller of:
  - `MeowMeowEvaluator`
  - `eval_mms_fn`
  - `eval_runtime_closure`
  - `eval_module_source`
  - engine-local `Value`, `MaterializedCE`, and `RuntimeClosure`
  - engine-local evaluator request/response and host-call types
- [ ] Inventory all engine component/API vocabulary sources:
  - canonical component names and aliases
  - parser-supported component names
  - constructors, builders, named properties, and positionals
  - component methods and signatures
  - signals
  - global and namespaced APIs
  - capability lists and dispatch matches
- [ ] Inventory all live host behavior:
  - spawn, register, attach, and initialization
  - queries and component methods
  - handler registration
  - audio and engine mutations
  - render-asset-dependent construction
  - source loading and relative imports
  - REPL navigation, inspection, clear, reset, and shutdown

## Phase 0 — pure parity and freeze

- [ ] Add a freeze comment to `src/scripting/world_evaluator.rs` directing new
      semantics to `crates/meow-meow-script`.
- [ ] Create one shared corpus runnable by the legacy and crate evaluators.
- [ ] Cover pure language behavior:
  - arithmetic, comparisons, dimensions, arrays, and tables
  - missing-key lookup returning `null`
  - functions, captures, reassignment, loops, branching, and pipes
  - component expressions without live host operations
- [ ] Compare successful values and typed failures for tokenization, parsing,
      invalid types, missing names, and source locations.
- [ ] Add the partial terrain-config/table-index regression first.
- [ ] Document intentional differences; fix or gate every accidental
      difference.
- [ ] Keep all existing crate library tests passing.

Exit gate: pure language results and typed errors agree, and known differences
are explicit regression tests.

## Phase 1 — authoritative catalog and complete host

- [ ] Introduce one Mittens registration type for component vocabulary,
      signatures, and concrete construction/method dispatch.
- [ ] Introduce one Mittens registration type for global/namespaced API
      signatures and concrete dispatch.
- [ ] Generate the `meow_meow_script::Runtime` specifications from those
      registrations.
- [ ] Generate `MittensHost` capability advertisement and dispatch tables from
      the same registrations.
- [ ] Add crate validation for duplicate or inconsistent names, aliases, and
      signatures.
- [ ] Add generated consistency tests proving every registration is:
  - parseable
  - advertised
  - dispatchable
  - backed by no orphan registry branch
- [ ] Migrate and remove independent vocabulary sources, including
      `SUPPORTED_COMPONENT_NAMES`, parser-only name lists, method-support
      matches, and manually maintained capability lists.
- [ ] Implement catalog-backed `CallApi`; remove the blanket unsupported
      response.
- [ ] Implement real REPL host responses or typed unsupported errors; remove
      no-op responses.
- [ ] Add source loading as a host request with importer identity, import
      specifier, resolved identity, and source text.
- [ ] Make the registry consume crate-owned evaluated values/component trees.
- [ ] Restrict `component_registry::expression_to_value` to serialization of
      already-ground AST, or replace it with crate materialization.
- [ ] Validate session ownership and live ECS generation for every component
      handle operation.
- [ ] Distinguish foreign, stale, unsupported, invalid, conversion, source, and
      host-failure errors.
- [ ] Add host integration tests for:
  - spawn, register, attach, and initialization
  - query results and component methods
  - foreign and stale handles
  - handlers and signals
  - audio and engine mutations
  - render-asset-dependent construction
  - errors without deadlock

Exit gate: the live parity corpus reaches no unimplemented operation through
the crate evaluator and `MittensHost`.

## Phase 2 — ordinary runners

- [ ] Extend the crate worker protocol with operation and host-call
      correlation, source identity, completion, typed errors, and shutdown.
- [ ] Replace permanently host-owned session state with a persistent,
      host-independent crate session.
- [ ] Service each operation with a short-lived main-thread `MittensHost`.
- [ ] Reimplement these engine-facing entry points on the crate worker:
  - `eval`
  - `eval_with_timeout`
  - `eval_with_path`
  - `eval_file`
  - `eval_file_with_timeout`
  - `eval_with_world`
  - `eval_with_world_at_path`
  - `eval_with_world_and_assets`
  - `eval_with_world_and_assets_at_path`
- [ ] Preserve relative imports through the source-load host capability.
- [ ] Preserve useful source paths and locations in typed errors.
- [ ] Preserve hostless versus live evaluation behavior without invoking the
      legacy evaluator.
- [ ] Switch executable examples and documentation tests to the canonical
      path.
- [ ] Remove ordinary script evaluation branches from
      `world_evaluator.rs`.
- [ ] Add live parity assertions for world topology, values, emitted intents,
      and failures.

Exit gate: every ordinary runner and executable MMS example evaluates through
the crate worker/session.

## Phase 3 — modules and factories

- [ ] Extend worker operations for module loading, named export calls, and
      sequence export calls.
- [ ] Move evaluated modules, exports, and their heap state into the persistent
      crate session.
- [ ] Migrate generic exported function calls.
- [ ] Preserve explicit template and live factory modes.
- [ ] Migrate template component-tree materialization.
- [ ] Migrate live factory spawning and uninitialized spawning.
- [ ] Preserve shared heap/table identity across repeated export calls.
- [ ] Keep live factories on the same session and host protocol as ordinary
      live evaluation.
- [ ] Make callback-free template artifacts detachable snapshots.
- [ ] Make callback-bearing template artifacts retain a lease on their
      originating session.
- [ ] Update asset, panel, pose, paint, preview, and world-panel callers.
- [ ] Add template-versus-live tests, including callback-bearing templates.
- [ ] Add import tests for relative resolution, cache identity, named exports,
      and sequence exports.

Exit gate: no module or factory helper calls `eval_module_source` or
`eval_mms_fn` in `world_evaluator.rs`.

## Phase 4 — callbacks and keyframes

- [ ] Add crate-owned `(SessionHandle, CallbackHandle)` references and callback
      invocation worker operations.
- [ ] Keep closure bodies, captures, and heap objects inside the originating
      crate session.
- [ ] Replace stored engine `RuntimeClosure` and raw function `Value`s with
      opaque callback references.
- [ ] Migrate scoped, named, and global signal handlers.
- [ ] Migrate keyframe callbacks, animation evaluation, and audio lookahead.
- [ ] Queue callbacks raised during host dispatch; do not synchronously
      re-enter the same session.
- [ ] Define session lease/release behavior for ECS handlers, keyframes,
      modules, and callback-bearing templates.
- [ ] Add lifetime tests proving delayed invocations retain:
  - captured component handles
  - shared table/array identity
  - mutable captured state
  - module heap identity after initial evaluation returns
- [ ] Test typed failure for a callback invoked after its session is released
      or reset.

Exit gate: no engine system calls `eval_runtime_closure` or `eval_mms_fn`, and
no ECS component stores an MMS closure body or runtime function value.

## Phase 5 — REPL and worker completion

- [ ] Extend worker operations for REPL snippets, navigation, inspection,
      reset, and orderly shutdown.
- [ ] Preserve bindings, heap identity, loaded modules, and current
      source/navigation context across snippets.
- [ ] Keep ECS traversal and engine-specific formatting in `MittensHost` or a
      REPL adapter.
- [ ] Replace engine-local `EvalRequest`, `EvalResponse`, `HostCallKind`, and
      `HostValue` use in the REPL/backend.
- [ ] Replace spin/yield polling with an efficient queue/channel wake-up.
- [ ] Add worker tests for:
  - multiple correlated host calls in one operation
  - stale, duplicate, and mismatched replies
  - callback invocation
  - persistent REPL state
  - reset invalidation
  - timeout and recoverable-error continuation
  - orderly shutdown and join

Exit gate: `src/scripting/repl` uses only the crate worker/session protocol and
host/REPL adapters.

## Phase 6 — legacy deletion

- [ ] Delete `src/scripting/world_evaluator.rs`.
- [ ] Delete the engine-local evaluator thread and ring-buffer protocol.
- [ ] Delete engine-local MMS `Value`, `ObjectWorld`, `MaterializedCE`, and
      closure state after migrating or re-exporting all callers.
- [ ] Remove legacy/external DTO conversion from `MittensHost`.
- [ ] Remove alternate MMS expression evaluation from
      `component_registry.rs`.
- [ ] Remove dead capability, vocabulary, and dispatch lists superseded by the
      catalog.
- [ ] Update all documentation that names the legacy evaluator or protocol as
      current architecture.
- [ ] Search outside `crates/meow-meow-script` for remaining MMS
      expression/statement evaluator helpers and remove them.
- [ ] Run formatting, documentation checks, crate tests, integration tests,
      examples, and the full workspace test suite.

Exit gate: the legacy evaluator and runtime object model no longer exist and
the full workspace suite passes.

## Final acceptance

- [ ] `meow-meow-script` is the sole owner of parsing, evaluation, runtime
      values, heap/session state, modules, and callbacks.
- [ ] Mittens owns only its runtime catalog, ECS integration, and main-thread
      host operations.
- [ ] One registration source makes every engine component/API parseable,
      advertised, and dispatchable.
- [ ] Every component operation validates session ownership and ECS
      generation.
- [ ] No engine helper evaluates arbitrary MMS expressions.
- [ ] All runner, module, callback, keyframe, REPL, and example paths use the
      crate worker/session.
- [ ] Template/live factory behavior is explicit and tested.
- [ ] Delayed callbacks retain their originating heap/session identity.
- [ ] Typed errors and recovery behavior are covered.
- [ ] The full workspace test suite passes.

## Required test matrix

| Suite | Required coverage |
|---|---|
| Pure parity | values, errors, tables, closures, control flow, component expressions, missing keys |
| Catalog consistency | every registration parseable, advertised, dispatchable; no orphan branches |
| Host integration | spawn/register/attach, queries, methods, handles, assets, handlers, audio, mutations |
| Modules | imports, export forms, repeated calls, shared heap |
| Factory modes | template, live, uninitialized live, callback-bearing template |
| Lifetime | handlers, keyframes, module exports, shared captured identity |
| Worker | correlation, callback invocation, reset, timeout/error recovery, shutdown |
| REPL | persistent values, navigation, inspection, reset |
| Workspace | all examples and tests on the canonical evaluator path |

## Related documents

- [Host API](../meow_meow/spec/host-call-api.md)
- [MeowMeowRunner](../meow_meow/spec/script-runner.md)
- [`eval_with_world`](../meow_meow/spec/eval-with-world.md)
- [Module component materialization versus instantiation](mms-module-component-materialization-vs-instantiation.md)
- [Live module previews versus panel materialization](live-mms-module-preview-components-vs-panel-materialization.md)

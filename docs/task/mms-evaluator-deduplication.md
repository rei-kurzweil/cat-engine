# MMS evaluator deduplication checklist

Date: 2026-07-29

Status: planned

Normative architecture:
[Mittens host and MMS runtime boundary](../meow_meow/spec/mittens-host-and-runtime-boundary.md).

This document tracks implementation only. Ownership, protocol invariants,
callback lifetime, `RuntimeSpec` builder rules, and completion semantics are
defined by the normative specification and must not be redefined here.

## Scope guardrails

- [ ] Keep `src/scripting/world_evaluator.rs` migration-only; add no language
      semantics to it.
- [ ] Do not add a production fallback or feature-by-feature evaluator
      selection.
- [ ] Keep MMS syntax, ECS, renderer, and editor redesigns out of scope.
- [ ] Preserve current engine-facing `MeowMeowRunner` entry points where
      practical.
- [ ] Use temporary legacy-path re-exports only for crate-owned DTOs.
- [ ] Do not describe the Mittens migration as non-breaking until a public
      Rust API and observable-behavior compatibility audit passes.
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
- [ ] Inventory the published/supported Mittens Rust surface, especially:
  - `MeowMeowRunner` method signatures
  - `LoadedMmsModule` and its public fields
  - legacy scripting-module type paths
  - callback/keyframe types exposed through ECS components
- [ ] Classify every compatibility hazard listed in the normative spec as:
  - preserved exactly
  - preserved through a deprecated boundary-safe facade
  - explicitly unstable/internal
  - deliberately breaking
- [ ] Add compile fixtures representing existing external Mittens runner and
      module consumers.
- [ ] Inventory every independent engine component/API vocabulary source that
      must be folded into the one `RuntimeSpec` builder:
  - canonical component names and aliases
  - parser-supported component names
  - constructors, builders, named properties, and positionals
  - component methods and signatures
  - signals and payload types
  - pure and host-dispatched builtins
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

## Phase 1 — one `RuntimeSpec` builder and complete host

- [ ] Add crate-owned `RuntimeSpec` and nested builder types.
- [ ] Make `with_standard_builtins()` seed crate-provided pure builtins and
      value types into that same builder; keep grammar unconditionally
      crate-owned.
- [ ] Support nested component declarations for:
  - canonical names and aliases
  - constructors and component-expression builder calls
  - named and positional properties
  - component methods and signatures
  - signals and payload fields
- [ ] Support nested global/namespaced declarations for pure builtins,
      host-dispatched builtins, and engine APIs.
- [ ] Let the same builder calls attach concrete Mittens construction,
      property, method, signal, builtin, and API implementations.
- [ ] Make `build()` produce one crate-owned `RuntimeSpec` for
      `meow_meow_script::Runtime` plus opaque implementation bindings for
      `MittensHost`.
- [ ] Ensure the implementation bindings contain no duplicate names,
      signatures, aliases, signal schemas, or parser metadata and therefore
      cannot act as a second specification.
- [ ] Remove `HostCapabilities` negotiation; host availability comes from the
      one specification, while missing per-call engine services produce typed
      unavailable-context errors.
- [ ] Make `build()` reject duplicate/inconsistent names, aliases, nesting, and
      signatures, missing implementations, and unreachable implementations.
- [ ] Add generated consistency tests proving every specification item is:
  - parseable
  - visible to validation and completion
  - bound to exactly one implementation when effectful
  - backed by no orphan implementation branch
- [ ] Migrate and remove independent vocabulary sources, including
      `SUPPORTED_COMPONENT_NAMES`, parser-only name lists, method-support
      matches, and manually maintained capability lists.
- [ ] Make component method and `CallApi` requests use opaque operation IDs
      assigned by `RuntimeSpec::build()`; remove string-based support matches
      and the blanket unsupported response.
- [ ] Implement real REPL host responses or typed unsupported errors; remove
      no-op responses.
- [ ] Add source loading as a host request with importer identity, import
      specifier, resolved identity, and source text.
- [ ] Make the registry consume crate-owned evaluated values/component trees.
- [ ] Restrict `component_registry::expression_to_value` to serialization of
      already-ground AST, or replace it with crate materialization.
- [ ] Validate session ownership and live ECS generation for every component
      handle operation.
- [ ] Distinguish foreign, stale, hostless-unsupported,
      unavailable-host-context, invalid, conversion, source, and host-failure
      errors.
- [ ] Add host integration tests for:
  - spawn, register, attach, and initialization
  - query results and component methods
  - foreign and stale handles
  - handlers and signals
  - audio and engine mutations
  - render-asset-dependent construction
  - errors without deadlock

Exit gate: Mittens gives MMS exactly one `RuntimeSpec`; every effectful item in
it has exactly one host implementation, and the live parity corpus reaches no
unimplemented operation.

## Phase 2 — ordinary runners

- [ ] Extend the crate worker protocol with operation and host-call
      correlation, source identity, completion, typed errors, and shutdown.
- [ ] Replace permanently host-owned session state with a persistent,
      host-independent crate session.
- [ ] Add a crate-owned `SessionClient` created from a configured runtime
      before runner/REPL construction.
- [ ] Add a host-generic crate `Runner` whose core constructor accepts
      `SessionClient`, not `RuntimeSpec` or a configuration builder.
- [ ] Define crate-owned run request, result, diagnostic, and output-event
      DTOs.
- [ ] Define the generic host-service callback/trait used to service
      `HostRequest` during a runner operation.
- [ ] Define component emit/register/attach command/reply DTOs.
- [ ] Provide collecting and rejecting component sink adapters.
- [ ] Provide blocking and polling runners over the same session protocol;
      decide whether the async adapter lands in this phase or immediately
      afterward.
- [ ] Service each operation with a short-lived main-thread `MittensHost`.
- [ ] Make `MeowMeowRunner` a Mittens compatibility wrapper over the generic
      crate runner.
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
- [ ] Add fake-host tests proving the crate runner has no Mittens dependency.
- [ ] Add tests for collecting emitted component trees without a live engine.

Exit gate: the crate runner works with arbitrary/fake hosts without a
configuration builder, and every Mittens runner and executable MMS example
delegates through it.

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
- [ ] Move REPL input classification and multiline completion into
      `meow-meow-script`.
- [ ] Add a programmatic crate `Repl` that accepts submitted input and emits
      structured `ReplEvent`s without requiring stdin/stdout.
- [ ] Construct the crate REPL from `Runner`/`SessionClient`, never from
      `RuntimeSpec` or its builder.
- [ ] Preserve bindings, heap identity, loaded modules, and current
      source/navigation context across snippets.
- [ ] Keep navigation over tables, arrays, and component artifacts inside the
      crate session using opaque `ValueRef`s.
- [ ] Add generic inspection request/response DTOs for world roots and live
      components.
- [ ] Keep ECS traversal, component liveness, short-ID/GUID resolution,
      subtree source rendering, and listing labels in `MittensHost` or a
      Mittens REPL adapter.
- [ ] Separate terminal I/O from REPL semantics:
  - optional standard terminal adapter in the crate
  - Mittens stdin ownership coordination in the engine adapter
  - no direct printing required by the core REPL
- [ ] Decide which of `tree`, `dump`, `help`, `clear`, and `reset` are
      REPL-only commands versus standard crate builtins.
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
- [ ] Add REPL tests for:
  - pure table/array navigation without a host
  - component-artifact navigation without a host
  - fake-host world/component inspection
  - unsupported live inspection with pure navigation still working
  - programmatic input/output without terminal access

Exit gate: the crate REPL works with arbitrary/fake hosts and no configuration
builder; `src/scripting/repl` contains only Mittens host, frame-loop, terminal,
and compatibility adapters.

## Phase 6 — legacy deletion

- [ ] Delete `src/scripting/world_evaluator.rs`.
- [ ] Delete the engine-local evaluator thread and ring-buffer protocol.
- [ ] Delete engine-local MMS `Value`, `ObjectWorld`, `MaterializedCE`, and
      closure state after migrating or re-exporting all callers.
- [ ] Remove legacy/external DTO conversion from `MittensHost`.
- [ ] Remove alternate MMS expression evaluation from
      `component_registry.rs`.
- [ ] Remove dead capability, vocabulary, signal, and dispatch lists
      superseded by the one builder expression.
- [ ] Update all documentation that names the legacy evaluator or protocol as
      current architecture.
- [ ] Search outside `crates/meow-meow-script` for remaining MMS
      expression/statement evaluator helpers and remove them.
- [ ] Run formatting, documentation checks, crate tests, integration tests,
      examples, and the full workspace test suite.

Exit gate: the legacy evaluator and runtime object model no longer exist and
the full workspace suite passes.

## Release and versioning

- [ ] Treat the direct `meow-meow-script` API migration as breaking.
- [ ] Bump `meow-meow-script` from `0.6.0` to `0.7.0` when the new API lands;
      do not publish it as `0.6.x`.
- [ ] Update the `mittens-engine` dependency requirement and lockfile to the
      new `meow-meow-script` version in the same release change.
- [ ] Publish a direct-embedder migration guide covering:
  - construction through the one `RuntimeSpec` builder
  - removal of separate `HostCapabilities`
  - the persistent host-independent session
  - constructing the generic runner/REPL from `SessionClient`
  - component sinks and optional REPL inspection
  - worker request/response correlation
  - callback and component handle changes
- [ ] Run the Mittens public API/source-compatibility fixtures.
- [ ] Verify observable compatibility for runner outputs, errors, modules,
      template/live factories, handlers, and keyframes.
- [ ] If supported Mittens APIs cannot be preserved with a boundary-safe
      facade, classify the engine change as breaking and bump
      `mittens-engine` from `0.7.0` to `0.8.0`.
- [ ] If those APIs and behaviors are preserved, record explicitly that the
      engine release is non-breaking even though its `meow-meow-script`
      dependency made a breaking release.

Exit gate: crate versions communicate the actual compatibility impact and
both direct embedders and Mittens users have an explicit migration story.

## Final acceptance

- [ ] `meow-meow-script` is the sole owner of parsing, evaluation, runtime
      values, heap/session state, modules, and callbacks.
- [ ] Mittens assembles exactly one crate-owned `RuntimeSpec`, plus ECS
      integration and main-thread implementations bound from the same builder
      calls.
- [ ] The nested builder covers every component, property, positional,
      constructor, method, builtin, signal type, and global/namespaced API.
- [ ] No second Mittens specification, capability schema, parser-name list, or
      method-support list exists.
- [ ] Every component operation validates session ownership and ECS
      generation.
- [ ] No engine helper evaluates arbitrary MMS expressions.
- [ ] All runner, module, callback, keyframe, REPL, and example paths use the
      crate worker/session.
- [ ] The crate owns a host-generic runner and programmatic REPL constructed
      from `SessionClient`, with no configuration-builder dependency.
- [ ] Component collecting/rejecting sinks and generic inspection work without
      Mittens.
- [ ] Template/live factory behavior is explicit and tested.
- [ ] Delayed callbacks retain their originating heap/session identity.
- [ ] Typed errors and recovery behavior are covered.
- [ ] `meow-meow-script` has a pre-1.0 breaking version bump and migration
      guide.
- [ ] Mittens public API compatibility is proven, or Mittens receives its own
      documented breaking version bump.
- [ ] The full workspace test suite passes.

## Required test matrix

| Suite | Required coverage |
|---|---|
| Pure parity | values, errors, tables, closures, control flow, component expressions, missing keys |
| Specification consistency | every item parseable and visible; every effectful item has one binding; no orphan bindings |
| Host integration | spawn/register/attach, queries, methods, handles, assets, handlers, audio, mutations |
| Modules | imports, export forms, repeated calls, shared heap |
| Factory modes | template, live, uninitialized live, callback-bearing template |
| Lifetime | handlers, keyframes, module exports, shared captured identity |
| Worker | correlation, callback invocation, reset, timeout/error recovery, shutdown |
| Generic runner | fake host, component collection/rejection, blocking/polling parity |
| REPL | persistent values, pure navigation, fake-host inspection, terminal-free I/O, reset |
| Workspace | all examples and tests on the canonical evaluator path |

## Related documents

- [Host API](../meow_meow/spec/host-call-api.md)
- [MeowMeowRunner](../meow_meow/spec/script-runner.md)
- [`eval_with_world`](../meow_meow/spec/eval-with-world.md)
- [Module component materialization versus instantiation](mms-module-component-materialization-vs-instantiation.md)
- [Live module previews versus panel materialization](live-mms-module-preview-components-vs-panel-materialization.md)
- [Generic runner and REPL boundary](../meow_meow/analysis/generic-runner-and-repl-boundary.md)

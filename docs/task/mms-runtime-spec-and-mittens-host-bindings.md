# MMS RuntimeSpec and MittensHost bindings

Date: 2026-08-13

Status: active focused implementation tracker

## Purpose

Complete the first two implementation slices of the Mittens/MMS ownership
cutover:

1. replace the preliminary flat runtime catalog with the supported nested
   `RuntimeSpec` builder and opaque implementation bindings; and
2. make `MittensHost` service that specification without consulting a second
   vocabulary, matching operation names as strings, or converting through the
   legacy evaluator/runtime model.

This is the focused Phase 1 gate for
[MMS evaluator deduplication](mms-evaluator-deduplication.md) and
[Mittens MMS ownership cutover for 0.9](mittens-mms-ownership-cutover-and-0.9-release.md).
The normative ownership and dispatch rules remain in
[Mittens host and MMS runtime boundary](../meow_meow/spec/mittens-host-and-runtime-boundary.md)
and [Host API](../meow_meow/spec/host-call-api.md). This tracker does not
redefine them.

## Outcome

One nested Mittens builder expression produces:

- one immutable crate-owned `RuntimeSpec`, compiled into the MMS `Runtime`;
  and
- one metadata-free implementation binding table indexed by opaque operation
  IDs and consumed by `MittensHost`.

At the exit gate, component names, aliases, constructors, properties,
positionals, methods, signals, builtins, and APIs are described once. A name
unknown to the specification fails before host dispatch, while a declared
operation whose short-lived host lacks a required engine service returns a
typed unavailable-context error.

## Out of scope

These remain downstream tasks and must not be pulled into this tracker unless
an interface fixture is required for the builder or host boundary:

- `SessionClient` and the persistent worker implementation;
- ordinary `MeowMeowRunner` cutover;
- module, factory, and source-cache migration;
- callback leases and keyframe/audio callback invocation migration;
- the programmatic REPL and live-navigation migration;
- deletion of `src/scripting/world_evaluator.rs`; and
- later numeric runtime types, receiver intrinsics, inference, and checking.

The builder and host protocol must be shaped so those tasks can consume them,
but this tracker does not implement those consumers.

Handler declarations and registration are still in scope: this task replaces
raw function values with an opaque callback reference and makes host dispatch
enqueue that reference without re-entering evaluation. Retaining the
originating session and executing the queued callback are downstream work.

## Current baseline

The preliminary crate slice already provides:

- `RuntimeBuilder`, `ComponentSpec`, `HostApiSpec`, `ValueType`, and
  `ValueSignature`;
- `ComponentNamePolicy::{OpenUppercase, StrictRegistered}`;
- `Runtime::standard()` and a collecting `StandardHost`;
- crate-owned host request/response DTOs and opaque component/callback handles;
  and
- a provisional `MittensHost` adapter.

The remaining boundary defects are:

- the builder is flat and builds `Runtime` directly rather than a public
  immutable `RuntimeSpec`;
- `HostCapabilities` remains a separately advertised schema;
- the builder cannot attach and validate engine implementations;
- component methods, APIs, mutations, audio, and signals still carry or match
  names rather than specification-assigned IDs;
- `SUPPORTED_COMPONENT_NAMES`, component-method matches, signal-name matches,
  and parser/registry knowledge remain parallel vocabulary sources;
- `MittensHost` converts crate values and component trees through engine-local
  `Value`, `MaterializedCE`, `RuntimeClosure`, and fresh legacy heaps; and
- handler dispatch still calls the engine-local evaluator.

Baseline verification on 2026-08-13:

- `cargo test -p meow-meow-script`: 44 passed; and
- `cargo check -p mittens-engine --lib`: passed with pre-existing warnings.

## Boundary decisions to lock

- [ ] Inventory the public flat builder API and decide which names receive a
      temporary deprecated facade versus a direct pre-1.0 replacement.
- [ ] Define the public build result containing the `RuntimeSpec` and opaque
      implementation bindings without exposing Mittens types from
      `meow-meow-script`.
- [ ] Define opaque ID domains for every effectful declaration that crosses the
      host boundary, including component construction/property application,
      component methods, signals, host builtins, namespaced APIs, audio, and
      engine mutations.
- [ ] Decide which pure constructors/properties are crate implementations and
      which are host operations; do not represent the same declaration in both
      domains.
- [ ] Define how binding functions receive short-lived host context without
      embedding `World`, `RxWorld`, render assets, or Mittens types in crate
      DTOs.
- [ ] Define the opaque callback-reference and enqueue seam needed for handler
      registration without implementing callback leases or invocation here.
- [ ] Define the transition rule: any temporary legacy lookup table must be
      generated from the built specification and must not be independently
      authored.
- [ ] Record typed build and dispatch errors before replacing the flat API.

Exit gate: the public shapes and migration strategy cannot require a second
catalog or permanently host-owned session.

## Workstream A — crate-owned RuntimeSpec

### A1. Immutable specification and identifiers

- [ ] Add public `RuntimeSpec` and `RuntimeSpecBuilder` types.
- [ ] Separate immutable specification data from mutable builder state.
- [ ] Make `Runtime` compile or wrap exactly one completed `RuntimeSpec`.
- [ ] Keep `RuntimeSpec` free of heap, host, session, and engine state.
- [ ] Add opaque, non-string operation identifiers with no public construction
      from arbitrary names.
- [ ] Preserve `Runtime::standard()` as a builder-free open-name convenience.
- [ ] Add `with_standard_builtins()` so crate-provided builtins and value types
      enter the same specification build.

### A2. Nested declaration builders

- [ ] Add nested component declarations owning:
  - canonical names and aliases;
  - component body mode, including `props_only`;
  - constructors and component-expression builder calls;
  - ordered positional fields and named properties;
  - component methods and signatures; and
  - signals and typed payload fields.
- [ ] Add nested global and namespaced declarations for pure builtins,
      host-dispatched builtins, and engine APIs.
- [ ] Let every effectful constructor, property, method, signal, builtin, and
      API declaration attach exactly one implementation binding in the same
      builder call.
- [ ] Keep intrinsic/pure implementation targets distinct from host-operation
      targets while sharing the same signature catalog.
- [ ] Preserve stable declaration order where it affects diagnostics,
      completion, reflection, or authored component fields.

### A3. Build validation

- [ ] Reject duplicate or case-conflicting canonical names and aliases.
- [ ] Reject invalid nesting, duplicate positionals/properties/methods/signals,
      and conflicting signatures.
- [ ] Reject unknown types referenced by signatures or signal fields.
- [ ] Reject missing implementations for effectful declarations.
- [ ] Reject implementation bindings unreachable from a declaration.
- [ ] Reject ambiguous operation dispatch and duplicate operation IDs.
- [ ] Ensure the returned binding table contains no names, aliases,
      signatures, signal schemas, parser metadata, or capability sets.
- [ ] Add deterministic diagnostics identifying the declaration path that
      failed, such as `Transform.method(set_position)`.

### A4. Runtime and protocol consumption

- [ ] Make parser/validation component-name lookup use only `RuntimeSpec`.
- [ ] Configure the standard runtime as `OpenUppercase` and the Mittens runtime
      as `StrictRegistered`.
- [ ] Resolve registered component body mode from `RuntimeSpec`; complete the
      focused `props_only` behavior task without an engine-local name map.
- [ ] Make component methods and host APIs resolve to opaque operation IDs
      before producing a host request.
- [ ] Carry opaque signal IDs in handler-registration requests.
- [ ] Replace name-bearing audio and mutation dispatch with declared operation
      IDs where those operations are MMS vocabulary.
- [ ] Ensure unknown names fail during validation and never reach the host.
- [ ] Keep universal component inspection and REPL inspection outside the
      registered vocabulary as required by the host API specification.

### A5. Flat API retirement

- [ ] Remove `HostCapabilities` from `Host` and session construction.
- [ ] Migrate crate examples and tests from the flat `RuntimeBuilder` API.
- [ ] Deprecate or remove `ComponentSpec`, `HostApiSpec`, and flat registration
      methods according to the compatibility decision.
- [ ] Document the direct-embedder migration from flat catalog registration to
      nested declarations and builder-bound implementations.

Exit gate: the crate exposes one validated specification and effectful calls
carry only IDs assigned by its build.

## Workstream B — one Mittens builder expression

### B1. Vocabulary inventory

- [ ] Inventory and classify every current source of Mittens MMS vocabulary:
  - `SUPPORTED_COMPONENT_NAMES` and component aliases;
  - parser-supported names and component body behavior;
  - component constructors and builder calls;
  - named and positional properties;
  - `component_method_registry` support and dispatch matches;
  - signal names and payload adapters;
  - global and namespaced APIs;
  - audio and engine mutation operation names; and
  - capability and documentation consistency lists.
- [ ] Add an inventory test that fails when an old source contains an entry not
      represented by the new builder.
- [ ] Classify universal host protocol operations separately from configured
      MMS vocabulary.

### B2. Assemble the strict Mittens specification

- [ ] Add one discoverable construction entrypoint for the complete Mittens
      build result.
- [ ] Start it with standard builtins and
      `ComponentNamePolicy::StrictRegistered`.
- [ ] Declare every supported component and alias through nested component
      builders.
- [ ] Attach construction, property, method, signal, builtin, and API engine
      implementations in those same declarations.
- [ ] Represent context requirements such as `World`, `RxWorld`, render
      assets, intent emission, and audio services as binding requirements, not
      capabilities or alternate catalogs.
- [ ] Make any transitional parser/registry/documentation views derive from
      the completed specification.
- [ ] Add a test proving the Mittens build has no missing or orphan bindings.

### B3. Retire parallel vocabulary

- [ ] Replace `SUPPORTED_COMPONENT_NAMES` consumers with `RuntimeSpec`
      iteration or a generated compatibility view.
- [ ] Remove parser-only component-name registration.
- [ ] Remove `supports_component_method` and other separately maintained
      method-support matches.
- [ ] Remove independently maintained signal-name dispatch lists once signal
      IDs and bindings cover them.
- [ ] Remove string-based API/audio/mutation support matches.
- [ ] Update documentation consistency tests to read the built specification.

Exit gate: adding or changing a script-visible Mittens operation requires one
builder edit, not coordinated edits to parallel lists and match arms.

## Workstream C — specification-bound MittensHost

### C1. Host construction and binding lookup

- [ ] Make `MittensHost` receive the opaque implementation bindings produced
      with the runtime specification.
- [ ] Dispatch effectful requests by opaque operation ID.
- [ ] Ensure an unknown ID is a typed protocol/invalid-request error rather
      than a string fallback.
- [ ] Remove `MittensHost::capabilities()` and catalog negotiation.
- [ ] Keep `MittensHost` short-lived and borrowing engine state only for one
      dispatch window.
- [ ] Prevent host dispatch from synchronously re-entering MMS evaluation.

### C2. Handle and context validation

- [ ] Preserve the complete generational ECS key in `ComponentHandle`.
- [ ] Validate requesting-session ownership before every component operation.
- [ ] Validate ECS liveness/generation after ownership.
- [ ] Return distinct `ForeignHandle` and `StaleHandle` errors.
- [ ] Replace public/raw handle conversion paths that bypass validation with
      checked conversion owned by the host context.
- [ ] Return `UnavailableHostContext` when a valid bound operation lacks
      `RxWorld`, render assets, audio, or another required service.
- [ ] Reserve `UnsupportedHostOperation` for hosts/runtime modes that truly do
      not implement an operation.

### C3. Host operation coverage

| Family | Current provisional path | Completion requirement |
|---|---|---|
| Emit/spawn | crate tree converted to legacy tree | consume the crate-owned evaluated tree through builder-bound construction |
| Register/attach/init | direct registry and `World` calls | validate ownership/liveness and use bound construction/property operations |
| Query | direct selector traversal | retain engine traversal with crate DTO results and checked handles |
| Component methods | component type and method strings | dispatch solely through the method operation ID |
| Engine APIs | blanket unsupported branch | dispatch every declared API binding or return unavailable context |
| Signals/handlers | signal strings and legacy function values | use signal IDs, register opaque callback references, and enqueue invocation without re-entering evaluation |
| Audio/mutations | operation strings routed to method registry | use declared operation IDs and typed arguments |
| Render-dependent construction | ambient optional assets | declare the binding requirement and return unavailable context when absent |
| REPL call-shaped operations | silent unit responses | declare and bind them or return a typed unsupported error; no no-op success |

- [ ] Implement every operation family declared by the Mittens specification.
- [ ] Remove blanket `CallApi` unsupported handling for registered APIs.
- [ ] Remove silent success for unsupported REPL requests.
- [ ] Ensure host failures preserve the operation ID, declaration identity for
      diagnostics, and underlying engine cause without reintroducing name
      dispatch.

### C4. Remove legacy DTO conversion from the host

- [ ] Make component construction consume crate-owned `MaterializedCE` and
      `Value` DTOs directly.
- [ ] Remove `external_tree_to_legacy`, `external_value_to_legacy`, and
      `legacy_value_to_external` from `MittensHost`.
- [ ] Do not allocate fresh legacy heaps while converting tables, closures, or
      modules.
- [ ] Ensure raw function/closure values never cross into or remain stored by
      Mittens; register and enqueue the callback boundary shape even though
      session leases and callback invocation are completed downstream.
- [ ] Restrict `component_registry::expression_to_value` to already-ground AST
      serialization or replace it with crate materialization.

Exit gate: `MittensHost` implements declared engine effects from opaque
bindings and crate DTOs without a legacy evaluator, heap, or vocabulary lookup.

## Verification

### Crate tests

- [ ] Nested builder happy-path coverage for every declaration kind.
- [ ] Duplicate, conflict, invalid nesting, missing binding, orphan binding,
      and unknown-type failures.
- [ ] Standard/open and strict/registered component-name behavior.
- [ ] Specification resolution from source name to opaque operation ID.
- [ ] Proof that binding tables expose no specification metadata.
- [ ] Custom-host construction without `HostCapabilities`.

### Mittens integration tests

- [ ] Generated consistency test: every declared component name and alias is
      parseable.
- [ ] Generated consistency test: every effectful declaration has exactly one
      reachable engine binding.
- [ ] Spawn, register, attach, initialize, query, and method dispatch.
- [ ] API, signal, handler-registration, audio, mutation, and render-dependent
      operation dispatch.
- [ ] Foreign handle, stale handle, unavailable context, invalid request,
      conversion failure, and engine failure remain distinguishable.
- [ ] No declared operation reaches a blanket unsupported or no-op branch.
- [ ] Existing representative MMS scenes build the same world topology and
      component values.

### Required commands

```sh
cargo fmt --check
cargo test -p meow-meow-script
cargo test -p mittens-engine scripting
cargo check -p mittens-engine --lib
```

Run the smallest executable MMS example after each engine-facing slice. The
full workspace suite remains a release/deletion gate rather than a requirement
for every small builder commit.

## Recommended implementation slices

1. Add immutable `RuntimeSpec`, opaque IDs, nested builder skeletons, and build
   validation while adapting the existing evaluator internally.
2. Migrate `Runtime::standard()`, crate examples, and tests; remove
   `HostCapabilities` from the crate path.
3. Inventory the Mittens vocabulary and assemble the strict specification,
   initially deriving any necessary transitional views from it.
4. Route one representative vertical slice—component construction, one
   property, one method, and one namespaced API—through opaque bindings.
5. Expand the binding pattern across all components, signals, APIs, audio, and
   mutations, adding generated consistency tests.
6. Make the component registry consume crate DTOs directly and delete the
   legacy conversions from `MittensHost`.
7. Remove the parallel vocabulary/capability sources and pass the focused
   verification matrix.

Each slice must keep one production evaluator path. Do not add a runtime flag
that selects between catalog implementations.

## Completion criteria

- [ ] Mittens constructs exactly one strict crate-owned `RuntimeSpec`.
- [ ] The same builder calls produce every engine binding.
- [ ] Binding storage contains opaque IDs and implementations only, not a
      second vocabulary or schema.
- [ ] `HostCapabilities` is gone from the supported host/session API.
- [ ] Component methods, signals, APIs, audio, and mutations cross the host
      boundary using specification-assigned IDs.
- [ ] Every declared effectful operation has exactly one implementation and no
      implementation is orphaned.
- [ ] `MittensHost` distinguishes unavailable context from unsupported,
      invalid, foreign, stale, conversion, and engine failures.
- [ ] `MittensHost` no longer consults names or legacy runtime values to
      dispatch declared operations.
- [ ] Component construction consumes evaluated crate DTOs and does not
      evaluate arbitrary MMS expressions.
- [ ] Parallel component, method, signal, API, and capability specifications
      have been removed or are mechanically derived temporary views.
- [ ] Crate tests and the focused Mittens integration matrix pass.

## Downstream handoff

After this tracker passes, resume Phase 2 at the host-independent
`SessionClient` and generic runner. The runtime handed to that session and the
bindings handed to each short-lived `MittensHost` must be exactly the outputs
completed here; the runner must not accept or reconstruct a builder.

## Related tasks

- [Mittens/MMS cutover resume checkpoint](mittens-mms-cutover-resume-checkpoint.md)
- [MMS evaluator deduplication](mms-evaluator-deduplication.md)
- [Mittens MMS ownership cutover for 0.9](mittens-mms-ownership-cutover-and-0.9-release.md)
- [MMS component reflection and table dot access](mms-component-reflection-and-table-dot-access.md)
- [Component expression `props_only` body mode](component-expression-props-only-body-mode.md)
- [MMS standalone runner and source loading](mms-standalone-runner-and-source-loading.md)

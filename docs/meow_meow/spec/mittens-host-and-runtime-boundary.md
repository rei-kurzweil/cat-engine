# Mittens host and MMS runtime boundary

Date: 2026-07-29

Status: normative

## Purpose

This document defines the ownership and execution boundary between
`meow-meow-script` and `mittens-engine`.

The central invariant is:

> `meow-meow-script` is the sole implementation of MMS parsing, evaluation,
> runtime values, heap/session state, modules, and callbacks. Mittens supplies
> an engine catalog and services typed host operations on the main thread.

Requirements in this document use **must**, **must not**, **should**, and
**may** in their normative sense. The migration checklist is
[MMS evaluator deduplication](../../task/mms-evaluator-deduplication.md).

## Ownership

### `meow-meow-script`

The language crate owns:

- tokenization, parsing, AST transforms, validation, and source diagnostics
- expression, statement, component-expression, and builtin evaluation
- the public runtime `Value` and component-tree DTOs
- scopes, frames, heap objects, closures, captures, and shared object identity
- module loading semantics, module state, imports, and exports
- callback registration and invocation state
- template and live evaluation modes
- typed evaluation, protocol, and host errors
- runtime catalog types and catalog validation
- persistent evaluator sessions and the transport-neutral worker protocol

There must be no second implementation of these responsibilities in Mittens.

### `mittens-engine`

Mittens owns:

- the authoritative registration catalog for its MMS components and APIs
- `MittensHost`, which translates host requests into engine operations
- opaque component-handle translation and ECS lifetime validation
- component construction, registration, attachment, and initialization
- queries, component-method dispatch, and engine API dispatch
- `World`, `RxWorld`, render assets, intents, audio, and engine mutations
- main-thread orchestration of the crate worker
- engine-facing runner conveniences and temporary compatibility re-exports

The host may convert an already evaluated crate DTO into engine data. It must
not inspect an AST in order to decide language semantics or evaluate an
arbitrary MMS expression.

## Three runtime responsibilities

The architecture has three distinct objects. Their lifetimes and ownership
must not be conflated.

### 1. `meow_meow_script::Runtime`

`Runtime` is immutable configured MMS vocabulary. It is built from pure
language definitions plus the Mittens registration catalog.

It describes and validates:

- canonical component names and aliases
- constructors and builder calls
- named and positional properties
- component methods and signatures
- signals
- global and namespaced engine APIs

`Runtime` does not contain a script heap, a `World`, or per-evaluation
bindings. A configured runtime may be shared by multiple sessions.

### 2. Persistent crate-owned worker/session

A session owns all mutable MMS state:

- lexical scopes and frames
- the object heap and identity-bearing values
- evaluated modules and export state
- closure bodies and captures
- registered callbacks
- source/module context
- opaque live component handles

The session is host-independent and remains alive across initial evaluation,
module export calls, callbacks, keyframes, and REPL snippets. It runs inside a
crate-owned worker, one operation at a time. An operation may pause while a
correlated host request is serviced, but the session and its heap remain
resident.

The old `Session<H>` shape, where the session permanently owns a host, is not
the embedding boundary. A short-lived host is supplied indirectly for each
operation through the request/response protocol.

### 3. Short-lived `MittensHost`

`MittensHost` is constructed or borrowed on the main thread while servicing
an operation. It may temporarily borrow:

- `World`
- `RxWorld`
- a signal or intent sink
- render assets
- component and method registries
- other engine services explicitly represented by the catalog

It services host requests and is then released. It is not stored in the MMS
session, and none of its engine borrows cross to the worker thread.

```text
main thread                                      crate worker
───────────                                      ────────────
Runtime + engine catalog ── create session ────► scopes / heap / modules

runner operation ──────────────────────────────► evaluate or invoke
                      HostRequest { id, ... } ◄── pause operation
short-lived MittensHost dispatch
                      HostResponse { id, ... } ─► resume same operation
operation result ◄────────────────────────────── completion or typed error
```

## Worker/session protocol

The protocol is transport-neutral. The crate defines message DTOs and state
transitions; the embedding chooses bounded channels, queues, and wake-up
mechanisms.

### Engine-to-worker operations

The protocol must support:

- source evaluation with source identity and explicit evaluation mode
- module loading
- named and sequence export calls
- callback invocation
- REPL snippet evaluation
- REPL navigation or inspection requests
- session reset
- orderly shutdown

Each operation has an operation ID. An operation completes exactly once with a
value/evaluation result or a typed error. Reset invalidates the session's
bindings, modules, callbacks, and handles before acknowledging completion.
Shutdown rejects new work, resolves or cancels outstanding work, acknowledges
completion, and permits the worker to be joined.

### Worker-to-engine messages

The protocol must support:

- a correlated `HostRequest`
- operation completion
- typed evaluation or protocol errors
- shutdown acknowledgement

Every host request contains both its parent operation ID and a request ID.
Every `HostResponse` echoes both identities. The worker must reject a missing,
duplicate, stale, or mismatched response rather than applying it to another
operation.

An operation may issue multiple host calls. The session is sequential unless a
future protocol explicitly defines concurrent operations; correlation must not
depend on that implementation detail.

### Source loading

Source loading is a host capability. Import evaluation and module caching are
crate responsibilities, but resolving or reading an engine-relative source is
not.

A source-load request includes:

- the importing source identity, when one exists
- the requested import specifier

A successful response includes:

- a stable resolved source identity
- source text

The crate uses the resolved identity for diagnostics, relative imports, and
module-cache identity. The evaluator must not call engine-specific filesystem,
asset, or URI APIs directly.

### Errors and recovery

Unsupported catalog operations, invalid requests, foreign or stale handles,
conversion failures, source-loading failures, evaluation failures, timeouts,
and protocol violations are distinct typed errors.

An error completes the affected operation without discarding the session
unless the error is explicitly fatal. A runner timeout must not leave an
unidentified host response capable of resuming later work. Tests must prove
that the next operation can run after recoverable failures.

There is no production fallback to the engine evaluator.

## Runtime and transport values

`meow-meow-script` owns the only runtime `Value`, component-tree DTO, closure
representation, and heap model.

Identity-bearing heap values remain inside their originating session. Values
that cross the worker boundary must use crate-owned transport DTOs:

- owned scalar values and snapshots where identity is not required
- opaque component handles for host-owned ECS objects
- opaque callback references for session-owned closures
- crate-owned materialized component trees for construction

Mittens must not define an equivalent runtime object model. Legacy module
paths may temporarily re-export crate-owned DTOs so existing Rust callers
remain source-compatible.

## Callbacks and delayed execution

Delayed execution is identified by an opaque pair:

```text
(SessionHandle, CallbackHandle)
```

The session part selects the persistent heap and module state. The callback
part selects a crate-owned closure in that session.

The crate retains:

- the closure body
- lexical captures
- captured tables and arrays
- captured component handles
- all heap aliasing needed by the closure

Engine handler and keyframe components retain only the opaque callback
reference. `RuntimeClosure`, an AST function body, and a raw function `Value`
must not be stored in ECS components or sent through the host boundary.

Invoking a handler, keyframe, animation lookahead, or other delayed action
sends `InvokeCallback` to the originating session. A callback-bearing module
or template artifact must therefore retain a lease on that session. Releasing
the final lease may shut down the session and invalidate its callbacks.

## Component handles

A `ComponentHandle` is opaque to MMS. Before every query, method, attachment,
mutation, audio operation, or other component access, `MittensHost` must
validate:

1. the handle belongs to the requesting session; and
2. its encoded ECS identity still names the same live slotmap generation.

Failure of the first check is a typed foreign-handle error. Failure of the
second is a typed stale-handle error. Raw ECS identifiers and their generation
bits must not be truncated or exposed to scripts.

## Authoritative Mittens catalog

Mittens must have one engine-side registration catalog that supplies both
description and behavior for every exposed component or API.

Each component registration contains, as applicable:

- canonical MMS name and short aliases
- constructors and builder calls
- named and positional properties
- component methods and signatures
- supported signals
- concrete construction and method-dispatch functions

Each engine API registration contains:

- canonical global or namespaced API ID and aliases
- its signature
- its concrete host-dispatch function

From these registrations Mittens builds:

1. the component/API specifications used to configure
   `meow_meow_script::Runtime`; and
2. the dispatch tables used by `MittensHost`.

Manually independent lists or matches are forbidden once their consumers are
migrated. This includes `SUPPORTED_COMPONENT_NAMES`, parser-only component
name lists, method-support matches, and separately maintained capability
lists.

The crate validates duplicate names, aliases, signatures, and inconsistent
specifications. Generated consistency tests must prove that each registration
is parseable, advertised, and dispatchable, and that no dispatch branch is
orphaned.

`CallApi` must use the registered dispatch function. Known-but-unavailable and
unknown APIs return typed unsupported or invalid-request errors; they must not
silently succeed. REPL requests likewise return real catalog/host results or
typed errors rather than no-op responses.

## Registry and construction boundary

The component registry consumes an evaluated, validated crate-owned component
tree. It may:

- construct concrete engine components
- resolve constructors, builders, properties, and positionals through the
  authoritative registration
- create child topology
- register, attach, and initialize live subtrees
- use render assets required by construction

`component_registry::expression_to_value` must either serialize an already
ground AST without evaluation or be replaced by crate materialization. It must
not remain an alternate evaluator for names, calls, fields, indexes, or other
MMS expressions.

## Runner compatibility and evaluation modes

The engine-facing `MeowMeowRunner` entry points should remain source-compatible
while their internals move to the crate worker. This compatibility does not
permit a legacy evaluator fallback.

### Ordinary evaluation

- Hostless evaluation uses the same `Runtime` and crate evaluator with no
  engine capabilities.
- Live evaluation creates or selects a persistent session and services its host
  requests with short-lived `MittensHost` instances.
- Source-path entry points pass a source identity so host-provided relative
  imports behave consistently.

### Modules and factories

Template and live factory modes remain explicit.

- **Template mode** returns a crate-owned component-tree artifact and performs
  no live ECS construction.
- **Live mode** evaluates the factory in a session with host access and returns
  or attaches a live component handle.

Both modes use the same module/session implementation and host protocol.
Template artifacts without callbacks may be detached snapshots. A template
artifact containing callbacks must retain its originating session so callback
captures and heap identity remain valid when the artifact is later spawned.

Exported calls in one loaded module session must observe shared heap/table
identity across calls.

### REPL

REPL snippets, callback calls, module export calls, and ordinary evaluations
are operations on the same session abstraction. REPL bindings, heap objects,
module cache, and current source/navigation context persist until reset or
shutdown.

Engine-specific tree traversal and formatting may live in `MittensHost` or a
REPL adapter. MMS expression evaluation does not.

## Threading and reentrancy

MMS evaluation and heap mutation occur on the crate worker. `World`, `RxWorld`,
render assets, component registries, and intent sinks remain on the main
thread.

The main-thread runner drives an operation by repeatedly servicing correlated
host requests until completion, error, or timeout. It must not hold an engine
borrow in persistent session state.

Host dispatch is not an evaluator callback: it performs one cataloged engine
operation and returns a DTO. A host implementation must not re-enter the same
session synchronously. Work caused by signals during dispatch is queued as a
later callback operation.

The transport should block or wake efficiently. The legacy ring-buffer
spin/yield protocol is not part of the public or target architecture.

## Migration constraints

During migration:

- `src/scripting/world_evaluator.rs` is frozen; no new language semantics may
  be added there
- parity harnesses may run both evaluators in tests
- production code must not select an evaluator per syntax feature
- legacy public paths may re-export crate-owned DTOs temporarily
- each caller category must pass through the crate worker before its legacy
  implementation is deleted

The gated order is: pure parity, catalog and host completion, ordinary
runners, modules/factories, callbacks/keyframes, REPL/worker, then legacy
deletion.

## Conformance

The boundary is complete only when:

- `src/scripting/world_evaluator.rs` and its ring-buffer protocol are deleted
- the crate owns the only evaluator, runtime `Value`, heap, module state, and
  closure model
- all engine operations are registered and dispatched through the catalog
- all component operations reject foreign and stale handles
- no engine helper evaluates an arbitrary MMS expression
- handlers, keyframes, and callback-bearing templates preserve session heap
  identity after initial evaluation returns
- all examples and engine-facing runners use the crate worker/session path
- the parity, catalog, integration, lifetime, worker, factory-mode, and
  workspace test suites pass

## Related specifications

- [Host API](host-call-api.md)
- [MeowMeowRunner](script-runner.md)
- [`eval_with_world`](eval-with-world.md)
- [Environment, heap, and object world](env-heap-object-world.md)
- [Module imports and exports](module-import-export.md)

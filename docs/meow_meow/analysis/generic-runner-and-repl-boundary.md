# Generic runner and REPL boundary

Date: 2026-07-30

Status: design inventory

## Question

`meow-meow-script` needs a useful runner and REPL that work with any host.
Neither should know how a `RuntimeSpec` is assembled or require callers to use
a configuration builder at the point of use.

This document inventories what the current implementations under
`src/scripting` get from Mittens and proposes the generic boundary needed to
move their language/session responsibilities into the crate.

The conclusion is:

> Runtime construction happens before runner or REPL construction. Both
> consume an already-created crate-owned `SessionClient`, service its generic
> host requests, and emit crate-owned output events.

The Mittens wrappers provide `MittensHost`, component/world inspection, source
loading, signal payload adaptation, terminal ownership, and frame-loop
integration. They do not implement runner state or REPL semantics.

## Current runner inventory

The current engine runner is `src/scripting/runner.rs`.

### Language/session responsibilities

These belong in `meow-meow-script`:

- start an evaluation operation
- preserve or release session state according to the operation
- correlate multiple host calls with one operation
- detect completion, failure, timeout, reset, and shutdown
- load modules into persistent session state
- call named and sequence exports
- select template versus live factory mode
- invoke callbacks in their originating session
- retain typed evaluation and protocol errors
- return crate-owned values, component artifacts, module handles, and
  callback references

### Generic orchestration responsibilities

These can also move to the crate without knowing any particular host:

- submit source text plus optional `SourceId`
- drive a worker operation until it completes
- pass each `HostRequest` to a caller-supplied service
- send the correlated result back to the worker
- collect diagnostics and non-host output
- apply a caller-supplied timeout/cancellation policy
- perform orderly reset and shutdown

### Mittens dependencies in the current runner

These remain outside the generic runner:

| Current dependency | Why Mittens-specific | Target owner |
|---|---|---|
| `World` and `ComponentId` | Concrete ECS storage and generation | `MittensHost` |
| `RxWorld` | Concrete signal-handler registry | `MittensHost` |
| `RenderAssets` | Engine render construction context | `MittensHost` |
| `SignalEmitter` and `IntentValue` | Engine command pipeline | Mittens runner adapter/host |
| `component_registry::spawn_tree*` | Concrete ECS component construction | `MittensHost` component sink |
| `component_method_registry` | Concrete method implementation | `MittensHost` operation bindings |
| world query traversal | Mittens component topology and selectors | `MittensHost` |
| audio component cloning | Concrete audio component implementation | `MittensHost` |
| `EventSignal` to MMS value conversion | Mittens signal payload types | `MittensHost` signal adapter |
| filesystem reads in `eval_file` | One source-loading policy | a reusable filesystem source loader or Mittens adapter |
| `rtrb` and spin/yield loops | Legacy engine worker transport | crate worker/session implementation |

### Emitted components

The existing hostless runner collects `IntentValue`, while the live runner
constructs ECS components directly. Neither result shape is host-neutral.

The generic concept is a component command sink:

```rust
pub trait ComponentSink {
    fn service_component(
        &mut self,
        command: ComponentCommand,
    ) -> Result<ComponentReply, HostError>;
}

pub enum ComponentCommand {
    Emit {
        tree: ComponentTree,
    },
    Register {
        tree: ComponentTree,
    },
    Attach {
        parent: Option<ComponentHandle>,
        child: ComponentHandle,
    },
}

pub enum ComponentReply {
    Component(ComponentHandle),
    Unit,
}
```

This is a behavioral sketch, not a requirement for these exact variant names.
It captures the minimum structural resemblance to Mittens:

- the evaluator emits an already evaluated crate-owned component tree
- the sink decides what “emit” means
- registration can return a live opaque handle
- attachment can place a previously registered handle

Useful generic implementations include:

- `CollectingComponentSink`, which stores emitted trees for tests, tools, or
  static compilation; for register/attach flows it allocates synthetic opaque
  handles and records the resulting attachment graph
- `RejectingComponentSink`, used by pure evaluation that forbids component
  effects
- a callback/closure adapter for lightweight embedders
- `MittensHost`, which constructs and attaches ECS components

The main `HostRequest` protocol may embed these component commands rather than
adding a second dispatch mechanism. `ComponentSink` is a convenience boundary
for hosts that only need component output; it is not another runtime
specification.

## Proposed generic runner API

The runner is downstream of runtime configuration:

```rust
let session = configured_runtime.spawn_session()?;
let mut runner = Runner::new(session);

let result = runner.run(
    RunRequest::EvaluateSource {
        source,
        source_id,
        mode: EvaluationMode::Live,
    },
    &mut host,
    &mut output,
)?;
```

The important public types are conceptually:

```rust
pub struct Runner {
    session: SessionClient,
}

pub enum RunRequest {
    EvaluateSource { /* source, identity, mode */ },
    LoadModule { /* source or source identity */ },
    CallExport { /* module, export, args, mode */ },
    InvokeCallback { /* callback, args */ },
    Reset,
    Shutdown,
}

pub trait HostService {
    fn service(
        &mut self,
        request: HostRequest,
    ) -> Result<HostResponse, HostError>;
}

pub trait RunOutput {
    fn emit(&mut self, event: RunEvent);
}
```

`HostService` names a role in this sketch. It should normally be the existing
crate `Host` contract, extended as needed for the host-independent session
protocol, rather than a second competing host trait. Closure adapters can make
one-off hosts convenient.

`RunEvent` covers output that is not a request for a return value, such as
diagnostics, printed text, tracing, or an emitted artifact in a collecting
mode. It must contain crate-owned DTOs only.

The exact synchronous API may be complemented by polling/async forms:

- `run(...)` drives to completion
- `start(...)` returns an operation handle
- `poll(...)` makes bounded progress for a frame loop
- an async adapter awaits the same protocol

These are adapters over one `SessionClient`; they must not create distinct
evaluation semantics.

### Why the runner takes a session, not a builder

The runner does not need to know:

- which components exist
- which aliases or methods were registered
- which signals are available
- how a runtime specification was assembled

Those decisions were compiled into the session's `Runtime` before the runner
received it. This makes the same runner usable with:

- standard pure MMS
- a test runtime
- Mittens
- another engine
- a tooling host

Convenience constructors may create a standard hostless runtime, but the core
`Runner::new(SessionClient)` API remains configuration-free.

## Current REPL inventory

The current REPL is split across:

- `src/scripting/repl/backend.rs`
- `src/scripting/repl/navigation.rs`
- `src/scripting/repl/formatter.rs`
- `src/scripting/repl/frontend.rs`

### Host-neutral REPL behavior

These responsibilities belong in `meow-meow-script`:

- classify `ls`, `pwd`, `cd`, and ordinary MMS snippets
- determine whether multiline input is complete
- queue inputs and allow only one active session operation
- evaluate snippets in a persistent session
- evaluate navigation expressions without auto-emission
- preserve bindings and heap identity across snippets
- own cursor and breadcrumb semantics
- navigate crate-owned tables, arrays, and component-tree artifacts
- format pure values and component-tree artifacts as MMS source
- coordinate reset, error recovery, and shutdown
- produce structured REPL output events

### Mittens dependencies in the current REPL

| Current dependency | Current use | Generic replacement |
|---|---|---|
| `World` | roots, parents, children, labels, liveness, GUID lookup | host inspection requests |
| `ComponentId`/`KeyData` | cursor identity and short-ID parsing | opaque `ComponentHandle` plus host-resolved path segments |
| `engine::repl::util::format_ls_line` | world listing presentation | structured inspection entries, formatted by REPL or UI |
| `subtree_to_ce_ast` | live component display/dump | host `RenderSource`/snapshot inspection |
| `MittensHost` behavior copied into `service_host_call` | spawn/query/method/audio/handler dispatch | the same `HostService` used by `Runner` |
| `EventSignal` conversion | handler argument construction | signal adapter registered by Mittens |
| `RenderAssets` | component construction while evaluating snippets | short-lived `MittensHost` context |
| `RxWorld` | register delayed callbacks | short-lived `MittensHost` context |
| `SignalEmitter` | engine effects and intents | short-lived `MittensHost` context |
| `claim_stdin`/`release_stdin` | coordinate terminal ownership with engine REPL | embedding-owned terminal adapter |
| `println!`, `eprintln!`, ANSI clear | terminal presentation | structured `ReplEvent` output |
| engine frame `sync(...)` | nonblocking polling from the game loop | generic poll API plus Mittens frame adapter |

### What should not move into the crate

The crate REPL must not:

- know about `World`, `RxWorld`, or `RenderAssets`
- parse or manufacture a Mittens `ComponentId`
- walk ECS nodes directly
- convert concrete Mittens events into MMS values
- claim a process-global stdin on behalf of an embedding
- print directly as its only output mechanism
- duplicate `MittensHost` dispatch

## Proposed generic REPL API

Like the runner, the REPL receives an already configured session:

```rust
let session = configured_runtime.spawn_session()?;
let runner = Runner::new(session);
let mut repl = Repl::new(runner);

repl.submit("let x = 1");
repl.submit("x + 1");

while let Some(event) = repl.poll(&mut host)? {
    ui.handle(event);
}
```

The core REPL is programmatic. Standard terminal I/O is an optional adapter,
not part of its semantics:

```rust
pub struct Repl {
    runner: Runner,
    navigation: ReplNavigation,
    pending: VecDeque<String>,
}

pub enum ReplEvent {
    Value(ValueView),
    Text(String),
    Diagnostic(Diagnostic),
    Listing(Vec<InspectionEntry>),
    Location(ReplPath),
    ClearRequested,
    OperationComplete,
}
```

An embedding may:

- feed lines from stdin, a GUI console, a socket, or tests
- render `ReplEvent` to a terminal, editor panel, log, or protocol
- call `poll` once per frame or use a blocking/async adapter

No part of this API mentions `RuntimeSpec` or `RuntimeBuilder`.

## Generic inspection boundary

REPL navigation spans two ownership domains:

1. session-owned values such as tables, arrays, and component artifacts
2. host-owned live components

The crate must not copy identity-bearing session values into an engine
navigator. Instead the session retains them behind an opaque `ValueRef`:

```rust
pub enum ReplTarget {
    World,
    SessionValue(ValueRef),
    Component(ComponentHandle),
}

pub enum InspectRequest {
    Validate { target: ReplTarget },
    List { target: ReplTarget },
    ResolveChild { target: ReplTarget, segment: String },
    Parent { target: ReplTarget },
    Describe { target: ReplTarget },
    RenderSource { target: ReplTarget },
}

pub struct InspectionEntry {
    pub segment: String,
    pub label: String,
    pub kind: InspectionKind,
    pub target: ReplTarget,
}
```

Session-value inspection is handled inside the worker because the heap lives
there. Component/world inspection becomes a typed host request. The REPL
combines both response forms into one navigation model.

This inspection protocol is not a runtime vocabulary specification:

- it does not declare MMS names, methods, builtins, or signals
- it does not affect parsing or validation
- a host may return `UnsupportedHostOperation`
- pure table/array navigation still works without host inspection

Mittens implements live inspection with `World`:

- list world roots or component children
- resolve a child by index, authored name, GUID, or engine-specific short ID
- find a parent
- validate generation/liveness
- provide labels and kinds
- render or snapshot a live subtree as MMS source

Engine-specific path syntax should be resolved by the host rather than taught
to the generic REPL. For example, parsing slotmap `indexvversion` strings does
not belong in `meow-meow-script`.

## REPL commands versus MMS builtins

Shell commands and MMS calls must remain distinct:

- `ls`, `pwd`, and `cd ...` are REPL commands
- `ls()`, `let cd = ...`, and other valid MMS remain language input

`tree(value)`, `dump(value)`, `help()`, `clear()`, and `reset()` are currently
implemented partly as evaluator builtins and partly as host calls. The
migration should choose one consistent home:

- REPL-only controls should become `ReplCommand` variants and require no
  `RuntimeSpec` entry.
- Language-call forms intended to work outside a REPL remain standard
  crate-provided builtins and emit structured output/inspection requests.

They must not be Mittens-only builtin declarations merely to support the
generic REPL.

## Source loading and terminal adapters

The crate may provide reusable adapters without making them core requirements:

- `FilesystemSourceLoader`
- `BlockingRunner`
- `PollingRunner`
- `AsyncRunner`
- `TerminalRepl`

`FilesystemSourceLoader` implements the same source-load request used by any
host. `TerminalRepl` reads and writes standard I/O but does not claim ownership
through Mittens globals. Mittens wraps it or supplies its own frontend when it
must coordinate stdin with another engine console.

## Proposed ownership after migration

### `meow-meow-script`

- `SessionClient` and worker protocol
- generic `Runner`
- `RunRequest`, `RunResult`, `RunEvent`, and output sink traits
- component command/reply DTOs and collecting/rejecting sinks
- generic `Repl`
- REPL input classification and multiline completion
- navigation over session-owned values
- generic inspection request/response DTOs
- pure value and component-artifact formatting
- optional filesystem, blocking, polling, async, and terminal adapters

### Mittens

- construction of the one `RuntimeSpec`
- `MittensHost`
- component sink backed by ECS construction
- host operation bindings
- source loading policy
- live world/component inspection
- signal payload adaptation
- frame-loop polling adapter
- terminal ownership coordination
- compatibility wrappers retaining existing `MeowMeowRunner` and
  `MeowMeowRepl` entry points

## Open design decisions

The following require prototypes but do not change the ownership decision:

1. Whether `HostService` is a trait or a closure accepted by `Runner::run`.
2. Whether `ComponentSink` is a public subtrait/adapter or only a group of
   `HostRequest` variants.
3. Whether `SessionClient` exposes blocking, polling, and async methods
   directly or through runner adapters.
4. How `ValueRef` lifetime is tied to reset and session release.
5. Whether live `RenderSource` returns source text, a component-tree snapshot,
   or both.
6. Which of `tree`, `dump`, `help`, `clear`, and `reset` remain callable MMS
   builtins versus REPL-only commands.
7. Whether a standard terminal frontend is a default feature or an optional
   crate feature.

## Acceptance tests for the generic boundary

- The crate runner evaluates pure source with `RejectingComponentSink`.
- The crate runner collects emitted component trees with
  `CollectingComponentSink`.
- A fake host registers, attaches, queries, and invokes methods without
  Mittens types.
- The same session runs through blocking and polling runner adapters.
- The crate REPL preserves bindings and table identity across submissions.
- The crate REPL navigates tables, arrays, and component artifacts without a
  host.
- A fake inspection host supplies world roots, component children, parents,
  labels, and rendered source.
- Unsupported live inspection does not prevent pure-value navigation.
- REPL input and output work without stdin/stdout by using programmatic
  submission and `ReplEvent`.
- The Mittens wrapper runs the same runner/REPL using `MittensHost`.
- No crate runner or REPL module imports engine types or requires a
  `RuntimeSpec` builder at construction.

## Related

- [Mittens host and MMS runtime boundary](../spec/mittens-host-and-runtime-boundary.md)
- [Host API](../spec/host-call-api.md)
- [MeowMeowRunner](../spec/script-runner.md)
- [MMS evaluator deduplication checklist](../../task/mms-evaluator-deduplication.md)

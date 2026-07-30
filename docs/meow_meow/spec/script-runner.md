# `MeowMeowRunner` — script and module evaluation

Status: target architecture with source-compatible migration

`MeowMeowRunner` is the high-level Mittens entry point for evaluating MMS
source and calling MMS module exports from Rust. Existing engine-facing entry
points should remain source-compatible, but their canonical implementation is
the persistent `meow-meow-script` worker/session and short-lived
`MittensHost`.

The ownership and lifetime rules are normative in
[Mittens host and MMS runtime boundary](mittens-host-and-runtime-boundary.md).
The former engine-local evaluator and ring-buffer runner are superseded.

## Scope

The runner exposes two related decisions:

1. ordinary source evaluation is hostless or live;
2. a component-producing module export is evaluated as a template or as a
   live factory.

These choices affect host capabilities and result shape. They do not select
different evaluators.

## Compatibility surface

Representative engine-facing types remain:

```rust
pub struct EvalOutput {
    pub intents: Vec<IntentValue>,
    pub errors: Vec<String>,
}

pub struct MeowMeowRunner;

pub enum ModuleFactoryEvalMode {
    Template,
    Live,
}
```

`EvalOutput` remains a compatibility type because source evaluation may
produce partial output and multiple diagnostics. Internally the runner keeps
crate-owned typed errors until mapping them at this boundary.

The engine may temporarily re-export crate-owned `Value`, `MaterializedCE`,
component tree, handle, and error DTOs from legacy module paths. It must not
maintain equivalent engine-local types.

## Ordinary source evaluation

Representative entry points are:

```rust
impl MeowMeowRunner {
    pub fn eval(source: &str) -> EvalOutput;

    pub fn eval_with_timeout(
        source: &str,
        timeout: Duration,
    ) -> EvalOutput;

    pub fn eval_with_path(
        source: &str,
        source_path: &str,
    ) -> EvalOutput;

    pub fn eval_with_world(
        source: &str,
        world: &mut World,
        rx: &mut RxWorld,
        emit: &mut dyn SignalEmitter,
    ) -> EvalOutput;
}
```

Asset/path variants additionally supply render assets and a stable source
identity to the main-thread host context.

### Hostless mode

`eval` uses the configured crate `Runtime` and a crate session without engine
capabilities.

- Pure MMS semantics are fully available.
- Component expressions may become evaluated component-tree DTOs.
- Live component allocation, queries, methods, and engine APIs return typed
  unsupported-host errors.
- No call falls back to the engine evaluator.

### Live mode

`eval_with_world` creates or selects a persistent crate session and drives one
source-evaluation worker operation.

- Live component bindings become opaque component handles.
- Each ECS operation is a correlated request serviced by a short-lived
  `MittensHost` on the main thread.
- `World`, `RxWorld`, render assets, registries, and intent sinks never enter
  the crate session.
- Published handlers or keyframes retain a lease to the session so callbacks
  can resume its heap later.

The lifecycle is detailed in [`eval_with_world`](eval-with-world.md).

### Source paths and imports

Path-aware entry points provide a source identity, not permission for the
evaluator to read engine-relative files directly.

For an import, the worker requests source loading from the host using the
importer identity and specifier. The returned resolved identity controls
nested relative imports, diagnostics, and the per-session module cache.

## Sessions

All runner operations target a crate-owned session. A session owns:

- bindings, frames, and heap objects
- modules and exports
- closures and callbacks
- opaque component handles
- REPL/source context

The session does not own `MittensHost`. The main-thread runner services host
requests only for the duration of the current operation.

A short-lived hostless call may release its session after returning. Any
module, handler, keyframe, or callback-bearing artifact that needs delayed
execution retains a session lease.

## Module export operations

Representative module helpers remain:

```rust
impl MeowMeowRunner {
    pub fn call_mms_module_fn(...) -> Result<Value, String>;

    pub fn materialize_mms_module_component(...)
        -> Result<MaterializedCE, String>;

    pub fn spawn_mms_module_component(...)
        -> Result<ComponentId, String>;

    pub fn spawn_mms_module_component_uninitialized(...)
        -> Result<ComponentId, String>;
}
```

Loading a module creates or uses persistent module state in a crate session.
Named exports, sequence exports, and repeated exported calls from that module
observe the same heap and table identities.

The public compatibility layer maps the crate's typed errors to `String` for
these existing signatures. The worker protocol and session API retain typed
errors.

## Factory evaluation modes

Template and live modes are explicit and share the same crate evaluator,
module/session state, and host protocol.

### `ModuleFactoryEvalMode::Template`

Template mode returns a crate-owned evaluated component-tree artifact without
allocating live ECS components.

Use it when Rust needs:

- an authored shell or prefab-like description
- a tree to inspect or rewrite
- a stable slot into which Rust-managed content will later be spliced

A callback-free template is an owned snapshot and may be detached from its
evaluation session.

A callback-bearing template is not fully detached. It carries opaque callback
references and a lease on the originating session. When the tree is later
spawned and its handlers or keyframes are registered, invocation resumes that
session so captures and shared heap identity remain valid.

Template materialization must not serialize raw closure bodies or function
`Value`s into the tree.

### `ModuleFactoryEvalMode::Live`

Live mode evaluates the export with main-thread host servicing.

- Intermediate component bindings may become live component handles.
- The export may return an already registered live root or an evaluated tree
  that the host then spawns.
- Runtime callbacks may capture live handles.
- Initialized and uninitialized spawn helpers differ only in final
  attachment/initialization policy.

Use live mode for previews, placement, and runtime subtrees that should behave
like ordinary live MMS immediately.

### Decision rule

- If Rust truly needs an authored tree, use template materialization.
- If Rust wants an actual runtime subtree, use live spawn.

Do not materialize a template and assume it reproduces live factory semantics.
Conversely, live mode is not a promise that every export has a meaningful
detached `MaterializedCE` result.

## Callback invocation

The engine stores delayed execution as:

```text
(SessionHandle, CallbackHandle)
```

Signal handlers, named/global handlers, keyframes, animation lookahead, and
other delayed work invoke a worker callback operation on the originating
session.

The engine must not store:

- `RuntimeClosure`
- a raw function `Value`
- an AST function body
- a copied closure environment

Session release or reset invalidates its callback references. Attempting to
invoke one produces a typed error.

## Compilation and execution pipeline

Every mode uses one crate-owned pipeline:

```text
source text
    │
    ▼
Tokenizer → Parser → AST transforms → Runtime validation
    │
    ▼
persistent crate session
    │
    ├─ pure evaluation and heap mutation
    ├─ module/export/callback state
    └─ correlated HostRequest ──► short-lived MittensHost
                                      │
                                      ▼
                             catalog dispatch + ECS
```

Mittens' component registry receives already evaluated crate DTOs. It performs
construction and conversion, not MMS expression evaluation.

## Completion and shutdown

Each runner call is a correlated worker operation and completes once with an
evaluation result or typed error. Multiple host calls may occur before
completion.

A timeout prevents late responses from being consumed by later operations.
Recoverable failures leave the session usable or cause an explicit reset.
Shutdown rejects new operations, resolves or cancels pending work,
acknowledges completion, and joins the worker.

## Examples

Hostless source evaluation:

```rust
let output = MeowMeowRunner::eval(include_str!("scene.mms"));
for intent in output.intents {
    universe.command_queue.push_intent_now(scope, intent);
}
```

Live source evaluation:

```rust
let output = MeowMeowRunner::eval_with_world(
    source,
    &mut universe.world,
    &mut universe.rx,
    &mut universe.command_queue,
);
```

Template materialization:

```rust
let shell = MeowMeowRunner::materialize_mms_module_component_from_file(
    "assets/components/panels.mms",
    "world_panel",
    args,
    Some(world),
    Some(emit),
)?;
```

Live factory instantiation:

```rust
let preview = MeowMeowRunner::spawn_mms_module_component_uninitialized_from_file(
    "assets/components/animated.mms",
    "rainbow_animated",
    vec![],
    world,
    emit,
)?;
world.add_child(preview_slot, preview)?;
```

The template example is appropriate only when the caller needs an authored
tree. Runtime previews should use the live path.

## Related specifications

- [Mittens host and MMS runtime boundary](mittens-host-and-runtime-boundary.md)
- [Host API](host-call-api.md)
- [`eval_with_world`](eval-with-world.md)
- [Module imports and exports](module-import-export.md)

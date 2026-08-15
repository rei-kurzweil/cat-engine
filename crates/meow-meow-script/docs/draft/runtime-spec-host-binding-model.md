# RuntimeSpec host binding model

Status: design draft; component factory and operation IDs are partially
implemented, while typed handler bindings and signal registration remain
future work.

Related:

- [Configure a script host](../how_to/configuring_a_script_host.md)
- [Host values, resources, and bound receivers](host-values-resources-and-bound-receivers.md)
- [Checker and registry integration](checker-and-registry.md)

## Purpose

`RuntimeSpec` owns the script-visible declaration: its name, aliases, nesting,
argument and result types, and diagnostic identity. A configured runtime build
assigns each host-effectful declaration an opaque `OperationId` and associates
that ID with exactly one host-defined binding entry.

```text
source name + signature
        ↓ RuntimeSpec resolution
OperationId + typed/evaluated values
        ↓ binding-table lookup
host implementation
```

Names remain available for source parsing, diagnostics, reflection, and
serialization. A configured host must not select an implementation by matching
those names again. A present but unknown or mismatched operation ID fails
closed rather than falling through to name-based dispatch.

Pure MMS declarations have no host binding. Equivalent declarations in two
separately built configured runtimes may have different IDs and bindings.
`ConfiguredRuntime<I>` therefore keeps the compiled runtime and its binding
table together.

## Proposed host binding categories

The binding type remains host-defined: `RuntimeSpecBuilder<I>` does not require
Mittens or prescribe a particular Rust enum. A useful host implementation will
usually need categories resembling:

```rust,ignore
enum HostBinding {
    ComponentConstructor(ComponentConstructorBinding),
    ComponentInitializer(ComponentInitializerBinding),
    ComponentMethod(ComponentMethodBinding),
    Api(ApiBinding),
    Signal(SignalBinding),
}
```

The three component categories encode receiver lifecycle rather than merely
whether an operation mutates:

- `ComponentConstructor` has no receiver and creates a component. Every
  component declaration has an implicit default constructor for bare syntax
  such as `Transform {}`; a named form such as `Transform.position(...) {}`
  selects an alternate constructor instead.
- `ComponentInitializer` receives the newly constructed component while its
  authored tree is being assembled but before that tree is initialized. It
  covers chained builder calls, implied-receiver calls in the component body,
  and authored properties.
- `ComponentMethod` receives a checked live component handle. It covers live
  mutation, getters which return values, commands which emit intents, and
  operations which begin asynchronous work.

APIs remain separate because they have no component receiver and may require
unrelated host services. A host-provided builtin may use the API handler shape
or a separate `HostBuiltin` category if its evaluation rules differ.

## Component construction and receiver phases

One component expression selects exactly one constructor binding. Bare syntax
selects the declaration's implicit default constructor; a first named call
selects the matching named constructor:

```text
Transform {}
└── ComponentConstructor::TransformDefault

Transform.position(1.0, 2.0, 3.0) {}
└── ComponentConstructor::TransformPosition

Renderable.cube() {}
└── ComponentConstructor::RenderableCube
```

There is no separate host factory operation followed by a named constructor
operation. The constructor itself creates the receiver. A declaration may
still have a non-operation component/type identity for reflection, handle
typing, and diagnostics, but that identity must not become a second host
construction dispatch.

After construction, chained calls and component-body calls receive the new
component in its construction phase:

```text
Transform.position(1.0, 2.0, 3.0).scale(2.0, 2.0, 2.0) {}
├── ComponentConstructor::TransformPosition
└── ComponentInitializer::TransformScale
    └── receiver: the newly constructed Transform

Transform {
    scale(2.0, 2.0, 2.0)
}
├── ComponentConstructor::TransformDefault
└── ComponentInitializer::TransformScale
    └── implied receiver: the surrounding Transform
```

Only a method call on a live component value selects `ComponentMethod`:

```text
let transform = Transform {}
transform.trs()
└── ComponentMethod::TransformTrs
    └── receiver: checked live Transform handle
```

The same source spelling may therefore name distinct RuntimeSpec declarations
and receive distinct IDs:

```text
Transform.constructor(position) → constructor OperationId
Transform.initializer(position) → initializer OperationId
Transform.method(position)      → live-method OperationId
```

Syntax and receiver state select the declaration; the host never resolves the
spelling again. Initializer and method handlers may share internal Rust code,
but their bindings remain distinct so initialization-only behavior cannot be
invoked accidentally on a live component, or vice versa.

The categories describe execution shapes, not necessarily unique Rust
functions. Several operation IDs may bind to the same reusable handler, but
each host-effectful operation has exactly one binding-table entry in a given
configured build.

## Signal bindings and handler registration

A signal binding is not the authored callback and is not an emitted event
instance. It associates a RuntimeSpec signal declaration with the host's
signal identity and registration behavior.

For example, a Mittens binding could map the declaration for `Click` to the
engine's `SignalKind::Click`. Registering an authored handler would then cross
the boundary with:

```text
signal OperationId
component handle or signal scope
opaque CallbackHandle owned by the MMS session
```

The host resolves the signal ID, validates the component handle and session,
and installs a route containing the opaque callback reference. It must not
retain an MMS closure body or heap object. When the engine emits the signal,
the host queues callback invocation back to the originating MMS session; it
must not synchronously re-enter evaluation while servicing host dispatch.

An illustrative request is:

```rust,ignore
RegisterSignalHandler {
    signal_operation_id: OperationId,
    receiver: Option<ComponentHandle>,
    callback: CallbackHandle,
}
```

The eventual protocol must also define session leases, handler removal,
receiver liveness, queued event arguments, and failure after a session is
released.

## Illustrative handler shapes

The exact Rust representation is deliberately open, but the semantic inputs
are approximately:

```rust,ignore
type ComponentConstructorHandler = fn(
    context: &mut HostContext,
    args: &[Value],
) -> Result<ComponentHandle, HostError>;

type ComponentInitializerHandler = fn(
    context: &mut HostContext,
    receiver: ComponentHandle,
    args: &[Value],
) -> Result<(), HostError>;

type ComponentMethodHandler = fn(
    context: &mut HostContext,
    receiver: ComponentHandle,
    args: &[Value],
) -> Result<Value, HostError>;

type ApiHandler = fn(
    context: &mut HostContext,
    args: &[Value],
) -> Result<Value, HostError>;
```

A real implementation may use enums, function pointers, trait objects, or
generated dispatch. Operations requiring `World`, render assets, audio,
networking, intent emission, or another service must report unavailable host
context distinctly from an unknown operation.

## Required invariants

- Every host-effectful declaration has one operation ID and one binding entry
  in the configured build.
- No binding exists without a matching declaration, and no host declaration
  exists without a binding.
- RuntimeSpec names and signatures are not duplicated in the binding table.
- Host dispatch uses operation IDs; names are diagnostic data only.
- Unknown and mismatched IDs fail before host state is mutated.
- Initializers may receive only a component created for the current tree
  assembly and run before tree initialization.
- Live component handles are checked for session ownership and ECS generation
  before component methods or signal registration.
- Callback closures and heap identity remain inside MMS. Hosts retain only
  opaque callback references and enqueue invocation.
- Missing runtime context is different from an unsupported or invalid
  operation.

## Current Mittens transition

Mittens currently uses string-bearing `MittensBinding` variants as typed
routing tokens and represents construction as a component factory plus an
optional constructor operation. The RuntimeSpec launch-scene slice validates
those tokens by operation ID before world mutation, but this is a transitional
two-stage construction model and the final engine implementations still
contain name matches.

The intended transition is to collapse the factory plus optional constructor
into exactly one `ComponentConstructor`, split construction-phase builder and
property bindings into `ComponentInitializer`, reserve `ComponentMethod` for
checked live receivers, replace string tokens with typed variants or handlers,
migrate implementation bodies out of the legacy component registries, carry
method and signal IDs through the host protocol, and then delete the
name-dispatch compatibility paths.

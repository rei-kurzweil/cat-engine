# ₊˚ʚ `eval_with_world` — live session evaluation

Status: target architecture

`MeowMeowRunner::eval_with_world` is the live engine-facing evaluation path.
Its public purpose remains unchanged: MMS may construct, navigate, and mutate
components in a running world. Its implementation uses the persistent
crate-owned worker/session defined by
[Mittens host and MMS runtime boundary](mittens-host-and-runtime-boundary.md).

The legacy engine evaluator, one-shot evaluator thread, and spin/yield
ring-buffer diagrams previously documented here are superseded.

## Why live evaluation exists

Hostless `eval` can evaluate pure MMS and materialize component trees, but it
cannot manufacture an ECS identity. Live evaluation is required when authored
code needs a component handle:

```mms
let box = T.position(0, 0, -1) {
    R { CUBE; C.rgba(1, 1, 1, 1) }
}

fn handle_press() {
    box."C".set_color(0, 1, 0, 1)
}
```

The `box` binding and later query/method call must refer to the same live,
generational ECS object.

## Public runner behavior

The engine-facing signature may retain its existing convenience form:

```rust
pub fn eval_with_world(
    source: &str,
    world: &mut World,
    rx: &mut RxWorld,
    emit: &mut dyn SignalEmitter,
) -> EvalOutput
```

Path/assets variants also provide a source identity and render assets to the
host-servicing context. Engine types above belong to the Mittens runner only;
they do not enter `meow-meow-script`.

`EvalOutput` preserves the runner's public compatibility behavior. Internally,
typed crate evaluation and host errors are retained until the compatibility
layer maps them into the existing output.

## Lifecycle

```text
main thread                                      crate worker/session
───────────                                      ────────────────────
select/create session
send EvaluateSource(operation_id, source, mode)
                                                parse, validate, evaluate
HostRequest(register tree) ◄──────────────────── pause
MittensHost { world, rx, emit, assets }.dispatch
HostResponse(component handle) ────────────────► bind live ComponentObject
HostRequest(register callback) ◄──────────────── pause
MittensHost stores opaque callback reference
HostResponse(unit) ─────────────────────────────► continue
EvaluationComplete ◄──────────────────────────── return output; session stays alive
```

The runner drives the operation until completion, typed error, or timeout.
`MittensHost` is short-lived and borrows engine services only while dispatching
a request. The crate session outlives the call when modules, handlers,
keyframes, or other delayed behavior retain it.

## Session selection and lifetime

An ordinary fire-and-forget call may create a session scoped to that
evaluation. A call that publishes callbacks or module exports must return or
store a session lease so the originating heap remains alive.

Every delayed callback is addressed by `(SessionHandle, CallbackHandle)`.
Invoking it submits a later worker operation to the same session. The engine
does not retain a closure body or raw function `Value`.

Reset invalidates session bindings, heap objects, modules, callbacks, and
component ownership records. Releasing the final lease permits orderly worker
shutdown.

## Live bindings

When a component expression is bound in live mode, the crate sends an
evaluated component-tree DTO to the host for registration. The response
contains an opaque component handle, and the session binds a crate-owned
`Value::ComponentObject` containing that handle.

The host validates session ownership and live ECS generation whenever that
handle is used for:

- attachment
- scoped query
- component-method dispatch
- handler registration
- audio or engine mutation

Foreign and stale handles are typed errors, not `null` results.

## Main-thread host servicing

Only `MittensHost` touches `World`, `RxWorld`, render assets, engine
registries, or intent sinks. It is built from the authoritative engine
registration catalog and performs one requested operation at a time.

The worker owns parsing and all evaluation. The host receives evaluated DTOs;
it never evaluates an AST or arbitrary MMS expression.

Signals raised while servicing a host call enqueue callback work. They do not
synchronously re-enter the worker operation that caused them.

## Relative imports

Path-aware live evaluation supplies a source identity. If evaluation imports
another module, the worker sends a source-load host request containing the
importer identity and import specifier. The host returns source text and a
stable resolved identity.

The crate then evaluates and caches the module in the same session. It does
not perform engine-specific filesystem or asset loading itself.

## Completion, timeout, and recovery

Each evaluation and each nested host call is correlated. The runner accepts
responses only for the active session operation and request.

On timeout or recoverable failure:

- the affected operation completes with a typed error
- late responses cannot be applied to another operation
- the session is either left in a defined recoverable state or explicitly
  reset
- subsequent operations can continue when the error is non-fatal

Shutdown stops accepting operations, resolves or cancels pending work,
acknowledges completion, and joins the worker. Busy polling is not part of the
target transport.

## Hostless versus live

Both paths use the same crate `Runtime`, parser, evaluator, runtime `Value`,
and heap implementation.

| Behavior | `eval` | `eval_with_world` |
|---|---|---|
| Pure language | supported | supported |
| Materialize component DTO | supported | supported |
| Allocate live component | unsupported | host request |
| Query/method on live handle | unsupported | host request |
| Register delayed engine callback | unsupported | opaque callback reference |
| Engine-relative source loading | only with source host | source host |

There is no fallback to `world_evaluator.rs` when live evaluation encounters
an unsupported request.

## Related specifications

- [Mittens host and MMS runtime boundary](mittens-host-and-runtime-boundary.md)
- [Host API](host-call-api.md)
- [MeowMeowRunner](script-runner.md)

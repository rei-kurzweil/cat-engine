# Configuring a script host

Use a configured runtime when an application wants MMS to recognize a fixed
set of components or host APIs. The runtime owns the script-visible catalog;
the host implements the effects requested by evaluation.

For a builder-free runtime that only collects arbitrary structural component
trees, see [Use the standard host](use_the_standard_host.md).

## Build the runtime

The current configuration API uses `Runtime::builder()`, `ComponentSpec`, and
`HostApiSpec`:

```rust
use meow_meow_script::{
    ComponentSpec, HostApiSpec, Runtime, ValueSignature, ValueType,
};

fn configured_runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let mut builder = Runtime::builder();
    builder.register_component(
        ComponentSpec::new("Panel")
            .alias("panel")
            .constructor(
                "new",
                ValueSignature::new(vec![ValueType::Number], ValueType::Component),
            )
            .property("title", ValueType::String)
            .method(
                "show",
                ValueSignature::new(vec![], ValueType::Null),
            ),
    )?;
    builder.register_host_api(
        HostApiSpec::method(
            "telemetry",
            "record",
            ValueSignature::new(vec![ValueType::Any], ValueType::Null),
        )
        .requires("telemetry.record"),
    )?;
    Ok(builder.build())
}
```

`RuntimeBuilder` currently defaults to
`ComponentNamePolicy::StrictRegistered`. Under this policy, an unknown
component-shaped expression produces a catalog validation error. Set
`OpenUppercase` explicitly only when unregistered ASCII-uppercase names should
be accepted as unvalidated structural data.

Component declarations may currently describe:

- canonical names and aliases;
- constructors and chained builder calls;
- named and positional values;
- component methods and their signatures; and
- normalization and validation callbacks.

Host API declarations may describe free functions or namespaced calls such as
`telemetry.record(...)`.

## Implement the host

Implement `Host::dispatch_with_context` to service `HostRequest` values. Use
the supplied `HostContext` to allocate synthetic component handles or validate
handles returned earlier in the same session.

```rust
use meow_meow_script::{
    Host, HostCapabilities, HostContext, HostError, HostRequest, HostResponse,
    TransportValue,
};

struct AppHost;

impl Host for AppHost {
    fn capabilities(&self) -> HostCapabilities {
        HostCapabilities::default()
            .supports_component("Panel")
            .supports_api("telemetry.record")
    }

    fn dispatch_with_context(
        &mut self,
        context: &mut HostContext,
        request: HostRequest,
    ) -> Result<HostResponse, HostError> {
        match request {
            HostRequest::Emit { tree } => {
                let handle = context.allocate_component();
                Ok(HostResponse::Component {
                    handle,
                    component_type: tree.component_type,
                })
            }
            HostRequest::CallApi { api_id, args }
                if api_id == "telemetry.record" =>
            {
                send_telemetry(args)?;
                Ok(HostResponse::Unit)
            }
            other => Err(HostError::unsupported(other.operation_name())),
        }
    }
}

fn send_telemetry(_values: Vec<TransportValue>) -> Result<(), HostError> {
    Ok(())
}
```

Return a typed unsupported error for requests the host cannot implement. Do
not silently return `Unit` or `Null` for an unsupported operation.

Values crossing a host API use `TransportValue`. Tables become owned
snapshots, component values become opaque `ComponentHandle`s, and callbacks
become opaque `CallbackHandle`s. A host should not retain MMS heap objects or
closure bodies.

The current API only gives components privileged live receiver identity. The proposed general
model for non-component resources and component-bound sub-receivers is documented in
[Host values, resources, and bound receivers](../draft/host-values-resources-and-bound-receivers.md).

## Create a session and evaluate

The current synchronous session API temporarily requires a capability summary
from the host:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = configured_runtime()?;
    let host = AppHost;
    let mut session = runtime.session(host)?;
    session.eval("Panel.new(320) { title = \"Inventory\" }")?;
    Ok(())
}
```

A real configured host must currently override `Host::capabilities()` and
advertise each registered component, required component operation, and host
API. See
[`EventStreamHost`](../../src/example_hosts.rs) and the complete
[`event_stream_host` example](../../examples/event_stream_host.rs).

## Current migration note

`HostCapabilities`, the flat `RuntimeBuilder`, and the permanently host-owned
`Session<H>` are transitional APIs. The ownership cutover will replace them
with the nested `RuntimeSpec` builder, opaque operation bindings, and a
host-independent session client. New integrations should keep configuration
and dispatch code close together so that migration does not leave a second
catalog in the host.

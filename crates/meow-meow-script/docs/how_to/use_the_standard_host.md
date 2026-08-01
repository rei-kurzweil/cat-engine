# Use the standard host

`StandardHost` is the crate-owned host for standalone MMS evaluation. It is
useful for tests, tools, generators, and applications that want evaluated
component data without linking `mittens-engine`.

It currently provides:

- `Runtime::standard()` with `ComponentNamePolicy::OpenUppercase`;
- collection of emitted component roots;
- local opaque handles for registered trees;
- validated, ordered attachment records;
- lookup of collected trees by handle; and
- resolution of registered attachments into ordinary nested component trees.

It deliberately does not emulate an engine. Queries, component methods, host
APIs, audio operations, and engine mutations return typed unsupported errors.
Filesystem source loading and MMS reflection methods are planned but are not
implemented yet.

## Run an MMS component tree

`Runner::standard()` constructs the standard runtime, session, and host:

```rust
use meow_meow_script::{CeChild, Runner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = Runner::standard()?;
    runner.eval(
        r#"
        Showcase {
            title = "Standalone MMS"
            Header { "hello" }
            Content { "world" }
        }
        "#,
    )?;

    let roots = runner.host().resolved_roots()?;
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].tree.component_type, "Showcase");
    assert!(matches!(
        &roots[0].tree.children[0],
        CeChild::Spawn(child) if child.component_type == "Header"
    ));
    Ok(())
}
```

The component names are arbitrary structural labels matching ASCII
`[A-Z][A-Za-z0-9_]*`. Because they are not registered, MMS preserves their
constructor data, authored fields, positionals, and children without applying
a component-specific schema or implementation.

An ordinary nested expression is fully materialized before the host receives
one `HostRequest::Emit`:

```text
Showcase
├── Header
└── Content
```

Nested components do not produce separate `Attach` requests. Their hierarchy
is already present as `CeChild::Spawn` entries in the emitted tree.

## Inspect the host from Rust

The host exposes two levels of inspection:

- `roots()` returns raw trees received through `Emit`;
- `registered()` returns all trees assigned local handles;
- `attachments()` returns the ordered attachment graph;
- `component(handle)` looks up a collected tree; and
- `resolved_roots()` returns forest roots with handle references recursively
  replaced by nested component trees.

Prefer `resolved_roots()` when exporting, rendering, serializing, or testing
the final generic component forest. Use the raw collections when debugging
the host protocol or preserving handle identity matters.

Detached registered components are intentionally absent from
`resolved_roots()`. An attachment with `parent: None` promotes a registered
component to a root. An attachment with a parent places it below that parent.
The host rejects foreign handles, missing collected trees, reparenting, and
attachment cycles.

## What MMS can trigger today

The current crate evaluator sends `Emit` for a component expression in
statement position. It does not yet generate `Register` or standalone
`Attach` requests from MMS source.

Consequently, this source:

```mms
let header = Header {}
Scene { header }
```

currently keeps `header` as a materialized `ComponentExpr` in the MMS session
and inserts a fresh `CeChild::Spawn` into `Scene`. The final hierarchy is
structurally correct, but it does not yet exercise identity-bearing detached
registration.

The intended later lifecycle is:

1. materialize and register `Header`, obtaining a local handle;
2. keep it detached from every forest root;
3. materialize `Scene` with `CeChild::Attach(header_handle)`; and
4. resolve or attach that handle below `Scene`.

The `StandardHost` register/attach protocol is implemented and tested, but
that evaluator cutover remains pending.

## Unsupported operations

For example, this produces `EvalError::Host` with
`HostErrorKind::UnsupportedHostOperation`:

```rust
use meow_meow_script::Runner;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = Runner::standard()?;
    let error = runner.eval("query(\"#engine-component\")").unwrap_err();
    println!("{error}");
    Ok(())
}
```

Use a configured custom host when the script needs queries, methods, APIs, or
other application effects. See
[Configuring a script host](configuring_a_script_host.md).

The runnable source is in the
[`standard_runtime` example](../../examples/standard_runtime.rs):

```sh
cargo run -p meow-meow-script --example standard_runtime
```

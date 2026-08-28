# RuntimeSpec component-body control flow drops generated children

Date: 2026-08-28

Status: open / confirmed

## Summary

The RuntimeSpec evaluator used by `cargo run -- load` does not materialize component children
produced by control flow inside an ordinary component body. A parent containing `for`, `if`, or
other non-expression statements may be created without the children that those statements should
produce.

This can look like a LayoutSystem or renderer failure. In the confirmed layout reproduction, every
`LayoutRoot` exists, but it has zero children, computes zero height, and never creates background
quads or registers nested renderables.

The legacy evaluator does materialize the same source correctly.

## Minimal reproduction

Evaluate through `MeowMeowRunner::eval_with_runtime_spec_at_path` or run an MMS file containing:

```mms
LayoutRoot {
    available_width(6.0)
    available_height(2.0)
    unit_scale(0.8)

    for item in range(4) {
        T {
            Style {
                display("inline-block")
                width(1.0)
                height(1.0)
                background_color([0.02, 0.85, 0.95, 0.85])
            }
        }
    }
}
```

Expected:

- the `LayoutRoot` has four transform children;
- four `StyleComponent`s exist;
- LayoutSystem computes a nonzero height;
- four generated `__bg` quads appear.

Actual:

- the `LayoutRoot` has zero children;
- no nested `StyleComponent`s or `__bg` quads exist;
- computed height is `-0.0`;
- the layout is marked clean after processing the empty root.

The larger reproduction is
`examples/planar-auto-transparency-optimization.mms`, whose nested loops should produce 12 rows and
144 styled cells.

## Confirmed failure boundary

`crates/meow-meow-script/src/evaluator.rs` materializes direct component expressions and builder
calls into a `MaterializedCE`. When it encounters a component-body statement outside its directly
handled set, it stores the whole body as `deferred_block` and stops ordinary child materialization.

That representation is meaningful for imperative body owners such as keyframes. The Mittens host
does not, however, execute a deferred block to populate an ordinary structural component such as
`LayoutRoot`. The parent is therefore spawned without the control-flow-generated children.

This is broader than layout and broader than `for`. Audit every statement currently handled by the
materializer's fallback, including:

- `for` and nested `for`;
- `if` / `else`;
- `while`;
- lexical assignments used by structural control flow;
- blocks and component-producing factory calls within those constructs;
- `break`, `continue`, and ordering around statements before and after control flow.

Relevant code:

- `crates/meow-meow-script/src/evaluator.rs`
- `crates/meow-meow-script/src/object.rs`
- `src/scripting/host.rs`
- `src/scripting/configured_registry.rs`
- `src/scripting/component_registry.rs`
- `src/scripting/runner.rs`

## Possible solution directions

- Add a component-body materialization sink that eagerly evaluates structural control flow and
  appends resulting `Spawn` and `Attach` children in authored order.
- Distinguish structural component bodies from intentionally deferred imperative bodies in the
  runtime/component specification instead of treating all unsupported statements as deferred.
- If deferred structural bodies remain supported, define a host operation that executes them with
  an explicit parent attachment scope before the parent subtree is initialized.

The solution must not execute imperative blocks twice or change keyframe callback semantics.

## Validation

- Add a focused `meow-meow-script` runtime test for a component containing a `for` loop.
- Add cases for nested loops, conditionals, empty iterations, factories returning
  `ComponentObject`s, and mixed static/dynamic children while preserving authored order.
- Add a Mittens integration test using `eval_with_runtime_spec_at_path`; do not rely only on the
  legacy evaluator.
- Assert the minimal layout root has four children and four styles before LayoutSystem runs.
- Tick LayoutSystem and assert nonzero computed height plus four generated backgrounds.
- Re-run `planar-auto-transparency-optimization.mms` and assert 12 rows, 144 cells, 144 `__bg`
  renderables, and 144 matching `VisualWorld` instances.
- Confirm keyframe and other intentionally deferred component bodies retain their current behavior.

## Relationship to transparency work

This bug prevents the planar benchmark from reaching transparency classification at all. Fix and
validate this evaluator issue before using that benchmark to judge single-layer versus multi-layer
transparency. The underlying transparency-order work remains separate.

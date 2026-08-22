# MMS component-reference provenance and live-handle invariant

## Goal

Define, test, and preserve when an MMS reference is a live `ComponentObject`
versus a deferred `ComponentExpr`. This is an evaluator/runtime correctness
task, not a request for authors to add defensive type checks.

Related language ergonomics task:
[MMS live component handles, deferred component expressions, and diagnostics](mms-live-component-handles-and-pattern-matching.md).

The superseding language-invariant and eye-tracking validation task is
[MMS: expose deferred component templates only through `import ast`](mms-componentexpr-only-via-import-ast.md).

## The two primary scenarios

### 1. Live scene binding: must remain a `ComponentObject`

In a normal scene evaluated against a live ECS world, a top-level binding
registers immediately, even before the object is attached into the authored
tree:

```mms
let marker = T.position(0.0, 1.0, -1.0) { R.cube() {} }

T { marker }

on(source, "Event", fn(event) {
    marker.update_transform([0.0, 1.0, -1.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
})
```

Expected provenance:

```text
T expression → Register → ComponentObject(detached) → Attach under parent
           → same ComponentObject captured by callback
```

The variable must retain the same component identity before and after the
`T { marker }` attachment. A callback seeing `ComponentExpr` here is an
invariant violation.

### 2. Intentional deferred expression: may remain a `ComponentExpr`

Component expressions must remain deferred where there is no live-world
registration context, or where the source deliberately represents a reusable
tree rather than a spawned object. Typical examples include exported component
factories/modules and detached REPL/navigation inspection:

```mms
export fn make_marker() {
    return T { R.icosahedron() {} }
}
```

Expected provenance:

```text
component factory / detached evaluation → ComponentExpr(MaterializedCE)
                                          → explicit spawn/register → ComponentObject
```

Only after an explicit materialization step can code invoke live-world methods
such as `update_transform`, `set_color`, or `look_at`.

## Observed regression

`examples/eye-tracking-example.mms` attached visual transforms successfully,
but an `XrEyeTrackingUpdated` callback later received the marker binding as a
`ComponentExpr`. `marker.update_transform(...)` failed with:

```text
receiver is not a ComponentObject, got ComponentExpr(MaterializedCE { ... })
```

The OpenXR and UDP eye-tracking systems remained active. The failure is in
MMS evaluation, binding, attachment, or closure capture provenance.

## Investigation checklist

1. Trace `maybe_register_live_component_value` for top-level assignment,
   assignment inside scene/component bodies, and callback closure capture.
2. Confirm whether `T { marker }` attaches the registered ID or evaluates a
   stale deferred copy in every relevant evaluator/runtime path.
3. Verify closure snapshots retain the post-registration `ComponentObject`,
   not a pre-registration `ComponentExpr`.
4. Build tests covering the two scenarios above in both legacy and configured
   RuntimeSpec execution paths.
5. Make component registration/attachment failure explicit; no silent fallback
   from an expected live binding to a deferred expression.

## Acceptance criteria

- A top-level live `let` has one stable component ID through attachment and
  event callback execution.
- A callback can mutate that object after attachment without re-querying it.
- Intentional detached/factory evaluation still returns `ComponentExpr` until
  explicitly materialized.
- Tests prove both paths and prevent the eye-tracking reproduction from
  regressing.

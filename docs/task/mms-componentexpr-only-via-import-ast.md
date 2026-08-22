# MMS: expose deferred component templates only through `import ast`

## Goal

Make a deferred `ComponentExpr(MaterializedCE)` an implementation detail of
MMS evaluation except where an author explicitly requests one with `import
ast`. In ordinary live MMS code, every component-producing expression that can
escape into author-visible state must become a live `ComponentObject`.

This is the required resolution of the component-reference provenance problem
seen in [`examples/eye-tracking-example.mms`](../../examples/eye-tracking-example.mms).
It is not a request to add a scene-loaded event or author-side type guards.

## Normative language rule

```text
component-producing expression in ordinary live MMS
    -> instantiate/register -> ComponentObject

component export selected with `import ast`
    -> preserve deferred MaterializedCE -> ComponentExpr
```

`MaterializedCE` remains internal evaluation data. It may exist temporarily
while evaluating a component expression, but it must not escape to ordinary
MMS bindings, returns, callback captures, or ordinary imports.

The sole author-facing way to receive a reusable deferred template is:

```mms
import ast { avatar, 0 as root } from "avatar.mms"
```

## Required promotion boundaries

Promote a `ComponentExpr` to a detached live `ComponentObject` before it can
escape through any ordinary live boundary:

- `let x = T { ... }` and reassignment;
- direct function/factory returns, including nested closures;
- ordinary named and positional module imports;
- values captured by `on(...)` handlers and other runtime closures;
- top-level component output;
- component-body children: attach an existing `ComponentObject`; instantiate a
  template only when it originated from `import ast`.

An imported function is still a function value. Each ordinary `factory()` call
must execute its body and yield a fresh live component identity.

`import ast` must reject scalar, table, and function exports with a clear
error, and must not register, attach, or initialize an ECS component.

## Non-goals

- Do not use a global `ContentLoaded`/scene-loaded event to defer eye-tracking
  callbacks. The handler owns direct references to its own visual components
  and must be able to mutate them as soon as an eye event arrives.
- Do not require authors to query their own components again or branch on an
  implementation value kind to work around incorrect capture provenance.
- Do not describe `MaterializedCE` as an AST node or a live component object.

## Implementation work

1. Audit both the legacy Mittens evaluator and `meow-meow-script` RuntimeSpec
   evaluator for every component-producing expression and closure boundary.
2. Ensure ordinary module instances cache direct component export identities;
   cache function values but not factory call results.
3. Keep the Rust-only template/materialization APIs explicit and separate from
   live instantiation APIs.
4. Update lifecycle documentation and error text so a deferred template is
   explained as an explicit `import ast` value, never as an accidental normal
   scene binding.

## Validation gate: eye tracking

The completion gate is the real scene, not only evaluator unit tests.
Restore the eye-event handler in
[`examples/eye-tracking-example.mms`](../../examples/eye-tracking-example.mms)
so that `XrEyeTrackingUpdated` updates both eye visualization squares and the
translation, direction, and pupil-size renderables/markers.

The scene must run against OpenXR/ALVR eye data with all of the following:

- both eye visuals update from received events;
- the color/translation/direction/pupil-size visualizations remain visible and
  update as authored;
- no handler error contains `receiver is not a ComponentObject` or exposes a
  `ComponentExpr(MaterializedCE ...)` for an ordinary scene binding;
- each callback mutates the same live component identities that were attached
  into the scene graph.

## Automated acceptance coverage

- Parser/unparser tests for ordinary and `import ast` named, aliased,
  positional, and mixed imports.
- Legacy and RuntimeSpec tests proving direct bindings, factory returns, and
  callback captures are live `ComponentObject`s.
- Tests proving repeated ordinary imports preserve a direct-export identity,
  while two factory calls yield distinct identities.
- Tests proving `import ast` is deferred and rejects non-component exports.
- A regression fixture mirroring the eye-tracking scene: attach two marker
  handles, invoke its eye-event callback, and verify both live handles receive
  their mutations.

# Dynamic component attachment from MMS handlers

Date: 2026-08-25
Status: supporting design note; no source changes proposed here.

## Decision needed

An MMS HTTP handler can already create component trees and cause world
topology changes, but the author needs an explicit attachment action.  A
function returning a component value does **not** by itself say where that
value belongs.  Treating every handler return as an implicit world-root emit
would make callbacks context-sensitive and turn an ordinary return value into
a side effect.

The recommended direction is an explicit free-function attachment vocabulary,
not reactive-stream or pipe semantics for ordinary functions:

```mms
attach(child)          // attach child as a world-forest root
attach(parent, child)  // attach child below parent
```

Both forms should return `null` and enqueue their topology mutation in the
same way as existing component methods.  The one-argument form is exactly the
explicit spelling for `parent: None`; it should not mean an ambient or hidden
handler-emission context.

## What works today

The existing live-component method surface provides the imperative operation:

```mms
let mount = T { name = "bars" }
mount

fn make_bar(value) {
    return T { /* bar subtree */ }
}

on(server, "HttpRequest", fn(req) {
    let bar = make_bar(10)
    mount.attach(bar)
})
```

In a live session, binding a component expression registers a detached live
`ComponentObject`; `mount.attach(bar)` then queues the attachment.  The engine
also exposes `detach`, `remove_child`, and `remove_subtree`.  For rolling-window
visuals, prefer removal over bare detachment so the old subtree is actually
cleaned up rather than left orphaned.

The current evaluator also promotes an explicit function return that is a
component expression into a live component object at the call boundary.  Thus
`mount.attach(make_bar(10))` is valid when `make_bar` uses an explicit
`return`; the local binding form remains clearer in examples because it makes
the allocation/attachment lifecycle visible.

There is no current free builtin named `attach` with either arity in the
configured MMS runtime.  The established API is the receiver method
`parent.attach(child)`.  A bare component expression in statement position is
handled as emission, but that is not an ergonomic, explicit answer to a
handler returning a component tree.

## Why handler-return auto-attach is not recommended

This speculative alternative would make a callback's return value implicitly
attach to an ambient handler scope or the world root:

```mms
on(server, "HttpRequest", fn(req) {
    return make_bar(10) // implicit attachment proposed by this alternative
})
```

It leaves important questions with surprising answers:

* Is the tree attached under `server`, under the handler registration scope, or
  at the world root?
* How does a handler intentionally return a component for later use without
  spawning it?
* Does a helper called from a handler acquire the same hidden emission context?
* If several event sources call the same function, which context owns its
  output?

Reactive-stream or pipe syntax does not remove those ownership questions; it
only moves them into an implicit subscription/emission context.  For a scene
graph, an explicit parent is valuable information.

## Proposed builtin contract

`attach` should accept either a detached `ComponentObject` or a component
expression returned from a factory.  When it receives an expression, the
runtime should register/spawn it first, then enqueue the attach.  That makes
the common form concise without conflating normal function returns with emits:

```mms
attach(make_bar(value))       // register then attach as a root
attach(chart_mount, make_bar(value)) // register then attach under mount
```

For an already-live object, it only enqueues the attach:

```mms
let bar = make_bar(value)
attach(chart_mount, bar)
```

Required semantics:

1. `attach(child)` attaches the child as a world root and initializes it.
2. `attach(parent, child)` validates both live handles, rejects cycles and
   invalid reparenting, and initializes the attached subtree as appropriate.
3. Reattaching an already attached object must have a defined result.  The
   safe v1 choice is a typed error, rather than silent reparenting.
4. The return value is `null`; callers obtain a handle by binding the factory
   result when they need one later.
5. `remove_subtree(child)` remains the explicit lifecycle complement for
   discarding rolling-window history.

## Relationship to the bar-graph review

The preferred HTTP bar graph uses `chart_mount.attach(bar)` today.  Adding the
free builtin would make its intent more uniform in a handler:

```mms
let bar = make_bar(value)
attach(chart_mount, bar)
```

It is an ergonomic addition, not a prerequisite for event-driven visualization:
the parent receiver method already provides programmatic, imperative attachment
from a handler. It also does not solve the separate MMS gaps for the proposed
`JSON` built-in table or maintaining an ordered mutable sample list.

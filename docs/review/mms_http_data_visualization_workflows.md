# MMS-only HTTP data-visualization workflows

Date: 2026-08-24  
Status: review of the current implementation; no source changes proposed here.

## Scope and conclusion

An MMS example can own an `HttpServer`, receive each accepted request through
`on(server, "HttpRequest", fn(req) { ... })`, and update the live scene without
Rust application code.  The checked-in
[`examples/http-server-example.mms`](../../examples/http-server-example.mms)
already demonstrates the HTTP event and a live `Text.set_text` update.

Both visualization strategies considered below are possible today:

1. create and attach one visual subtree for each request; or
2. retain request-derived state and replace the visualization subtree from the
   complete retained dataset on each request.

The second strategy is viable for small demonstrations, but it lacks a native
mutable, ordered collection and a one-call child-clear operation.  That is the
main MMS authoring gap this review identifies.

## Established runtime facts

* `HttpServer.bind(address)` constructs a server; `HttpRequest` handlers receive
  `request_id`, method, path, query, headers, body text, and related metadata.
  `server.reply_text(req, status, body)` supplies the response.  See the
  [component API](../meow_meow/spec/component_api.md#http-server).
* A component expression stored in a `let` becomes a detached live component
  handle in a live MMS session.  The universal live methods `attach`, `detach`,
  `remove_child`, and `remove_subtree` are registered for every component type.
  This is exercised by
  [`examples/mesh-factory-example.mms`](../../examples/mesh-factory-example.mms)
  and [`examples/animation-for-topology.mms`](../../examples/animation-for-topology.mms).
* Callback closures retain heap-backed table identity.  Therefore a table made
  before `on(...)` can be mutated from every later callback.  MMS table keys are
  strings; table iteration is supported.  Arrays are currently value-backed and
  do not have append, deletion, or index assignment.
* Handler-issued intents are deferred until after event dispatch, then executed
  in queue order.  A handler should send its HTTP reply promptly; it should not
  depend on its new visuals having been rendered before replying.

## Scenario A: event-driven component factory

Use a stable, emitted mount component and a factory that returns one fresh
component expression.  In the request handler, bind the factory result first,
then attach that live handle to the mount.

```mms
let marks = T { name = "request_marks" }
marks

fn make_mark(req, y) {
    T.position(0.0, y, 0.0) {
        R.sphere() { C.rgba(0.25, 0.8, 1.0, 1.0) }
        Text { req.method + " " + req.path }
    }
}

let state = { next_y = 0.0 }
let server = HttpServer.bind("127.0.0.1:7000") {}
server

on(server, "HttpRequest", fn(req) {
    let mark = make_mark(req, state.next_y)
    marks.attach(mark)
    state.next_y = state.next_y - 0.35
    server.reply_text(req, 202, "accepted\n")
})
```

`let mark = ...` is significant: binding the component expression registers it
and produces the live handle required by `marks.attach(mark)`.  Do not assume
that `marks.attach(make_mark(...))` implicitly registers the factory result;
the component-method argument is validated as a component handle.

This supports an event-driven component factory today.  If the visualization
needs unbounded history, it also needs a retention policy (for example, remove
an old child before attaching a new one) to avoid unbounded scene growth.

## Scenario B: retain data, then rebuild the visualization

Use a heap-backed table as mutable state.  Store rows under string keys (an HTTP
request id can be converted with `"" + req.request_id`), preserve a row count,
remove the previously rendered children, then attach fresh factory results.

```mms
let chart_mount = T { name = "chart_mount" }
chart_mount

let state = { rows = {}, rendered = 0 }

fn make_bar(row) {
    T.position(row.x, row.y, 0.0) {
        R.cube() { C.rgba(1.0, 0.55, 0.25, 1.0) }
    }
}

fn redraw() {
    // Removing index 0 repeatedly clears the current direct-child list.
    for ignored in range(state.rendered) {
        chart_mount.remove_child(0)
    }

    let rendered = 0
    for entry in state.rows {
        let visual = make_bar(entry.value)
        chart_mount.attach(visual)
        rendered = rendered + 1
    }
    state.rendered = rendered
}

let server = HttpServer.bind("127.0.0.1:7000") {}
server
on(server, "HttpRequest", fn(req) {
    let key = "" + req.request_id
    state.rows[key] = { x = req.request_id * 0.2, y = 0.0 }
    redraw()
    server.reply_text(req, 202, "accepted\n")
})
```

This is a full redraw, not an in-place data renderer.  It is reasonable for an
MMS example and a small dataset.  It is intentionally unsuitable for a high
rate or large time-series source because it allocates and initializes a new
visual tree for every retained row after every request.

## Gaps to document or address later

1. **No mutable ordered collection.** MMS has heap-backed tables but no
   `push`/`append`, array index assignment, array removal, or ordered map.  A
   table can retain rows, but its iteration order is not a presentation-order
   contract.  Time-series/list visualizations therefore need an explicit
   sortable field and cannot express a reliable ordered traversal in MMS alone.

2. **No `clear_children()` / replace-children operation.** Rebuild requires
   keeping a rendered-child count and issuing `remove_child(0)` once per child.
   A single, deterministic `clear_children()` (or `replace_children(factory)`)
   method would make Scenario B both safer and clearer.

3. **Factory-to-attach conversion is not implicit.** A factory result needs a
   `let` binding before `parent.attach(result)`.  This is workable but non-obvious
   and should be called out in the component authoring documentation; an
   explicit `spawn(factory())` expression could be considered later.

4. **The public component API document omits topology methods.** `attach`,
   `attach_clone`, `detach`, `remove_child`, and `remove_subtree` are runtime
   methods, but the current component API page lists neither their contracts nor
   their deferred mutation timing.  Documenting them would make the MMS-only
   workflow discoverable.

## Recommended authoring choice

Start an HTTP visualization example with Scenario A when each request naturally
maps to one marker, bar, log row, or other independent visual.  Add bounded
retention explicitly.  Use Scenario B only where the visual genuinely depends
on the complete dataset and the dataset is small.  Treat ordered, growing data
and efficient whole-view replacement as MMS API work, not as Rust required for
the basic example.

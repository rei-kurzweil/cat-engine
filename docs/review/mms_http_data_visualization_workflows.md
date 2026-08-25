# MMS-only HTTP data-visualization workflows

Date: 2026-08-25
Status: review of the current implementation; no source changes proposed here.

## Scope and conclusion

An MMS example can own an `HttpServer`, receive each accepted request through
`on(server, "HttpRequest", fn(req) { ... })`, and update the live scene without
Rust application code.  The checked-in
[`examples/http-server-example.mms`](../../examples/http-server-example.mms)
already demonstrates the HTTP event and a live `Text.set_text` update.

Scenario A is the primary workflow for interactively testing MMS JSON support
without any HTTP server or replay harness:

1. synchronously read a JSON fixture file;
2. parse it into an array of integer records; and
3. create and attach one labeled bar subtree for each record.

Scenario B is the unbounded HTTP version: each POST adds one integer and one
bar. Scenario C extends it with a bounded rolling window: each POST adds a bar
and removes the historic sample/bar that falls outside the window.

The present MMS API lacks the `JSON` table and synchronous file read needed for
Scenario A. It also lacks the mutable ordered-list operations needed for
Scenario C. Those are deliberate gaps to document; this review does not hide
them behind a count-only workaround.

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
* MMS has no current JSON builtin or general synchronous file-read builtin.
  The file-backed graph below is therefore the intended authoring shape, not a
  claim that its `File.read_text` or `JSON.parse` calls run today.
* Handler-issued intents are deferred until after event dispatch, then executed
  in queue order.  A handler should send its HTTP reply promptly; it should not
  depend on its new visuals having been rendered before replying.

## Proposed `JSON` built-in table

Add one reserved built-in table, analogous to `Math`, rather than unrelated
numeric and JSON conversion functions:

| MMS call | Result | Error condition |
| --- | --- | --- |
| `JSON.parse(text)` | MMS value: table, array, string, number, bool, or `null` | invalid JSON text |
| `JSON.stringify(value)` | JSON string | functions, component handles, cycles, or other non-JSON MMS values |

`JSON.parse` maps JSON objects to heap-backed MMS tables and JSON arrays to MMS
arrays. It therefore makes the intended HTTP contract direct and readable:

```mms
let record = JSON.parse(req.body_text)
let value = record.value
```

No separate `parse_number` builtin is needed: a JSON number is already an MMS
number after `JSON.parse`. The table should have typed parse/stringify errors,
not return `null` silently on invalid input.

## Proposed `File` built-in table

Scenario A also needs one deliberately small, synchronous file API:

| MMS call | Result | Error condition |
| --- | --- | --- |
| `File.read_text(path)` | UTF-8 string | missing, unreadable, non-UTF-8, or sandbox-disallowed path |

It should be restricted to the MMS asset/project roots selected by the host; it
is not arbitrary filesystem authority. The complete JSON test input is then
plain and interactive: `JSON.parse(File.read_text(path))`.

## Scenario A: synchronous JSON-file bar graph

Use a JSON fixture such as `examples/data/bar-samples.json`, containing
`[{"value": 4}, {"value": 12}, {"value": 7}]`. The script reads it once,
parses the record array, and builds the complete graph. Each column is an
inline-block layout item so bars flow beside one another; inside it, the value
label and bar are block elements. The cube is scaled into a rectangular prism
whose height is proportional to the sample value.

`File.read_text` and `JSON.parse` below name the missing MMS operations
explicitly. They define the desired no-Rust authoring experience.

```mms
let chart = T {
    LayoutRoot {
        available_width(20.0wu)
        available_height(10.0wu)
        T {
            name = "bar_chart"
            Style {
                display("block")
                width(20.0wu)
            }
        }
    }
}
chart
let bars = query("#bar_chart")

fn make_bar(value) {
    let height = value * 0.05
    return T {
        Style {
            display("inline-block")
            width(0.5wu)
            vertical_align("bottom")
        }
        Text {
            "" + value
            Style { display("block") }
        }
        T.position(0.0, height / 2.0, 0.0).scale(0.4, height, 0.4) {
            Style { display("block") }
            R.cube() { C.rgba(0.25, 0.8, 1.0, 1.0) }
        }
    }
}

let text = File.read_text("examples/data/bar-samples.json") // absent today
let records = JSON.parse(text)                               // absent today
for record in records {
    let value = record.value
    let bar = make_bar(value)
    bars.attach(bar)
}
```

`let bar = ...` makes the allocation and attachment lifecycle clear: binding
the component expression registers it and produces the live handle required by
`bars.attach(bar)`.  A factory with an explicit `return` is also promoted to a
live component object at its call boundary, so `bars.attach(make_bar(value))`
is a valid shorter form in the current evaluator.

The current engine supports the live-component part of this flow: bind a
factory result, then attach it. Scenario A does not need mutable runtime state,
HTTP, or a client launcher. It is the first interactive test for the `JSON`
and `File` built-in tables.

## Scenario B: unbounded, event-driven HTTP bar graph

Each accepted POST parses one integer record, appends it to the logical sample
list, and attaches one labeled bar. This is intentionally unbounded and is the
direct HTTP counterpart to Scenario A.

```mms
let bars = T {
    name = "bar_chart"
    LayoutRoot { available_width(20.0wu) available_height(10.0wu) }
}
bars

let state = {
    samples = []
}

fn make_bar(value) {
    let height = value * 0.05
    return T {
        Style { display("inline-block") width(0.5wu) vertical_align("bottom") }
        Text { "" + value Style { display("block") } }
        T.position(0.0, height / 2.0, 0.0).scale(0.4, height, 0.4) {
            Style { display("block") }
            R.cube() { C.rgba(1.0, 0.55, 0.25, 1.0) }
        }
    }
}

let server = HttpServer.bind("127.0.0.1:7000") {}
server

on(server, "HttpRequest", fn(req) {
    if req.method != "POST" {
        server.reply_text(req, 405, "POST only\n")
        return
    }

    let record = JSON.parse(req.body_text)  // required JSON table; absent today
    let value = record.value
    state.samples.push(value)               // required mutable-list API; absent today
    let bar = make_bar(value)
    bars.attach(bar)

    server.reply_text(req, 202, "accepted\n")
})
```

Every request retains its sample and bar. The chart is therefore useful for
proving the handler/factory/attachment path, but its world and sample list grow
without bound.

## Scenario C: rolling-window, event-driven HTTP bar graph

Use Scenario B's chart, server, and `make_bar` factory, but give its state a
window size and remove the oldest sample and visual after adding each new one.
The list and the direct child order remain oldest-to-newest.

```mms
let state = { samples = [], window_size = 32 }

on(server, "HttpRequest", fn(req) {
    if req.method != "POST" {
        server.reply_text(req, 405, "POST only\n")
        return
    }

    let record = JSON.parse(req.body_text)  // required JSON table; absent today
    let value = record.value
    state.samples.push(value)               // required mutable-list API; absent today
    let bar = make_bar(value)
    bars.attach(bar)

    if len(state.samples) > state.window_size {
        state.samples.remove(0)             // required mutable-list API; absent today
        // Child 0 is the oldest visual; removal schedules subtree cleanup.
        bars.remove_child(0)
    }

    server.reply_text(req, 202, "accepted\n")
})
```

The attach and removal intents are deferred but maintain handler order. At
capacity, the new bar is attached and then the existing child at index zero is
removed, leaving `window_size` retained samples and live visual children.

The `LayoutRoot`/`Style` design needs an implementation check when the example
is added: the chart mount must lay its `inline-block`, `0.5wu`-wide columns out
horizontally, while the label and bar inside each column lay out vertically with
their baseline/bottom alignment behaving as intended.

## Gaps to document or address later

1. **No mutable ordered collection.** MMS has heap-backed tables and a `len`
   builtin, but no `push`/`append`, array index assignment, array removal, or
   ordered map. The desired `state.samples` list in Scenarios B and C cannot
   currently be implemented faithfully.  A table can retain rows, but its
   iteration order is not a presentation-order contract.

2. **No `JSON` or `File` built-in table.** MMS needs typed `JSON.parse(text)`,
   `JSON.stringify(value)`, and restricted `File.read_text(path)`. These are
   required for Scenario A, and `JSON.parse` is required before a posted record
   can drive a Scenario B/C bar height and label without Rust.

3. **The public component API document omits topology methods.** `attach`,
   `attach_clone`, `detach`, `remove_child`, and `remove_subtree` are runtime
   methods, but the current component API page lists neither their contracts nor
   their deferred mutation timing.  Documenting them would make the MMS-only
   workflow discoverable.

## Recommended authoring choice

Build the first example around Scenario A: a synchronous JSON-file bar graph.
It exercises `File.read_text`, `JSON.parse`, JSON-table field access, layout,
factory returns, and attachment without introducing HTTP timing or a client
harness. Follow it with Scenario B's unbounded per-POST factory flow, then
Scenario C's bounded rolling list and matching bounded world subtree.

### Footnote: handler-return attachment context

See [Dynamic component attachment from MMS handlers](mms_dynamic_component_attachment_gap.md)
for the related question of whether a handler's returned component tree should
be implicitly attached, and for a proposed explicit free-function vocabulary:
`attach(child)` for a world root and `attach(parent, child)` for a child.

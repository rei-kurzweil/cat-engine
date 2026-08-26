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

The Mittens runtime now provides the host-advertised `JSON` namespace needed
for JSON parsing and encoding. It still lacks the synchronous file read needed
for Scenario A and the mutable ordered-list operations needed for
Scenario C. Those are deliberate gaps to document; this review does not hide
them behind a count-only workaround.

## Planned example files

Before implementing the MMS or Mittens changes, reserve one MMS example per
scenario under `examples/`:

| Scenario | Planned file | Purpose | Prerequisites |
| --- | --- | --- | --- |
| A | `examples/data-viz-json-file.mms` | Read a JSON fixture synchronously and build the complete labeled bar graph. | restricted `File` host API |
| B | `examples/data-viz-http-unbounded.mms` | Accept JSON POSTs and append one labeled bar per request. | mutable ordered sample list |
| C | `examples/data-viz-http-rolling-window.mms` | Accept JSON POSTs while retaining only the newest N samples and bar subtrees. | Scenario B prerequisite plus ordered removal |

These are planned filenames only. Do not add placeholder examples before their
required MMS surfaces exist; when implemented, each should be independently
launchable and should demonstrate only its named scenario.

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
* `JSON.parse` and `JSON.stringify` are registered Mittens host APIs. MMS does
  not implement a third-party JSON codec itself. MMS still has no general
  synchronous file-read host API, so the file-backed graph below remains the
  intended authoring shape rather than a claim that `File.read_text` runs today.
* Handler-issued intents are deferred until after event dispatch, then executed
  in queue order.  A handler should send its HTTP reply promptly; it should not
  depend on its new visuals having been rendered before replying.

## Mittens `JSON` host namespace

Mittens registers one `JSON` host namespace through its `RuntimeSpec`; MMS
does not reserve or implement this third-party codec itself:

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

No separate `parse_number` API is needed: a JSON number is already an MMS
number after `JSON.parse`. The host API returns typed parse/stringify errors,
not `null` silently on invalid input.

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

`File.read_text` below names the remaining missing MMS host operation. Together
with the implemented `JSON.parse`, it defines the desired no-Rust authoring
experience.

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
let records = JSON.parse(text)
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
HTTP, or a client launcher. It is the first interactive test for the `File`
host API alongside JSON parsing.

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

    let record = JSON.parse(req.body_text)
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

    let record = JSON.parse(req.body_text)
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

2. **No restricted `File` host API.** `JSON.parse(text)` and
   `JSON.stringify(value)` are implemented by Mittens' advertised `JSON`
   namespace. `File.read_text(path)` remains required for Scenario A; it must
   be restricted to host-selected asset/project roots.

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

// data-viz-http-rolling-window.mms
//
// POST {"value": 12} to http://127.0.0.1:7000/ to retain a rolling chart.
// Requires: JSON.parse(text), mutable array push/remove, and len(array).

RendererSettings { window_size(960, 720) }
BGC.rgba(0.04, 0.05, 0.09, 1.0)
AL.rgb(0.48, 0.48, 0.54)
T.position(0.0, 2.4, 7.5) { C3D { Pointer {} } }

let chart = T.position(-2.8, 1.8, 0.0).scale(0.12, 0.12, 0.12) {
    LayoutRoot {
        available_width(46.0)
        available_height(28.0)
        T {
            Style {
                display("block") width(46.0) height(28.0) padding(2.0)
                background_color = [0.09, 0.11, 0.20, 0.96]
            }
            Text { "HTTP bar chart — newest 8 samples" C.rgba(0.88, 0.94, 1.0, 1.0) }
            T {
                Style { display("block") margin_bottom(1.5) }
                Text { "POST {\"value\": 12} to 127.0.0.1:7000" C.rgba(0.55, 0.70, 0.92, 1.0) }
            }
            T { name = "bar_chart" Style { display("block") width(42.0) height(20.0) } }
        }
    }
}
chart

let bars = chart.query("#bar_chart")
let state = { samples = [], window_size = 8 }

fn make_bar(value) {
    let height = value * 0.7
    return T {
        Style { display("inline-block") width(4.5) vertical_align("bottom") }
        Text { "" + value C.rgba(0.90, 0.96, 1.0, 1.0) Style { display("block") } }
        T.position(0.0, height / 2.0, 0.0).scale(1.5, height, 1.5) {
            Style { display("block") }
            R.cube() { C.rgba(0.30, 0.92, 0.62, 1.0) }
        }
    }
}

let server = HttpServer.bind("127.0.0.1:7000") {}
server

on(server, "HttpRequest", fn(req) {
    if req.method != "POST" {
        server.reply_text(req, 405, "POST only\\n")
        return
    }

    let record = JSON.parse(req.body_text)
    let value = record.value
    state.samples.push(value)
    bars.attach(make_bar(value))

    if len(state.samples) > state.window_size {
        state.samples.remove(0)
        bars.remove_child(0)
    }

    server.reply_text(req, 202, "accepted\\n")
})

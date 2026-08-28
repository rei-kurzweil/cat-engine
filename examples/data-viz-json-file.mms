// data-viz-json-file.mms
//
// Build a labeled bar chart from a local JSON fixture. This is the intended
// authoring shape for the restricted File and JSON built-in tables.
//
// Requires: File.read_text(path), JSON.parse(text).
import { star_kawaii_background } from "../assets/components/backgrounds/star_kawaii_background.mms"


RendererSettings { window_size(960, 720) }
RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.045) half_res(true) } }
    Bloom { intensity(0.8) emissive_scale(1.5) }
}

BGC.rgba(0.15, 0.15, 0.15, 1.0)
AL.rgb(0.48, 0.48, 0.54)

// Desktop fly camera: WASD/RF movement, right-mouse look, Q/E roll.
I {
    speed(2.5)
    InputTransformMode.forward_z() {
        roll_axis_y()
    }
    T.position(0.0, 2.4, 7.5) { C3D { Pointer {} } }
}

// my boy, this peace is what all true warriors strive for looop for

BG.occlusion_and_lighting() {
    star_kawaii_background([1.0, 0.9, 0.8, 1.0])
}

let chart = T.position(-2.8, 1.8, 0.0).scale(0.12, 0.12, 0.12) {
    LayoutRoot {
        available_width(36.0)
        available_height(28.0)
        // Diagnostic overlay: content, padding, and margin boxes for each
        // layout-managed item. Remove this once the bar baseline is verified.
        //InspectLayout {}

        T {
            Style {
                display("block")
                width(46.0)
                height(28.0)
                padding(2.0)
                background_color = [0.09, 0.11, 0.20, 0.96]
            }
            T.position(0.0, 0.0, 0.2) {
                Style { display("block") margin_bottom(0.45) font_size(1.35) }
                T.position(0.0, 0.0, 0.15) {
                    Text {
                        "not JSON file bar chart"
                        C.rgba(0.88, 0.94, 1.0, 1.0)
                        EM.on() { intensity(0.7) }
                    }
                }
            }
            T.position(0.0, 0.0, 0.2) {
                Style { display("block") margin_bottom(1.5) font_size(0.8) }
                T.position(0.0, 0.0, 0.15) {
                    Text { "examples/data/bar-samples.json" C.rgba(1.0, 1.0, 1.0, 1.0) }
                }
            }
            T {
                name = "bar_chart"
                Style {
                    display("block")
                    width(42.0)
                    height(20.0)
                    background_color = [0.12, 0.16, 0.28, 0.72]
                    // Keep this diagnostic chart-region quad behind its own
                    // content but ahead of the enclosing panel background.
                    background_z(-0.01)
                }
            }
        }
    }
}
chart

let bars = chart.query("#bar_chart")

fn make_bar(value) {
    let height = value * 0.7
    return T {
        Style {
            display("inline-block")
            width(4.5)
            vertical_align("bottom")
            // Temporary box contrast while validating visual placement.
            background_color = [0.20, 0.26, 0.40, 0.32]
            background_z(-0.01)
        }
        Text {
            "" + value
            C.rgba(1.0, 1.0, 1.0, 1.0)
            Style { display("block") }
        }
        // Bounds-derived placement belongs to layout; authors specify the
        // data-driven scale, not half-width/half-height translations.
        T.scale(1.5, height, 1.5) {
            R.cube() { C.rgba(1.0, 1.0, 1.0, 1.0) }
        }
    }
}

let text = File.read_text("examples/data/bar-samples.json")
let records = JSON.parse(text)
for record in records {
    bars.attach(make_bar(record.value))
}

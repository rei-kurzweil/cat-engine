// Floating transform telemetry. The target is measured independently from this
// panel's own transform, so the panel can be dragged or XR-grabbed freely.

let PANEL_WIDTH_GU = 19.0
let PANEL_UNIT_SCALE = 0.035
let PANEL_TITLE_HEIGHT_GU = 3.0
let PANEL_UPDATE_INTERVAL_SEC = 0.1

fn fixed_5(value) {
    let scaled = Math.round(Math.abs(value) * 100000.0)
    let whole = Math.floor(scaled / 100000.0)
    let fraction = scaled - whole * 100000.0
    let padding = ""

    if fraction < 10.0 {
        padding = "0000"
    } else if fraction < 100.0 {
        padding = "000"
    } else if fraction < 1000.0 {
        padding = "00"
    } else if fraction < 10000.0 {
        padding = "0"
    }

    let sign = ""
    if value < 0.0 && scaled > 0.0 {
        sign = "-"
    }
    return sign + whole + "." + padding + fraction
}

fn coordinate_row(label, initial_value) {
    return Text {
        label + ": " + initial_value
        C.rgba(0.82, 0.95, 1.0, 1.0)
        Style {
            display("block")
            width(100%)
            padding_xy(0.35, 0.12)
            background_color([0.035, 0.075, 0.12, 0.90])
            background_z(-0.01)
        }
    }
}

export fn transform_info_panel(target) {
    let x_text = coordinate_row("x", "0.00000")
    let y_text = coordinate_row("y", "0.00000")
    let z_text = coordinate_row("z", "0.00000")
    let refresh = { elapsed_sec = PANEL_UPDATE_INTERVAL_SEC }

    let panel = T {
        name = "transform_info_panel"
        Grabbable {}

        T {
            name = "transform_info_panel_layout"
            LayoutRoot {
                available_width(PANEL_WIDTH_GU)
                unit_scale(PANEL_UNIT_SCALE)

                T {
                    name = "transform_info_panel_title"
                    Draggable.parent().plane("camera")
                    Raycastable.enabled() { interaction_priority(100.0) }
                    Style {
                        display("block")
                        width(100%)
                        height(PANEL_TITLE_HEIGHT_GU)
                        padding_xy(0.45, 0.20)
                        background_color([0.08, 0.36, 0.58, 0.96])
                        background_z(-0.015)
                    }
                    Text { "telemetry" C.rgba(0.97, 0.99, 1.0, 1.0) }
                }

                T {
                    name = "transform_info_panel_values"
                    Style {
                        display("flex")
                        flex_direction("column")
                        width(100%)
                        padding(0.20)
                        background_color([0.015, 0.03, 0.055, 0.88])
                        background_z(-0.02)
                    }
                    x_text
                    y_text
                    z_text
                }
            }
        }
    }

    on_global("FrameTick", fn(event) {
        refresh.elapsed_sec = refresh.elapsed_sec + event.dt_sec
        if refresh.elapsed_sec > PANEL_UPDATE_INTERVAL_SEC - 0.0000001 {
            refresh.elapsed_sec = refresh.elapsed_sec - PANEL_UPDATE_INTERVAL_SEC

            let position = target.translation()
            x_text.set_text("x: " + fixed_5(position[0]))
            y_text.set_text("y: " + fixed_5(position[1]))
            z_text.set_text("z: " + fixed_5(position[2]))
        }
    })

    return panel
}

// Floating transform telemetry. The target is measured independently from this
// panel's own transform, so the panel can be dragged or XR-grabbed freely.

// Match the authored editor-settings panel's world-space layout scale and
// visual hierarchy: neutral title chrome, a yellow content well, and pale
// padded rows. This remains standalone UI, rather than an editor panel.
let PANEL_WIDTH_GU = 16.0
let PANEL_UNIT_SCALE = 0.08
let PANEL_TITLE_HEIGHT_GU = 3.5
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

fn coordinate_row(label, text) {
    return T {
        name = "transform_info_panel_" + label
        Style {
            display("block")
            width(100%)
            margin_xy(0.25, 0.20)
            padding_xy(0.55, 0.45)
            color([0.0, 0.0, 0.0, 1.0])
            background_color([0.92, 0.97, 0.92, 1.0])
            background_z(-0.01)
            text_align("left")
            vertical_align("middle")
        }
        text
    }
}

export fn transform_info_panel(target) {
    let x_text = Text { "x: 0.00000" }
    let y_text = Text { "y: 0.00000" }
    let z_text = Text { "z: 0.00000" }
    let x_row = coordinate_row("x", x_text)
    let y_row = coordinate_row("y", y_text)
    let z_row = coordinate_row("z", z_text)
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
                        display("flex")
                        align_items("center")
                        width(100%)
                        height(PANEL_TITLE_HEIGHT_GU)
                        background_color([0.68, 0.70, 0.72, 0.98])
                        background_z(-0.01)
                    }
                    T {
                        name = "transform_info_panel_title_label_wrap"
                        Style {
                            display("flex")
                            width(0.0)
                            flex_grow(1.0)
                            height(PANEL_TITLE_HEIGHT_GU)
                            align_items("center")
                            padding(1.0)
                            color([0.08, 0.09, 0.10, 1.0])
                        }
                        T.position(0.0, 0.0, 0.02) {
                            Text { "telemetry" }
                        }
                    }
                }

                T {
                    name = "transform_info_panel_values"
                    Style {
                        display("flex")
                        flex_direction("column")
                        width(100%)
                        margin_xy(0.0, 0.3)
                        padding(0.25)
                        background_color([0.96, 0.92, 0.18, 0.80])
                        background_z(-0.001)
                    }
                    x_row
                    y_row
                    z_row
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

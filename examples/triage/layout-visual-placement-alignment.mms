// Focused repro for docs/task/style-driven-layout-visual-placement.md
//
// This scene separates two operations that currently share the
// `vertical_align` vocabulary:
//
//   1. external: align an inline-block item box within its line;
//   2. internal: align bounded visual content within a fixed item box.
//
// Expected:
//   - Scenario 1: the short orange box and two tall reference boxes share a
//     bottom edge across the full three-block row.
//   - Scenario 2: the red, green, and blue cubes appear at the top, middle,
//     and bottom of three equal white slots. Every cube is centered on X.
//
// Observed baseline:
//   - Scenario 1 works through the inline vertical-align post-pass.
//   - Scenario 2 places all three cubes slightly below and to the right of the
//     bottom-right corner of their slots. __layout_visual_placement currently
//     ignores Style and always aligns the visual AABB's bottom edge with the
//     content box's bottom edge; the right-edge overshoot additionally points
//     to a source/target coordinate-space or composition error.

RendererSettings { window_size(960, 720) }

BGC.rgba(0.055, 0.065, 0.085, 1.0)
AL.rgb(0.85, 0.85, 0.85)

I {
    speed(1.0)
    InputTransformMode.forward_z() {
        roll_axis_y()
        fps_rotation()
    }
    T.position(0.0, 1.4, 5.0) {
        C3D { Pointer {} }
    }
}

let UNIT_SCALE = 0.08
// Three 10-GU blocks plus two 1-GU gaps, with 0.5 GU row padding and 1 GU
// panel padding. The extra GU over the block/gap sum is the row padding.
let PANEL_WIDTH = 35.0
let PANEL_HEIGHT = 33.0
let WHITE = [0.94, 0.95, 0.98, 1.0]
let DARK = [0.10, 0.12, 0.17, 0.98]
let ROW = [0.16, 0.18, 0.24, 1.0]
let TEXT = [0.94, 0.96, 1.0, 1.0]

fn visual_cube(name, color) {
    return T.position(0.0, 0.0, 0.03).scale(0.12, 0.12, 0.06) {
        name = name
        R.cube() {
            C.rgba(color[0], color[1], color[2], color[3])
        }
    }
}

let panel = T {
    name = "alignment_triage_panel"
    Style {
        display("block")
        width(PANEL_WIDTH)
        height(PANEL_HEIGHT)
        padding(1.0)
        background_color(DARK)
        background_z(-0.02)
    }

    T {
        Style {
            display("block")
            // The narrowed panel wraps this heading onto two lines.
            height(4.0)
            margin_bottom(0.5)
            color = TEXT
            text_align("left")
            vertical_align("middle")
        }
        Text { "Layout visual placement: external vs internal alignment" }
    }

    T {
        Style {
            display("block")
            // Reserve two lines so wrapped text cannot overlap the row.
            height(4.0)
            margin_bottom(0.5)
            color = TEXT
            text_align("left")
            vertical_align("middle")
        }
        Text { "1. ITEM BOXES: three blocks should share a bottom edge" }
    }

    T {
        name = "scenario_1_external_line_alignment"
        Style {
            display("block")
            width(33.0)
            height(7.0)
            padding(0.5)
            margin_bottom(1.0)
            background_color(ROW)
            background_z(-0.015)
        }

        T {
            name = "short_bottom_aligned_item"
            Style {
                display("inline-block")
                width(10.0)
                height(2.0)
                margin_right(1.0)
                vertical_align("bottom")
                background_color([0.95, 0.48, 0.16, 1.0])
                background_z(-0.01)
            }
        }

        T {
            name = "tall_reference_item"
            Style {
                display("inline-block")
                width(10.0)
                height(5.0)
                margin_right(1.0)
                vertical_align("top")
                background_color([0.12, 0.72, 0.82, 1.0])
                background_z(-0.01)
            }
        }

        T {
            name = "third_reference_item"
            Style {
                display("inline-block")
                width(10.0)
                height(4.0)
                vertical_align("bottom")
                background_color([0.55, 0.34, 0.88, 1.0])
                background_z(-0.01)
            }
        }
    }

    T {
        Style {
            display("block")
            // Reserve two lines so wrapped text cannot overlap the row.
            height(4.0)
            margin_bottom(0.5)
            color = TEXT
            text_align("left")
            vertical_align("middle")
        }
        Text { "2. VISUALS: red=top, green=middle, blue=bottom inside equal white slots" }
    }

    T {
        name = "scenario_2_internal_visual_alignment"
        Style {
            display("block")
            width(33.0)
            height(8.0)
            padding(0.5)
            background_color(ROW)
            background_z(-0.015)
        }

        T {
            name = "top_visual_slot"
            Style {
                display("inline-block")
                width(10.0)
                height(6.0)
                margin_right(1.0)
                background_color(WHITE)
                background_z(-0.01)
                text_align("center")
                vertical_align("top")
            }
            visual_cube("top_visual", [0.90, 0.18, 0.20, 1.0])
        }

        T {
            name = "middle_visual_slot"
            Style {
                display("inline-block")
                width(10.0)
                height(6.0)
                margin_right(1.0)
                background_color(WHITE)
                background_z(-0.01)
                text_align("center")
                vertical_align("middle")
            }
            visual_cube("middle_visual", [0.16, 0.78, 0.34, 1.0])
        }

        T {
            name = "bottom_visual_slot"
            Style {
                display("inline-block")
                width(10.0)
                height(6.0)
                background_color(WHITE)
                background_z(-0.01)
                text_align("center")
                vertical_align("bottom")
            }
            visual_cube("bottom_visual", [0.18, 0.42, 0.94, 1.0])
        }
    }
}

Selectable.off() {
    T.position(-1.92, 2.35, 0.0) {
        Overlay {
            LayoutRoot {
                name = "layout_visual_placement_alignment_root"
                available_width(PANEL_WIDTH)
                available_height(PANEL_HEIGHT)
                unit_scale(UNIT_SCALE)

                // Keep the geometry overlay disabled: it currently interacts
                // with transparent backgrounds and obscures this comparison.
                // Set MITTENS_TRACE_LAYOUT_VISUAL_PLACEMENT=1 for console
                // source/target/transform diagnostics without overlay quads.
                // InspectLayout {}

                panel
            }
        }
    }
}

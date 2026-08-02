// accordion.mms — single-section collapsible UI molecule (=^･ω･^=)

import { accordion_down_arrow_icon } from "../icons.mms"
//
// The accordion owns its title bar, toggle handler, and body teardown.
// Its body must have one outer root named `accordion_body`.
//
// Options:
// Required options:
//   root_name        string
//   width_gu         number
//   title            string
//   title_color      rgba array
//   background_color rgba array
//   title_controls   component object (use T {} when empty)
//   title_controls_width_gu number (use 0.0 when empty)
//   body             component object rooted at #accordion_body
//
// Opening emits `AccordionRestoreRequested` on the accordion root with the
// stable body mount as its optional component payload. The owner repopulates
// the mount; the accordion does not listen for an acknowledgement.

let ACCORDION_TITLE_HEIGHT_GU = 3.5
let ACCORDION_TOGGLE_WIDTH_GU = 4.0
let ACCORDION_BODY_GAP_GU = 0.6
// Transition timing is currently beat-based. At the default 120 BPM,
// 0.6 beats is 300 ms.
let ACCORDION_TOGGLE_TRANSITION_BEATS = 0.6

export fn accordion_body(content) {
    return T {
        name = "accordion_body"
        Style {
            display("flex")
            flex_direction("column")
            width(100%)
            margin_xy(0.0, ACCORDION_BODY_GAP_GU / 2.0)
            padding(1.0)
        }
        content
    }
}

export fn accordion(options) {
    let root_name = options.root_name
    let width_gu = options.width_gu
    let title = options.title
    let title_rgba = options.title_color
    let background_rgba = options.background_color
    let title_controls = options.title_controls
    let title_controls_width_gu = options.title_controls_width_gu

    let toggle_icon = T.position(
        ACCORDION_TOGGLE_WIDTH_GU / 2.0,
        -ACCORDION_TITLE_HEIGHT_GU / 2.0,
        0.02,
    ).scale(1.62, 1.62, 1.0).rotation(0.0, 0.0, 0.0) {
        name = "accordion_toggle_icon"
        Transition {
            duration_beats(ACCORDION_TOGGLE_TRANSITION_BEATS)
            ease_out_cubic()
            capture_from_current(true)
            replace_same_target()
        }
        accordion_down_arrow_icon([0.72, 0.90, 1.0, 1.0], 1.8)
    }

    let toggle = T {
        name = "accordion_toggle"
        Raycastable.enabled() {
            interaction_priority(110.0)
        }
        Style {
            display("flex")
            width(ACCORDION_TOGGLE_WIDTH_GU)
            height(ACCORDION_TITLE_HEIGHT_GU)
            align_items("center")
            justify_content("center")
            background_color([0.12, 0.34, 0.66, 1.0])
            background_z(-0.015)
            color([0.97, 0.99, 1.0, 1.0])
        }
        toggle_icon
    }

    let body_mount = T {
        name = "accordion_body_mount"
        Style {
            display("flex")
            flex_direction("column")
            width(100%)
        }
        options.body
    }

    let panel_root = T {
        name = root_name
        Style {
            display("flex")
            flex_direction("column")
            width(width_gu)
        }

        T {
            name = "title_bar"
            Draggable.parent()
            Raycastable.enabled()
            Style {
                display("flex")
                flex_direction("row")
                align_items("center")
                width(100%)
                height(ACCORDION_TITLE_HEIGHT_GU)
                background_color(background_rgba)
                background_z(-0.01)
            }

            T {
                name = "accordion_title_slot"
                Style {
                    display("flex")
                    width(0.0)
                    flex_grow(1.0)
                    height(ACCORDION_TITLE_HEIGHT_GU)
                    align_items("center")
                    padding(1.0)
                    color(title_rgba)
                }
                T.position(0.0, 0.0, 0.02) {
                    Text {
                        name = "title_label"
                        title
                    }
                }
            }

            T {
                name = "accordion_title_controls"
                Style {
                    display("flex")
                    flex_direction("row")
                    align_items("center")
                    width(title_controls_width_gu)
                    height(ACCORDION_TITLE_HEIGHT_GU)
                }
                title_controls
            }
            toggle
        }

        body_mount
    }

    on(toggle, "Click", fn(event) {
        let current_body = body_mount.query("#accordion_body")
        if current_body {
            current_body.remove_subtree()
            toggle_icon.update_transform(
                [ACCORDION_TOGGLE_WIDTH_GU / 2.0, -ACCORDION_TITLE_HEIGHT_GU / 2.0, 0.02],
                [0.0, 0.0, -1.570796],
                [1.62, 1.62, 1.0],
            )
            emit_data(panel_root, "AccordionMinimized", body_mount)
        } else {
            toggle_icon.update_transform(
                [ACCORDION_TOGGLE_WIDTH_GU / 2.0, -ACCORDION_TITLE_HEIGHT_GU / 2.0, 0.02],
                [0.0, 0.0, 0.0],
                [1.62, 1.62, 1.0],
            )
            emit_data(panel_root, "AccordionRestoreRequested", body_mount)
        }
    })

    return panel_root
}

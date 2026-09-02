// info_panel.mms — reusable authored title/content accordion panel (=^･ω･^=)
//
// Required options:
//   root_name, width_gu, unit_scale, title, content
// Optional options:
//   icon, background_color, toggle_background_color
//
// The panel wraps `content` in its body internally. When the panel is restored
// after minimizing, the caller receives the forwarded
// AccordionRestoreRequested event and may attach a freshly built body to the
// event payload's accordion_body_mount.

import { accordion, accordion_body } from "../internal/ui/accordion.mms"

let INFO_PANEL_TITLE_HEIGHT_GU = 3.5
let INFO_PANEL_TITLE_BACKGROUND = [0.08, 0.12, 0.19, 0.98]
let INFO_PANEL_TOGGLE_BACKGROUND = [0.18, 0.42, 0.74, 1.0]
let INFO_PANEL_BODY_BACKGROUND = [0.025, 0.04, 0.07, 0.94]

fn info_panel_title(title) {
    return T {
        name = "info_panel_title"
        Style {
            display("flex")
            width(0.0)
            flex_grow(1.0)
            height(INFO_PANEL_TITLE_HEIGHT_GU)
            align_items("center")
            padding_xy(0.9, 0.3)
            color([0.88, 0.95, 1.0, 1.0])
        }
        T.position(0.0, 0.0, 0.02) { Text { title } }
    }
}

fn info_panel_icon(icon) {
    return T {
        name = "info_panel_icon"
        Style {
            display("flex")
            width(INFO_PANEL_TITLE_HEIGHT_GU)
            height(INFO_PANEL_TITLE_HEIGHT_GU)
            align_items("center")
            justify_content("center")
        }
        T.position(0.0, 0.0, 0.02) { icon }
    }
}

fn panel_with_title_children(options, title_children, title_bar_background, toggle_background) {
    // Body structure is an implementation detail of the panel, rather than a
    // second public helper callers must remember to use.
    let body = accordion_body(T {
        name = "info_panel_content"
        Style {
            display("flex")
            flex_direction("column")
            width(100%)
            padding(0.65)
            row_gap(0.35)
            color([0.90, 0.95, 1.0, 1.0])
            background_color(INFO_PANEL_BODY_BACKGROUND)
            background_z(-0.01)
        }
        options.content
    })
    return accordion({
        root_name = options.root_name
        width_gu = options.width_gu
        unit_scale = options.unit_scale
        background_color = title_bar_background
        toggle_background_color = toggle_background
        children = title_children
        body = body
    })
}

// The returned accordion root emits AccordionMinimized and
// AccordionRestoreRequested. The latter carries accordion_body_mount as its
// payload so the caller can reload dynamic content.
export fn info_panel(options) {
    let background_color = INFO_PANEL_TITLE_BACKGROUND
    let authored_background_color = options["background_color"]
    if authored_background_color { background_color = authored_background_color }

    let toggle_background_color = INFO_PANEL_TOGGLE_BACKGROUND
    let authored_toggle_background_color = options["toggle_background_color"]
    if authored_toggle_background_color { toggle_background_color = authored_toggle_background_color }

    let title = info_panel_title(options.title)
    let icon = options["icon"]
    if icon {
        return panel_with_title_children(
            options,
            [info_panel_icon(icon), title],
            background_color,
            toggle_background_color,
        )
    }
    return panel_with_title_children(
        options,
        [title],
        background_color,
        toggle_background_color,
    )
}

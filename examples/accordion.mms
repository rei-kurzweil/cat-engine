// examples/accordion.mms — reusable accordion + custom DataEvent demo
//
// Left: AccordionRestoreRequested is handled entirely in MMS.
// Right: the event carries accordion_body_mount to examples/accordion.rs,
//        which repopulates the body from native Rust code.

import { accordion, accordion_body } from "../assets/components/ui/accordion.mms"
import { star_kawaii_background } from "../assets/components/backgrounds/star_kawaii_background.mms"

RendererSettings {
    window_size(1440, 960)
}

BGC.rgba(0.008, 0.025, 0.085, 1.0)
AL.rgb(0.12, 0.16, 0.28)

RenderGraph {
    EmissivePass {
        BlurPass {
            radius_ndc(0.06)
            half_res(true)
        }
    }
    Bloom {
        intensity(0.95)
        radius_ndc(0.16)
        emissive_scale(1.2)
        half_res(true)
    }
}

star_kawaii_background([1.0, 0.9, 0.8, 1.0])

T.position(3.5, 5.5, 4.0) {
    DL { intensity(1.0) color(0.72, 0.84, 1.0) }
}
T.position(-4.0, 2.8, 1.5) {
    PL { intensity(2.2) distance(14.0) color(0.25, 0.52, 1.0) }
}
T.position(4.2, 1.8, 0.5) {
    PL { intensity(1.8) distance(12.0) color(0.74, 0.34, 1.0) }
}

I.speed(3.0) {
    InputTransformMode.forward_z() {
        fps_rotation()
        roll_axis_y()
    }
    T.position(0.0, 1.2, 5.8) {
        C3D { Pointer {} }
    }
}

fn title_label(label) {
    return T {
        name = "accordion_demo_title_label"
        Style {
            display("flex")
            width(0.0)
            flex_grow(1.0)
            height(3.5)
            padding_xy(0.7, 0.3)
            align_items("center")
            color([0.70, 0.86, 1.0, 1.0])
        }
        T.position(0.0, 0.0, 0.02) { Text { label } }
    }
}

fn title_chip(label) {
    return T {
        name = "accordion_demo_title_chip"
        Style {
            display("flex")
            width(14.0)
            height(3.5)
            padding_xy(0.7, 0.3)
            align_items("center")
            color([0.70, 0.86, 1.0, 1.0])
        }
        T.position(0.0, 0.0, 0.02) { Text { label } }
    }
}

fn body_card(heading, detail, accent) {
    return accordion_body(T {
        name = "accordion_demo_card"
        Style {
            display("flex")
            flex_direction("column")
            width(100%)
            row_gap(0.8)
            color([0.91, 0.96, 1.0, 1.0])
            background_color([0.025, 0.075, 0.18, 0.97])
            background_z(-0.01)
        }
        T.position(0.0, 0.0, 0.02) {
            name = "accordion_demo_card_heading"
            Style {
                display("block")
                width(100%)
                height(1.5)
                padding_xy(0.4, 0.2)
            }
            Text { heading }
        }
        T.position(0.0, 0.0, 0.02) {
            name = "accordion_demo_card_detail"
            Style {
                display("block")
                width(100%)
                height(2.8)
                padding_xy(0.4, 0.2)
            }
            Text { detail C.rgba(accent[0], accent[1], accent[2], 1.0) }
        }
        T {
            Style {
                display("block")
                width(100%)
                height(2.0)
                background_color(accent)
                background_z(-0.012)
            }
        }
    })
}

fn native_body_shell() {
    return accordion_body(T {
        name = "native_body_shell"
        Style {
            display("block")
            width(100%)
            height(9.0)
            color([0.91, 0.96, 1.0, 1.0])
            background_color([0.025, 0.075, 0.18, 0.97])
            background_z(-0.01)
        }
        T {
            name = "native_content_slot"
            Style { display("block") width(100%) }
            T.position(0.0, 0.0, 0.02) {
                Text { "initial native panel body (authored shell)" }
            }
        }
    })
}

let demo_state = {
    mms_generation = 0
}

let mms_panel = accordion({
    root_name = "mms_accordion"
    width_gu = 34.0
    unit_scale = 1.0
    background_color = [0.04, 0.16, 0.42, 0.98]
    toggle_background_color = [0.95, 0.73, 0.16, 1.0]
    children = [title_label("MMS responder"), title_chip("DataEvent → MMS")]
    body = body_card(
        "generation 0",
        "this body will be deleted, then rebuilt by an MMS DataEvent handler",
        [0.18, 0.62, 1.0, 1.0],
    )
})

let native_panel = accordion({
    root_name = "native_accordion"
    width_gu = 34.0
    unit_scale = 1.0
    background_color = [0.16, 0.08, 0.42, 0.98]
    toggle_background_color = [0.72, 0.46, 0.95, 1.0]
    children = [title_label("Native responder"), title_chip("DataEvent → Rust")]
    body = native_body_shell()
})

let mms_status = Text { "MMS panel: expanded" }
let native_status = Text { name = "native_accordion_status" "Native panel: expanded" }

T.position(-2.25, 2.3, 0.0).scale(0.065, 0.065, 1.0) {
    LayoutRoot {
        name = "accordion_demo_layout"
        available_width(74.0)
        available_height(44.0)

        T {
            name = "accordion_demo_shell"
            Style {
                display("flex")
                flex_direction("column")
                row_gap(2.0)
                width(100%)
            }

            T {
                Style {
                    display("flex")
                    flex_direction("row")
                    align_items("flex_start")
                    column_gap(3.0)
                    width(100%)
                }
                mms_panel
                native_panel
            }

            T {
                Style {
                    display("flex")
                    flex_direction("column")
                    row_gap(0.5)
                    width(100%)
                    padding(1.0)
                    color([0.72, 0.84, 1.0, 1.0])
                    background_color([0.01, 0.04, 0.12, 0.82])
                    background_z(-0.01)
                }
                T.position(0.0, 0.0, 0.02) {
                    Style { display("block") width(100%) }
                    mms_status
                }
                T.position(0.0, 0.0, 0.02) {
                    Style { display("block") width(100%) }
                    native_status
                }
                T.position(0.0, 0.0, 0.02) {
                    Style { display("block") width(100%) }
                    Text { "click −/+ in either title bar; WASD/RF/QE + RMB to move" }
                }
            }
        }
    }
}

let mms_body_mount = mms_panel.query("#accordion_body_mount")
on(mms_panel, "DataEvent", fn(event) {
    if event == "AccordionMinimized" {
        mms_status.set_text("MMS panel: body removed")
    } else if event == "AccordionRestoreRequested" {
        demo_state.mms_generation = demo_state.mms_generation + 1
        let generation_text = "generation " + demo_state.mms_generation
        let restored = body_card(
            generation_text,
            "restored entirely by an MMS handler responding to the molecule event",
            [0.22, 0.78, 1.0, 1.0],
        )
        mms_body_mount.attach(restored)
        mms_status.set_text("MMS panel: restored " + generation_text)
    }
})

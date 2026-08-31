// Rough-transmission comparison scene. Roughness increases from top-left to
// bottom-right. Every panel is a cube scaled 4:4:1.
import { star_kawaii_background } from "../assets/components/backgrounds/star_kawaii_background.mms"

RendererSettings { window_size(1280, 800) }
BGC.rgba(0.012, 0.018, 0.050, 1.0)
AL.rgb(0.16, 0.18, 0.28)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.04) half_res(true) } }
    Bloom { intensity(0.75) radius_ndc(0.04) emissive_scale(1.15) half_res(true) }
}

BG.occlusion_and_lighting() {
    star_kawaii_background([1.0, 0.68, 0.12, 1.0])
}

T.position(-3.5, 4.5, 3.5) {
    PL.color(0.48, 0.72, 1.0).intensity(7.0).distance(18.0)
}
T.position(3.5, 1.5, 4.0) {
    PL.color(1.0, 0.42, 0.68).intensity(6.0).distance(16.0)
}

fn rough_transmission_panel(panel_name, position, color, roughness) {
    return T.position(position[0], position[1], position[2]).scale(1.6, 1.6, 0.4) {
        name = panel_name
        R.cube() {
            C.rgba(color[0], color[1], color[2], color[3])
            RoughTransmission
                .ior(1.45)
                .thickness(0.14)
                .strength(1.0)
                .edge_fade(0.025)
                .roughness(roughness)
        }
    }
}

rough_transmission_panel("rough_transmission_0_00", [-1.9,  1.9, 0.0], [0.62, 0.88, 1.00, 0.48], 0.00)
rough_transmission_panel("rough_transmission_0_25", [ 1.9,  1.9, 0.0], [0.72, 1.00, 0.88, 0.48], 0.25)
rough_transmission_panel("rough_transmission_0_55", [-1.9, -1.9, 0.0], [0.94, 0.72, 1.00, 0.48], 0.55)
rough_transmission_panel("rough_transmission_0_85", [ 1.9, -1.9, 0.0], [1.00, 0.68, 0.78, 0.48], 0.85)

// Movable desktop fly camera; local -Z is forward.
I.speed(2.8) {
    InputTransformMode.forward_z() {
        fps_rotation()
        roll_axis_y()
    }
    T.position(0.0, 0.0, 10.0) {
        C3D { Pointer {} }
    }
}

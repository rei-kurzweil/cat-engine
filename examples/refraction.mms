// Refraction comparison scene. Each panel is a cube scaled 4:4:1 so it reads
// as a square, thick pane while preserving enough depth to inspect its edges.
import { star_kawaii_background } from "../assets/components/backgrounds/star_kawaii_background.mms"
import { transform_info_panel } from "../assets/components/ui/transform_info_panel.mms"

RendererSettings { window_size(1280, 800) }
BGC.rgba(0.012, 0.012, 0.040, 1.0)
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

// High-contrast, emissive references behind every transmissive object. Their
// narrow edges make the direction and strength of the displacement easy to see.
fn neon_line(y, color, tilt) {
    return T.position(0.0, y, -1.4).rotation(0.0, 0.0, tilt).scale(8.4, 0.09, 0.12) {
        R.cube() {
            C.rgba(color[0], color[1], color[2], 1.0)
            Emissive.on() { intensity(2.3) }
        }
    }
}

neon_line( 3.0, [1.00, 0.12, 0.22],  0.10)
neon_line( 2.0, [1.00, 0.38, 0.08], -0.08)
neon_line( 1.0, [1.00, 0.88, 0.10],  0.06)
neon_line( 0.0, [0.18, 1.00, 0.38], -0.05)
neon_line(-1.0, [0.10, 0.78, 1.00],  0.07)
neon_line(-2.0, [0.28, 0.34, 1.00], -0.09)
neon_line(-3.0, [0.88, 0.16, 1.00],  0.11)

// Foreground-depth rejection reference. This narrow opaque card sits just in front of and beside
// the sphere. Refraction may bend the background stripes near it, but must not pull the card's
// nearer pixels sideways into the sphere when the displaced lookup crosses the card.
T.position(1.62, 0.0, 2.35).scale(0.16, 1.55, 0.12) {
    name = "refraction_foreground_depth_card"
    R.cube() { C.rgba(1.0, 0.92, 0.12, 1.0) }
}

fn refraction_panel(panel_name, position, color, ior, thickness) {
    return T.position(position[0], position[1], position[2]).scale(1.6, 1.6, 0.4) {
        name = panel_name
        Grabbable {}
        R.cube() {
            C.rgba(color[0], color[1], color[2], color[3])
            Refraction.ior(ior).thickness(thickness).strength(1.0).edge_fade(0.025)
        }
    }
}

// Refraction should replace the covered scene sample. Keep alpha at 1.0 so the
// fixture shows only the refracted lookup rather than blending in the original.
refraction_panel("refraction_ior_1_10", [-1.9,  1.9, 0.0], [0.62, 0.88, 1.00, 1.0], 1.10, 0.05)
refraction_panel("refraction_ior_1_33", [ 1.9,  1.9, 0.0], [0.72, 1.00, 0.88, 1.0], 1.33, 0.10)
refraction_panel("refraction_ior_1_50", [-1.9, -1.9, 0.0], [0.94, 0.72, 1.00, 1.0], 1.50, 0.16)
refraction_panel("refraction_ior_1_80", [ 1.9, -1.9, 0.0], [1.00, 0.68, 0.78, 1.0], 1.80, 0.24)

// The sphere's smoothly changing normals should bend the neon lines continuously.
// It sits slightly in front of the panes so its silhouette remains easy to inspect.
let refraction_sphere = T.position(0.0, 0.0, 0.75).scale(1.35, 1.35, 1.35) {
    name = "refraction_sphere"
    Grabbable {}
    R.sphere() {
        C.rgba(0.88, 0.96, 1.00, 1.0)
        Refraction.ior(1.52).thickness(0.22).strength(1.0).edge_fade(0.025)
    }
}
refraction_sphere

T.position(-4.8, 4.0, 1.5) {
    EditorUI {
        panels([
            { panel = "settings" },
            { panel = "grid" },
        ])
    }
}

// Standalone world-space telemetry for the sphere above. It is intentionally a
// sibling of the scene objects, not a child of the camera or the inspected mesh.
T.position(-4.7, 2.8, 5.5) {
    transform_info_panel(refraction_sphere)
}

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

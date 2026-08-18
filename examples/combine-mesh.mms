import { star_kawaii_background } from "../assets/components/backgrounds/star_kawaii_background.mms"
import { truss } from "../assets/components/truss.mms"

RendererSettings { window_size(1440, 900) }
BGC.rgba(0.018, 0.024, 0.055, 1.0)
AL.rgb(0.11, 0.13, 0.22)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.035) half_res(true) } }
    Bloom { intensity(0.55) radius_ndc(0.035) emissive_scale(1.0) half_res(true) }
}

BG.occlusion_and_lighting() {
    // Saturated gold stays warm under the background's emissive star treatment
    // and bloom; pale yellow tends to read as white.
    star_kawaii_background([1.0, 0.68, 0.12, 1.0])
}

// Three independently placed instances; each truss's many bars become one
// phase-1 CombineMesh visual. ED keeps their authored hierarchy available to
// the editor's world panel.
ED {
    T.position(-5.0, 1.2, -4.0).rotation(0.0, 0.22, 0.08) { truss() }
    T.position( 4.6, 2.4, -5.5).rotation(0.0, -0.34, -0.10) { truss() }
    T.position( 0.0, 5.7, -7.0).rotation(0.0, 0.08, 1.5708) { truss() }
}

// Keep this scene's editor light: the asset browser is not needed here and
// its long list is expensive until panel clipping/scrolling is optimized.
T.position(-2.75, 2.8, -1.5) {
    EditorUI {
        panels([
            { panel = "world" },
            { panel = "settings" },
        ])
    }
}

T.position(0.0, -1.9, -4.0).scale(15.0, 0.10, 12.0) {
    R.cube() { C.rgba(0.035, 0.050, 0.10, 1.0) }
}

T.position(-2.5, 5.0, 2.5) {
    PL.color(0.40, 0.72, 1.0).intensity(8.0).distance(18.0)
}
T.position(5.5, 3.0, 1.5) {
    PL.color(1.0, 0.42, 0.64).intensity(7.0).distance(16.0)
}
T.position(0.0, 8.0, -2.0) {
    DL.color(0.78, 0.84, 1.0).intensity(0.75)
}

// Desktop fly camera: local -Z is forward.
I.speed(2.8) {
    InputTransformMode.forward_z() { fps_rotation() roll_axis_y() }
    T.position(0.0, 2.2, 13.5) {
        C3D { Pointer {} }
    }
}

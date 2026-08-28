// planar-auto-transparency-optimization.mms
//
// Benchmark scene for automatic planar transparency classification.
//
// The 12 x 12 layout creates exactly 144 translucent layout-generated __bg
// quads, which are the initial automatic-transparency optimization candidates.

import { truss } from "../assets/components/truss.mms"

RendererSettings { window_size(1440, 900) }

// Pink sunset sky.
BGC.rgba(0.72, 0.16, 0.46, 1.0)
AL.rgb(0.20, 0.08, 0.26)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.035) half_res(true) } }
    Bloom { intensity(0.65) radius_ndc(0.05) emissive_scale(1.2) half_res(true) }
}

// Orange concentric sun rings on the horizon.
T.position(0.0, -0.7, -17.0).scale(3.2, 3.2, 1.0) {
    R.partial_annulus_2d(0.20, 0.34, 0.0, 6.283185, 96) {
        C.rgba(1.0, 0.30, 0.04, 1.0)
        Emissive.on() { intensity(2.2) }
    }
    R.partial_annulus_2d(0.48, 0.62, 0.0, 6.283185, 96) {
        C.rgba(1.0, 0.46, 0.07, 1.0)
        Emissive.on() { intensity(1.8) }
    }
    R.partial_annulus_2d(0.78, 0.92, 0.0, 6.283185, 96) {
        C.rgba(1.0, 0.62, 0.14, 1.0)
        Emissive.on() { intensity(1.4) }
    }
}

// The truss asset's rails run along local X. Rotating 90 degrees around Z
// stands them vertically. Each site has an upper segment crossing the waterline,
// a bridge segment, and a lower segment continuing into the deep water.
fn vertical_truss_stack(x, z, yaw) {
    return T.position(x, 0.0, z).rotation(0.0, yaw, 0.0) {
        T.position(0.0, -0.15, 0.0).rotation(0.0, 0.0, 1.5708).scale(0.42, 0.42, 0.42) {
            truss()
        }
        T.position(0.0, -3.425, 0.0).rotation(0.0, 0.0, 1.5708).scale(0.42, 0.42, 0.42) {
            truss()
        }
        T.position(0.0, -6.70, 0.0).rotation(0.0, 0.0, 1.5708).scale(0.42, 0.42, 0.42) {
            truss()
        }
    }
}

vertical_truss_stack(-6.2, -13.2, 0.16)
vertical_truss_stack(-3.5, -15.0, -0.12)
vertical_truss_stack(5.2, -13.8, 0.22)

// Purple-blue translucent ocean. This uses the ordinary lit mesh material for
// now; animated wave shader inputs are tracked separately in docs/task.
T.position(0.0, -3.3, -5.0).rotation(-1.5708, 0.0, 0.0).scale(640.0, 640.0, 1.0) {
    R.plane() { C.rgba(0.30, 0.06, 0.58, 0.62) }
}

// Lights should remain visible on the ordinary ocean material and trusses.
T.position(-7.0, 5.0, 5.0) {
    PL.color(1.0, 0.26, 0.58).intensity(8.0).distance(24.0)
}
T.position(7.0, 2.0, 1.0) {
    PL.color(0.48, 0.28, 1.0).intensity(7.0).distance(22.0)
}
T.position(0.0, 8.0, -5.0) {
    PL.color(1.0, 0.62, 0.22).intensity(6.0).distance(26.0)
}

fn benchmark_cell() {
    return T {
        Style {
            display("inline-block")
            width(2.45)
            height(2.45)
            margin_right(0.22)
            background_color([0.64, 0.07, 0.34, 0.50])
        }
    }
}

fn benchmark_row() {
    return T {
        Style {
            display("block")
            width(32.2)
            height(2.45)
            margin_bottom(0.22)
        }
        for column in range(12) {
            benchmark_cell()
        }
    }
}

// Layout flows downward from its top-left. The LayoutRoot explicitly converts
// glyph units to world units; the parent transform only places the completed
// layout and does not implicitly rescale its renderable descendants.
T.position(-5.0, 5.2, -1.5) {
    LayoutRoot {
        name = "planar_auto_transparency_benchmark"
        available_width(33.0)
        available_height(33.0)
        unit_scale(0.31)

        for row in range(12) {
            benchmark_row()
        }
    }
}

// Movable camera without FPS mouse-look. Input uses +Z-forward translation and
// the requested local Z roll axis.
Input.speed(3.0) {
    InputTransformMode.forward_z() { roll_axis_z() }
    T.position(0.0, 1.4, 15.0) {
        C3D {
            Pointer {}
            RendererStats {
                update_interval_sec(0.25)
                smoothing(0.85)
                color([1.0, 0.86, 0.94, 1.0])
                emissive(true)
            }
        }
    }
}

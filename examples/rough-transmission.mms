// Rough-transmission comparison scene. Roughness increases from top-left to
// bottom-right. Every main panel is a cube scaled 6:6:1. Lit objects, dark
// translucent clouds, and the ground make the blur footprint easy to see.
import { cloud } from "../assets/components/backgrounds/cloud.mms"

RendererSettings { window_size(1280, 800) }
BGC.rgba(0.94, 0.94, 0.92, 1.0)
AL.rgb(0.18, 0.18, 0.18)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.04) half_res(true) } }
    Bloom { intensity(1.1) radius_ndc(0.04) emissive_scale(1.35) half_res(true) }
}

BG.occlusion_and_lighting() {
    // Rows recede and descend into the distance, mixing three- and five-puff
    // silhouettes. The cloud prefab controls each cluster's base width.
    T.position(-36.0,  7.0, -30.0) {
        cloud(5.0, 10.5, C.rgba(0.72, 0.82, 0.98, 1.0) {   })
    }
    T.position(  -10.0,  8.2, -31.0) {
        cloud(3.0, 8.5, C.rgba(0.98, 0.78, 0.86, 1.0) {   })
    }
    T.position( 16.0,  6.5, -29.0) {
        cloud(5.0, 10.8, C.rgba(0.80, 0.93, 0.82, 1.0) {   })
    }
    T.position(-19.0,  2.4, -54.0) {
        cloud(3.0, 11.5, C.rgba(0.98, 0.88, 0.68, 1.0) {   })
    }
    T.position( -1.5,  3.4, -55.0) {
        cloud(5.0, 11.0, C.rgba(0.82, 0.78, 0.96, 1.0) {   })
    }
    T.position( 17.0,  2.0, -53.0) {
        cloud(3.0, 10.5, C.rgba(0.74, 0.91, 0.92, 1.0) {   })
    }
    T.position(-34.0, -2.0, -84.0) {
        cloud(5.0, 13.0, C.rgba(0.95, 0.76, 0.78, 1.0) {   })
    }
    T.position(  19.5, -0.8, -85.0) {
        cloud(3.0, 12.0, C.rgba(0.78, 0.85, 0.98, 1.0) {   })
    }
    T.position( 54.0, -2.6, -83.0) {
        cloud(5.0, 13.2, C.rgba(0.84, 0.96, 0.80, 1.0) {   })
    }

    // Directional lights encode the incoming-light direction in their
    // transform position. A strong overhead key plus a cool side fill keep
    // both the background clouds and nearby test objects easy to read.
    T.position(0.15, 1.0, 0.30) {
        DL.color(1.0, 1.0, 1.0).intensity(1.8)
    }
    T.position(-0.85, 0.38, 0.42) {
        DL.color(0.72, 0.84, 1.0).intensity(1.05)
    }

    // A broad, soft-grey floor grounds the scene.
    T.position(0.0, -7.35, -18.0).rotation(-1.5708, 0.0, 0.0).scale(60.0, 60.0, 0.01) {
        R.square() {
            C.rgba(0.62, 0.63, 0.61, 1.0)
            Unlit {}
        }
    }
}

T.position(-3.5, 4.5, 3.5) {
    PL.color(0.48, 0.72, 1.0).intensity(7.0).distance(18.0)
}
T.position(3.5, 1.5, 4.0) {
    PL.color(1.0, 0.42, 0.68).intensity(6.0).distance(16.0)
}
fn rough_transmission_panel(panel_name, position, color, roughness) {
    return T.position(position[0], position[1], position[2]).scale(2.4, 2.4, 0.4) {
        name = panel_name
        R.cube() {
            C.rgba(color[0], color[1], color[2], color[3])
            RoughTransmission
                .strength(4.0)
                .edge_fade(0.05)
                .roughness(roughness)
        }
        
    }
}

rough_transmission_panel("rough_transmission_0_00", [-1.9,  1.9, 0.0], [0.62, 0.88, 1.00, 1.00], 0.95)
rough_transmission_panel("rough_transmission_0_25", [ 1.9,  1.9, 0.0], [0.72, 1.00, 0.88, 1.00], 0.95)
rough_transmission_panel("rough_transmission_0_55", [-1.9, -1.9, 0.0], [0.94, 0.72, 1.00, 1.00], 0.85)
rough_transmission_panel("rough_transmission_0_85", [ 1.9, -1.9, 0.0], [1.00, 0.68, 0.78, 1.00], 0.85)

fn lit_cube(cube_name, position, color) {
    return T.position(position[0], position[1], position[2]).scale(0.84, 0.84, 0.84) {
        name = cube_name
        R.cube() {
            C.rgba(color[0], color[1], color[2], 1.0)
        }
    }
}

// Keep the lit cubes behind the panels (negative Z: farther from this camera),
// with a few just outside their silhouettes for a crisp frosted comparison.
lit_cube("cube_cyan_top_left",    [-3.25,  3.15, -2.0], [0.05, 0.90, 1.00])
lit_cube("cube_magenta_top",      [ 0.00,  3.65, -2.0], [1.00, 0.05, 0.62])
lit_cube("cube_lime_top_right",   [ 3.25,  3.15, -2.0], [0.30, 1.00, 0.08])
lit_cube("cube_orange_left",      [-4.10,  0.00, -2.0], [1.00, 0.28, 0.04])
lit_cube("cube_violet_center",    [ 0.00,  0.00, -2.0], [0.55, 0.12, 1.00])
lit_cube("cube_blue_right",       [ 4.10,  0.00, -2.0], [0.10, 0.35, 1.00])
lit_cube("cube_rose_bottom_left", [-3.25, -3.15, -2.0], [1.00, 0.06, 0.25])
lit_cube("cube_gold_bottom_right",[ 3.25, -3.15, -2.0], [1.00, 0.72, 0.05])

fn lit_orb(orb_name, position, color) {
    return T.position(position[0], position[1], position[2]).scale(0.48, 0.48, 0.48) {
        name = orb_name
        R.sphere() {
            C.rgba(color[0], color[1], color[2], 1.0)
        }
    }
}

// Axis-alignment controls: with the camera looking straight down -Z, each
// panel and its named cube share exactly the same X/Y. Their apparent centers
// should therefore line up before refraction displaces the background sample.
rough_transmission_panel("axis_rough_transmission_0_00", [7.2,  4.6, 0.0], [0.62, 0.88, 1.00, 1.00], 0.95)
rough_transmission_panel("axis_rough_transmission_0_45", [7.2,  0.0, 0.0], [0.72, 1.00, 0.88, 1.00], 0.95)
rough_transmission_panel("axis_rough_transmission_0_85", [7.2, -4.6, 0.0], [1.00, 0.68, 0.78, 1.00], 0.95)

lit_cube("axis_cube_0_00", [7.2,  4.6, -2.0], [0.10, 0.90, 1.00])
lit_cube("axis_cube_0_45", [7.2,  0.0, -2.0], [1.00, 0.12, 0.62])
lit_cube("axis_cube_0_85", [7.2, -4.6, -2.0], [0.95, 0.35, 1.00])

// Separate lit orbs give the frosted panels a round, high-contrast shape
// in addition to the axis-aligned cube controls.
lit_orb("orb_top",    [5.2,  4.6, -2.4], [0.15, 0.55, 1.00])
lit_orb("orb_center", [5.2,  0.0, -2.4], [1.00, 0.18, 0.45])
lit_orb("orb_bottom", [5.2, -4.6, -2.4], [0.75, 0.22, 1.00])

// Movable desktop fly camera; local -Z is forward.
I.speed(2.8) {
    InputTransformMode.forward_z() {
        fps_rotation()
        roll_axis_y()
    }
    T.position(0.0, 0.0, 16.0) {
        C3D { Pointer {} }
    }
}

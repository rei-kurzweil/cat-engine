// Refractive implicit-surface study. The star field and floating emissive
// spheres provide sharp, colourful scene detail for inspecting distortion.
//
// Run with:
//   cargo run --release -- load examples/implicit-surface-refraction-clouds.mms

import { star_kawaii_background } from "../assets/components/backgrounds/star_kawaii_background.mms"

RendererSettings { window_size(1280, 800) }
BGC.rgba(0.012, 0.020, 0.060, 1.0)
AL.rgb(0.12, 0.16, 0.26)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.045) half_res(true) } }
    Bloom { intensity(0.80) radius_ndc(0.045) emissive_scale(1.20) half_res(true) }
}

// Background stays behind the opaque scene capture that refraction samples.
BG.occlusion_and_lighting() {
    star_kawaii_background([0.74, 0.86, 1.0, 1.0])
}

// Warm/cool lights make the opaque reference spheres legible outside the blob.
T.position(-4.0, 4.5, 3.0) {
    PL.color(0.34, 0.72, 1.0).intensity(7.0).distance(18.0)
}
T.position(4.0, 1.5, 2.5) {
    PL.color(1.0, 0.32, 0.62).intensity(6.0).distance(16.0)
}

// Bright air-borne references behind the refractive surface. Bloom happens in
// the final composite, while their sharp cores are available to the lookup.
fn sky_reference(position, color, radius) {
    return T.position(position[0], position[1], position[2]).scale(radius, radius, radius) {
        R.sphere() {
            C.rgba(color[0], color[1], color[2], 1.0)
            Emissive.on() { intensity(2.3) }
        }
    }
}

sky_reference([-3.2,  2.3, -2.8], [1.00, 0.22, 0.42], 0.48)
sky_reference([ 2.8,  2.0, -3.2], [0.18, 0.86, 1.00], 0.58)
sky_reference([-2.6, -2.0, -2.6], [1.00, 0.78, 0.12], 0.42)
sky_reference([ 3.0, -1.8, -3.0], [0.72, 0.22, 1.00], 0.50)

// Intersecting implicit spheres bake into one smooth mesh. Every neighbouring
// puff overlaps, so the result is one semi-connected lens-like blob.
ImplicitSurface
    .bounds(-3.2, -2.4, -1.8, 3.2, 2.4, 1.8)
    .voxel_size(0.11)
    .iso_level(0.0)
    .smooth_min_radius(0.42) {

    name = "refractive_implicit_blob"
    C.rgba(0.86, 0.96, 1.0, 1.0)
    Refraction.ior(1.48).thickness(0.20).strength(1.0).edge_fade(0.025)

    T.position(-1.35, -0.15, 0.00) { ImplicitSphere.radius(1.20) {} }
    T.position(-0.45,  0.48, 0.10) { ImplicitSphere.radius(1.05) {} }
    T.position( 0.52,  0.25, 0.00) { ImplicitSphere.radius(1.18) {} }
    T.position( 1.35, -0.30, 0.10) { ImplicitSphere.radius(1.00) {} }
    T.position(-0.70, -0.82, 0.18) { ImplicitSphere.radius(0.84) {} }
    T.position( 0.38, -0.74, 0.08) { ImplicitSphere.radius(0.92) {} }
    T.position( 0.05,  1.05, 0.02) { ImplicitSphere.radius(0.76) {} }
}

// Movable desktop camera; local -Z is forward.
I.speed(2.8) {
    InputTransformMode.forward_z() {
        fps_rotation()
        roll_axis_y()
    }
    T.position(0.0, 0.0, 10.0) {
        C3D { Pointer {} }
    }
}

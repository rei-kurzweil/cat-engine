// Early anime visual-novel background study for the ImplicitSurface first slice.
// The final staircase scene can copy this terrain/tree composition once mesh
// CSG is ready for the stair and path clearance cuts.
//
// Run with:
//   cargo run --release -- load examples/implicit-surface.mms

RendererSettings { window_size(1440, 900) }
BGC.rgba(0.66, 0.84, 0.98, 1.0)
AL.rgb(0.30, 0.32, 0.28)

// Warm late-afternoon key light with a soft cool fill.
T.rotation(-0.62, -0.55, 0.0) {
    DL.color(1.0, 0.88, 0.68).intensity(1.15)
}
T.position(-5.0, 7.0, 4.0) {
    PL.color(0.72, 0.84, 1.0).intensity(5.0).distance(24.0)
}

// Change this one value to explore another deterministic landscape. Math.perlin
// has no seed argument yet, so the seed selects stable coordinate offsets.
let terrain_seed = 23.0

fn terrain_height(row, x, z, seed) {
    // The first two rows straddle the tree at z=6. Their calibrated sphere
    // centers put the fused ground at the bottom of the existing trunk.
    if row <= 1.0 {
        return -7.90
    }

    let downhill = (row - 1.0) * 0.20
    let seed_x = seed * 1.31
    let seed_z = seed * 0.73
    // One readable, lower-frequency layer. Fade it in after the plateau so the
    // tree transition stays level, then let it produce broad rolling variation.
    let rolling = Math.perlin(x * 0.025 + 13.0 + seed_x, z * 0.025 - 7.0 - seed_z)
    let noise_fade = Math.smoothstep(row, 1.0, 3.0)
    return -7.90 - downhill + rolling * 0.85 * noise_fade
}

// A 12x12 lattice of large overlapping implicit spheres. Its X/Z footprint is
// four times the previous study. The nearest row is centered under the camera,
// the tree lies between the first two rows, and later rows roll downward.
ImplicitSurface
    .bounds(-47.0, -22.0, -76.5, 47.0, 3.0, 18.0)
    .voxel_size(0.90)
    .iso_level(0.0)
    .smooth_min_radius(2.80) {
    name = "rolling-hill-lattice"
    C.rgba(0.34, 0.58, 0.24, 1.0)

    for row in range(12) {
        for column in range(12) {
            let x = (column - 5.5) * 7.2
            let z = 10.5 - row * 7.2
            let y = terrain_height(row, x, z, terrain_seed)
            T.position(x, y, z) {
                ImplicitSphere.radius(6.20) {}
            }
        }
    }
}

// Camera-right foreground tree, placed between the camera and the rolling
// hill view. The trunk stays intentionally plain until a bark texture exists;
// the canopy is a softly fused cluster rather than intersecting sphere meshes.
T.position(3.1, -1.1, 6.0) {
    T.position(0.0, -0.05, 0.0).scale(0.38, 2.35, 0.42) {
        R.cube() { C.rgba(0.34, 0.18, 0.075, 1.0) }
    }

    ImplicitSurface
        .bounds(-3.1, -0.5, -3.0, 3.1, 5.2, 3.0)
        .voxel_size(0.13)
        .iso_level(0.0)
        .smooth_min_radius(0.55) {
        name = "deciduous-canopy"
        C.rgba(0.20, 0.52, 0.18, 1.0)

        T.position(-0.85, 2.25, 0.05) {
            ImplicitSphere.radius(1.65) {}
        }
        T.position(0.80, 2.35, 0.10) {
            ImplicitSphere.radius(1.55) {}
        }
        T.position(-0.10, 3.15, -0.20) {
            ImplicitSphere.radius(1.50) {}
        }
        T.position(-0.15, 2.35, 0.95) {
            ImplicitSphere.radius(1.35) {}
        }
        T.position(0.15, 2.25, -1.00) {
            ImplicitSphere.radius(1.30) {}
        }
    }
}

// Fixed first-person framing for the future anime VN background. Camera3D
// looks along local -Z; fly controls remain useful while shaping the scene.
I.speed(2.4) {
    InputTransformMode.forward_z() { roll_axis_z() }
    T.position(0.0, 1.2, 10.5) {
        C3D { Pointer {} }
    }
}

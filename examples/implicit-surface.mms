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

// Camera-right hillside: successively lower overlapping spheres make the
// skyline and ground plane slope down toward the right edge of the shot.
ImplicitSurface
    .bounds(-7.0, -12.0, -11.5, 16.0, 3.5, 4.0)
    .voxel_size(0.24)
    .iso_level(0.0)
    .smooth_min_radius(1.25) {
    name = "descending-right-hill"
    C.rgba(0.34, 0.58, 0.24, 1.0)

    T.position(1.0, -4.6, -3.8) {
        ImplicitSphere.radius(6.2) {}
    }
    T.position(5.2, -5.2, -3.3) {
        ImplicitSphere.radius(5.6) {}
    }
    T.position(9.3, -5.9, -2.8) {
        ImplicitSphere.radius(4.9) {}
    }
}

// Simple temporary lawn beneath the camera. The eventual staircase/path CSG
// scene will replace or trim this transition without moving the hill study.
T.position(0.0, -1.72, 1.0).scale(16.0, 0.10, 10.0) {
    R.cube() { C.rgba(0.30, 0.50, 0.22, 1.0) }
}

// Nearer camera-left deciduous tree. The trunk stays intentionally plain until
// a bark texture exists; the canopy is a softly fused cluster rather than a
// collection of visibly intersecting sphere meshes.
T.position(-3.5, 0.0, -0.8) {
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

// First-slice CSG example: a deliberately asymmetric box subtraction.
// Run with:
//   cargo run --release -- load examples/constructive-solid-geometry.mms

RendererSettings { window_size(1280, 800) }
BGC.rgba(0.018, 0.024, 0.040, 1.0)
AL.rgb(0.16, 0.18, 0.24)

T.position(0.0, 0.4, -1.5).rotation(-0.10, 0.28, 0.0) {
    CSG {
        name = "offset-box-subtraction"
        Subtraction {
            // The result keeps this base material.
            T.scale(2.6, 2.0, 2.2) {
                R.cube() { C.rgba(0.20, 0.62, 0.92, 1.0) }
            }

            // Offset toward the top-right-front corner so the cut breaches
            // three faces and exposes the newly generated interior clearly.
            T.position(1.0, 0.65, 0.72).scale(1.55, 1.35, 1.35) {
                R.cube() { C.rgba(1.0, 0.24, 0.18, 1.0) }
            }
        }
    }
}

// A dark floor makes the silhouette and the open notch easy to read.
T.position(0.0, -1.75, -1.5).scale(8.0, 0.10, 7.0) {
    R.cube() { C.rgba(0.045, 0.055, 0.075, 1.0) }
}

T.position(-3.5, 5.0, 4.0) {
    PL.color(0.72, 0.86, 1.0).intensity(9.0).distance(18.0)
}
T.rotation(-0.55, -0.65, 0.0) {
    DL.color(1.0, 0.78, 0.58).intensity(1.0)
}

// Desktop fly camera; Camera3D looks along local -Z.
I.speed(2.5) {
    InputTransformMode.forward_z() { roll_axis_z() }
    T.position(0.0, 1.3, 8.5) {
        C3D { Pointer {} }
    }
}

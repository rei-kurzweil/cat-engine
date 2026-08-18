// A static square-section space truss.  The CombineMesh root deliberately owns
// every bar: phase-1 CombineMesh bakes these Renderables into one visual.

fn rail(x, y, z, length, thickness) {
    return T.position(x, y, z).scale(length, thickness, thickness) {
        R.cube() {}
    }
}

fn brace_xy(x, z, angle) {
    // Local X is the bar's long axis.  This spans one side of the square
    // section in the XY plane; the alternating angle makes triangular bays.
    return T.position(x, 0.0, z).rotation(0.0, 0.0, angle).scale(1.735, 0.10, 0.10) {
        R.cube() {}
    }
}

fn brace_xz(x, y, angle) {
    // The matching bracing on the XZ faces.
    return T.position(x, y, 0.0).rotation(0.0, angle, 0.0).scale(1.735, 0.10, 0.10) {
        R.cube() {}
    }
}

export fn truss() {
    // The rails overhang the braced section by two bays (2 × 1.125) on both
    // ends, so they read as continuous structural beams rather than stopping
    // flush with the outermost diagonal.
    let length = 13.5
    let half_length = length / 2.0
    let half_width = 0.65
    let brace_angle = 0.857072 // atan(1.3 / 1.125): equilateral-triangle bays

    return T {
        name = "truss"
        CombineMesh {
            // Four long X rails at the corners of the YZ square.
            rail(0.0,  half_width,  half_width, half_length, 0.13)
            rail(0.0,  half_width, -half_width, half_length, 0.13)
            rail(0.0, -half_width,  half_width, half_length, 0.13)
            rail(0.0, -half_width, -half_width, half_length, 0.13)

            // Alternating diagonals on both XY faces (front/back).
            brace_xy(-3.9375,  half_width,  brace_angle)
            brace_xy(-2.8125,  half_width, -brace_angle)
            brace_xy(-1.6875,  half_width,  brace_angle)
            brace_xy(-0.5625,  half_width, -brace_angle)
            brace_xy( 0.5625,  half_width,  brace_angle)
            brace_xy( 1.6875,  half_width, -brace_angle)
            brace_xy( 2.8125,  half_width,  brace_angle)
            brace_xy( 3.9375,  half_width, -brace_angle)

            brace_xy(-3.9375, -half_width, -brace_angle)
            brace_xy(-2.8125, -half_width,  brace_angle)
            brace_xy(-1.6875, -half_width, -brace_angle)
            brace_xy(-0.5625, -half_width,  brace_angle)
            brace_xy( 0.5625, -half_width, -brace_angle)
            brace_xy( 1.6875, -half_width,  brace_angle)
            brace_xy( 2.8125, -half_width, -brace_angle)
            brace_xy( 3.9375, -half_width,  brace_angle)

            // Alternating diagonals on the XZ faces (top/bottom).
            brace_xz(-3.9375,  half_width, -brace_angle)
            brace_xz(-2.8125,  half_width,  brace_angle)
            brace_xz(-1.6875,  half_width, -brace_angle)
            brace_xz(-0.5625,  half_width,  brace_angle)
            brace_xz( 0.5625,  half_width, -brace_angle)
            brace_xz( 1.6875,  half_width,  brace_angle)
            brace_xz( 2.8125,  half_width, -brace_angle)
            brace_xz( 3.9375,  half_width,  brace_angle)

            brace_xz(-3.9375, -half_width,  brace_angle)
            brace_xz(-2.8125, -half_width, -brace_angle)
            brace_xz(-1.6875, -half_width,  brace_angle)
            brace_xz(-0.5625, -half_width, -brace_angle)
            brace_xz( 0.5625, -half_width,  brace_angle)
            brace_xz( 1.6875, -half_width, -brace_angle)
            brace_xz( 2.8125, -half_width,  brace_angle)
            brace_xz( 3.9375, -half_width, -brace_angle)
        }
    }
}

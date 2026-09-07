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

export fn truss(segment_count) {
    // Keep the original eight-bay truss as the no-argument default.
    if segment_count == null { segment_count = 8 }

    let bay_width = 1.125
    let length = segment_count * bay_width
    let half_width = 0.65
    let brace_angle = 0.857072 // atan(1.3 / 1.125): equilateral-triangle bays

    return CombineMesh {
        name = "truss"
        // The rail length matches the full braced span. Each outer rail end
        // reaches half a bay beyond the center of the outermost diagonal.
        rail(0.0,  half_width,  half_width, length, 0.13)
        rail(0.0,  half_width, -half_width, length, 0.13)
        rail(0.0, -half_width,  half_width, length, 0.13)
        rail(0.0, -half_width, -half_width, length, 0.13)

        for segment in range(segment_count) {
            let x = (segment - (segment_count - 1) / 2.0) * bay_width

            if segment % 2 == 0 {
                // Alternating diagonals on both XY faces (front/back).
                brace_xy(x,  half_width,  brace_angle)
                brace_xy(x, -half_width, -brace_angle)

                // Matching bracing on the XZ faces (top/bottom).
                brace_xz(x,  half_width, -brace_angle)
                brace_xz(x, -half_width,  brace_angle)
            } else {
                brace_xy(x,  half_width, -brace_angle)
                brace_xy(x, -half_width,  brace_angle)

                brace_xz(x,  half_width,  brace_angle)
                brace_xz(x, -half_width, -brace_angle)
            }
        }
    }
}

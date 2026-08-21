// Desktop-only paint/grid diagnostic scene.
//
// Run with:
//   MITTENS_DEBUG_PAINT_STROKE=1 cargo run --example paint-grids-desktop
//
// This is intentionally a no-XR repro: do not add InputXR, CXR, XRHand, or XR
// nodes here. See docs/task/paint-grid-stroke-cell-deduplication-and-desktop-repro.md.

RendererSettings { window_size(1100, 720) }
BGC.rgba(0.055, 0.070, 0.11, 1.0)
AL.rgb(0.28, 0.30, 0.36)

T.position(-3.5, 5.0, 4.5) {
    DL.color(1.0, 0.92, 0.82).intensity(1.15)
}

ED.active() {
    name = "paint_grids_desktop_editor"

    // Adjacent, independently raycastable wall targets make an unintended
    // target/cell transition visible while retaining one coplanar surface.
    T.position(-2.05, 1.15, -3.0).scale(1.95, 2.3, 0.14) {
        name = "paint_target_left"
        R.cube() { C.rgba(0.20, 0.46, 0.82, 1.0) Raycastable.enabled() }
    }
    T.position(0.0, 1.15, -3.0).scale(1.95, 2.3, 0.14) {
        name = "paint_target_center"
        R.cube() { C.rgba(0.30, 0.72, 0.48, 1.0) Raycastable.enabled() }
    }
    T.position(2.05, 1.15, -3.0).scale(1.95, 2.3, 0.14) {
        name = "paint_target_right"
        R.cube() { C.rgba(0.88, 0.43, 0.30, 1.0) Raycastable.enabled() }
    }

    // Grid-local X/Z maps to this wall's XY plane.
    T.position(0.0, 1.15, -2.90).rotation(1.570796, 0.0, 0.0) {
        name = "debug_vertical_grid_spacing_0_5"
        Grid.spacing(0.5) {
            size_x(16)
            size_z(8)
            enabled(true)
            hidden(false)
            selectable(true)
        }
    }
}

T.position(-3.7, 3.0, -0.5) {
    EditorUI {
        panels([
            { panel = "world" },
            { panel = "assets" },
            { panel = "paint" },
            { panel = "color" },
            { panel = "grid" },
            { panel = "settings" },
        ])
    }
}

I.speed(2.2) {
    InputTransformMode.forward_z() {
        roll_axis_y()
        fps_rotation()
    }
    T.position(0.0, 1.4, 6.5) {
        name = "paint_grids_desktop_camera"
        C3D { Pointer {} }
    }
}

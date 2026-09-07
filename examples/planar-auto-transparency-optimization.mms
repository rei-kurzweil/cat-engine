// planar-auto-transparency-optimization.mms
//
// Benchmark scene for automatic planar transparency classification.
//
// The 24 x 24 layout creates exactly 576 translucent layout-generated __bg
// quads. Each cell also contains one opaque cube behind its background, making
// this a useful current single-layer benchmark and future depth-write fixture.

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

// The truss asset's rails run along local X. Its diagonal braces make the full
// visible length about 9.09 asset units, or 3.82 world units at scale 0.42.
// Give each segment an explicit block of that height plus a small gap so layout
// owns the stack spacing without waiting for the combined mesh to be measured.
fn vertical_truss_stack(x, z, yaw) {
    let truss_width = 0.55
    let truss_height = 3.82
    let segment_gap = 0.08

    // Keep the first segment centered near its previous y=-0.15 position.
    return T.position(x, 1.76, z).rotation(0.0, yaw, 0.0) {
        T.position(-truss_width / 2.0, 0.0, 0.0) {
            LayoutRoot {
                name = "vertical_truss_stack"
                available_width(truss_width)
                available_height(12.0)
                unit_scale(1.0)

                for segment in range(3) {
                    T {
                        Style {
                            display("block")
                            width(truss_width)
                            height(truss_height)
                            margin_bottom(segment_gap)
                        }
                        T.rotation(0.0, 0.0, 1.5708).scale(0.42, 0.42, 0.42) {
                            truss(8)
                        }
                    }
                }
            }
        }
    }
}

vertical_truss_stack(-6.2, -13.2, 0.16)
vertical_truss_stack(-3.5, -15.0, -0.12)
vertical_truss_stack(5.2, -13.8, 0.22)

// Purple-blue translucent ocean. The missing layout content is now known to
// occur before transparency batching, so the ocean can remain in the scene.
T.position(0.0, -3.3, -5.0).rotation(-1.5708, 0.0, 0.0).scale(640.0, 640.0, 1.0) {
    R.plane() { C.rgba(0.40, 0.26, 0.58, 0.62) }
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

T.position(-15.0, 5.0, -20.0) {
    PL.color(1.0, 0.62, 0.22).intensity(6.0).distance(26.0)
}

T.position(10.0, 8.0, -15.0) {
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
        // Authored opaque geometry behind the layout-owned translucent __bg.
        // `position` is local to this inline-block; layout preserves the
        // explicit Z because only item TCs receive layout-owned layering.
        T.position(0.0, 0.0, -2.0).scale(0.5, 0.5, 0.5) {
            R.cube() { C.rgba(1.0, 1.0, 1.0, 1.0) }
        }
    }
}

fn benchmark_row() {
    return T {
        Style {
            display("block")
            width(64.5)
            height(2.45)
            margin_bottom(0.22)
        }
        for column in range(24) {
            benchmark_cell()
        }
    }
}

// Layout flows downward from its top-left. The LayoutRoot explicitly converts
// glyph units to world units; the parent transform only places the completed
// layout and does not implicitly rescale its renderable descendants.
T.position(-10.0, 10.3, -1.5) {
    LayoutRoot {
        name = "planar_auto_transparency_benchmark"
        available_width(65.0)
        available_height(65.0)
        unit_scale(0.31)

        for row in range(24) {
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
        }
        // Camera-relative stats panel: starts in front of the camera and can
        // be repositioned by dragging its backing square.
        T.position(-3.8, 1.9, -8.75) {
            name = "planar_benchmark_stats"
            Draggable.plane("camera")
            T.position(2.0, -0.35, -0.02).scale(5.0, 0.8, 1.0) {
                R.square() {
                    C.rgba(0.04, 0.02, 0.10, 0.78)
                    Raycastable.drag_only()
                }
            }
            RendererStats {
                update_interval_sec(0.25)
                smoothing(0.85)
                color([1.0, 0.86, 0.94, 1.0])
                emissive(true)
            }
        }
    }
}

// XR-only rough-transmission validation fixture.
//
// This intentionally contains no Refraction components. It gives the first
// per-eye scene snapshot + rough-pyramid implementation one material model,
// five known filter footprints, and high-contrast opaque/emissive content
// behind the panels. Keep sharp refraction as a later, separate XR slice.

RendererSettings { window_size(960, 640) }
BGC.rgba(0.0, 0.0, 0.0, 1.0)
AL.rgb(0.12, 0.14, 0.20)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.040) half_res(true) } }
    Bloom { intensity(0.75) radius_ndc(0.040) emissive_scale(1.15) half_res(true) }
}

// Tracked XR is the only camera path in this fixture.
T {
    InputXR.on() {
        InputXRGamepad { locomotion() speed(1.5) }
        T { CXR { Pointer {} } }
    }
}
XR.on()

// A dark opaque card and a dense bright pattern give every roughness level
// edges, small features, and Bloom halos to filter. They are behind the glass
// panels for an XR viewer at the origin looking down local -Z.
T.position(0.0, 1.55, -5.2).scale(3.05, 1.45, 1.0) {
    R.square() { C.rgba(0.025, 0.035, 0.075, 1.0) }
}

fn pattern_color(index) {
    if index % 3 == 0 { return [0.98, 0.28, 0.42, 1.0] }
    if index % 3 == 1 { return [0.24, 0.82, 1.00, 1.0] }
    return [1.00, 0.76, 0.22, 1.0]
}

for row in range(5) {
    for column in range(11) {
        let index = row * 11 + column
        let x = (column - 5.0) * 0.48
        let y = 2.48 - row * 0.46
        let color = pattern_color(index)
        let emission = 1.4 + (index % 5) * 0.20

        T.position(x, y, -4.85).scale(0.13, 0.13, 0.10) {
            R.cube() {
                C.rgba(color[0], color[1], color[2], color[3])
                Emissive.on() { intensity(emission) }
            }
        }
    }
}

fn rough_panel(panel_name, x, roughness, color) {
    return T.position(x, 1.55, -2.5).scale(0.34, 0.56, 0.12) {
        name = panel_name
        Grabbable {}
        R.cube() {
            C.rgba(color[0], color[1], color[2], color[3])
            RoughTransmission
                .ior(1.45)
                .thickness(0.14)
                .strength(1.0)
                .edge_fade(0.025)
                .roughness(roughness)
        }
    }
}

// 0.00 is the sharp control; the other panels exercise every interpolation
// region through the bounded 1/2..1/32 rough-transmission pyramid.
rough_panel("xr_rough_transmission_0_00", -1.60, 0.00, [0.66, 0.92, 1.00, 0.56])
rough_panel("xr_rough_transmission_0_25", -0.80, 0.25, [0.74, 1.00, 0.86, 0.56])
rough_panel("xr_rough_transmission_0_50",  0.00, 0.50, [0.94, 0.80, 1.00, 0.56])
rough_panel("xr_rough_transmission_0_75",  0.80, 0.75, [1.00, 0.78, 0.68, 0.56])
rough_panel("xr_rough_transmission_1_00",  1.60, 1.00, [1.00, 0.64, 0.82, 0.56])

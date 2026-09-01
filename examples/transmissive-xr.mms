// XR-only mixed transmissive-material scene: two refraction panels and two
// rough-transmission panels. There is intentionally no desktop camera or avatar.
import { star_kawaii_background } from "../assets/components/backgrounds/star_kawaii_background.mms"

RendererSettings { window_size(960, 640) }
BGC.rgba(0.012, 0.018, 0.050, 1.0)
AL.rgb(0.18, 0.20, 0.30)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.045) half_res(true) } }
    Bloom { intensity(0.8) radius_ndc(0.045) emissive_scale(1.2) half_res(true) }
}

BG.occlusion_and_lighting() {
    star_kawaii_background([1.0, 0.68, 0.12, 1.0])
}

T.position(0.0, 4.0, 1.5) {
    PL.color(0.62, 0.78, 1.0).intensity(6.0).distance(14.0)
}

// Tracked XR locomotion and camera are the sole camera path.
T {
    InputXR.on() {
        InputXRGamepad { locomotion() speed(1.5) }
        T { CXR { Pointer {} } }
    }
}
XR.on()

T.position(-2.75, 2.8, -1.5) {
    EditorUI {
        panels([
            { panel = "settings" },
            { panel = "grid" },
        ])
    }
}

// Four 4:4:1 thick panels sit about 2.5 metres in front of the XR origin.
T.position(-0.75, 1.85, -2.5).scale(0.62, 0.62, 0.155) {
    name = "xr_refraction_1_33"
    Grabbable {}
    R.cube() {
        C.rgba(0.62, 0.88, 1.00, 0.50)
        Refraction.ior(1.33).thickness(0.08).strength(1.0).edge_fade(0.025)
    }
}
T.position(0.75, 1.85, -2.5).scale(0.62, 0.62, 0.155) {
    name = "xr_refraction_1_60"
    Grabbable {}
    R.cube() {
        C.rgba(0.94, 0.72, 1.00, 0.50)
        Refraction.ior(1.60).thickness(0.16).strength(1.0).edge_fade(0.025)
    }
}
T.position(-0.75, 0.45, -2.5).scale(0.62, 0.62, 0.155) {
    name = "xr_rough_transmission_0_25"
    Grabbable {}
    R.cube() {
        C.rgba(0.72, 1.00, 0.88, 0.50)
        RoughTransmission.ior(1.45).thickness(0.14).strength(1.0).edge_fade(0.025).roughness(0.25)
    }
}
T.position(0.75, 0.45, -2.5).scale(0.62, 0.62, 0.155) {
    name = "xr_rough_transmission_0_70"
    Grabbable {}
    R.cube() {
        C.rgba(1.00, 0.68, 0.78, 0.50)
        RoughTransmission.ior(1.45).thickness(0.14).strength(1.0).edge_fade(0.025).roughness(0.70)
    }
}

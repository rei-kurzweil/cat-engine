// Side-by-side Bisket shading-model comparison.
//
// Run with:
//   cargo run --release -- load examples/shading-models.mms

// The left model uses the default GLTF shading model. The right model uses the
// albedo-derived AnimeShading material. Both models use the same relaxed A-pose
// and each is lit by an identical movable white spotlight fixture.

import { pose as relaxed_pose_factory } from "../assets/components/poses/bisket/000-relaxed.pose.mms"
import { bisket_secondary_motion } from "../assets/components/secondary_motion/bisket.mms"
import { tripod_light } from "../assets/components/tripod_light.mms"
import { truss } from "../assets/components/truss.mms"

RendererSettings { window_size(1280, 720) }
BGC.rgba(0.035, 0.040, 0.055, 1.0)
AL.rgb(0.06, 0.06, 0.06)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.025) half_res(true) } }
    Bloom { intensity(0.30) radius_ndc(0.025) emissive_scale(1.0) half_res(true) }
}

// Neutral floor and backdrop make differences in the two model materials clear.
T.position(0.0, -0.06, 0.0).scale(12.0, 0.12, 10.0) {
    name = "shading_models_floor"
    R.cube() { C.rgba(0.11, 0.12, 0.15, 1.0) }
}
T.position(0.0, 2.4, -2.2).scale(12.0, 4.8, 0.12) {
    name = "shading_models_backdrop"
    R.cube() { C.rgba(0.15, 0.16, 0.20, 1.0) }
}

// A subdued overhead truss frames the comparison without drawing attention
// away from the two shading models.
T.position(0.0, 4.05, -0.7).scale(0.78, 0.78, 0.78) {
    name = "shading_models_overhead_truss"
    Unlit {
        truss(32)
    }
}

let left_model_x = -1.65
let right_model_x = 1.65

// Default GLTF shading model.
T.position(left_model_x, 0.0, 0.0) {
    name = "bisket_default_shading"
    GLTF.new("assets/models/bisket.glb") {
        relaxed_pose_factory()
        bisket_secondary_motion(false)
    }
}

// Albedo-derived anime shading model with a two-state light ramp and rim light.
T.position(right_model_x, 0.0, 0.0) {
    name = "bisket_anime_shading"
    GLTF.new("assets/models/bisket.glb") {
        relaxed_pose_factory()
        bisket_secondary_motion(false)
        AnimeShading.shade_color([0.4, 0.4, 0.65])
            .shade_strength(0.30)
            .shade_threshold(0.35)
            .lit_threshold(0.55)
            .rim_color([1.0, 1.0, 1.0])
            .rim_strength(0.18)
            .rim_power(4.0)
    }
}

// Matching grabbable fixtures flank the models and aim at their upper bodies.
tripod_light(
    "default_shading_spotlight",
    [-4.5, 0.0, 2.2],
    [left_model_x, 1.25, 0.0],
    SL.color(1.0, 1.0, 1.0).intensity(8.0).distance(12.0).angle(0.58).penumbra(0.25),
)
tripod_light(
    "anime_shading_spotlight",
    [4.5, 0.0, 2.2],
    [right_model_x, 1.25, 0.0],
    SL.color(1.0, 1.0, 1.0).intensity(8.0).distance(12.0).angle(0.58).penumbra(0.25),
)

// Movable desktop overview camera. C3D looks along local -Z.
I.speed(2.0) {
    name = "shading_models_camera_input"
    InputTransformMode.forward_z() {
        fps_rotation()
        roll_axis_y()
    }
    T.position(0.0, 1.55, 7.5) {
        name = "shading_models_camera"
        C3D { Pointer {} }
    }
}

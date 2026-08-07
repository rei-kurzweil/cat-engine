// XR-only secondary-motion prototype. Spring metadata is attached by the Rust loader.
// Move and turn your head/body in front of the mirror: hair, bust, tail, and
// shirt-hem chains should sag visibly under gravity, lag behind the primary
// avatar pose, keep their lengths, and oscillate briefly before settling.
import { bisket_shirt_physics } from "../assets/components/secondary_motion/bisket-shirt-physics.mms"
import { bisket_colliders } from "../assets/components/colliders/bisket.mms"

RendererSettings { window_size(640, 480) }
BGC.rgba(0.12, 0.16, 0.24, 1.0)
AL.rgb(0.18, 0.18, 0.22)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.06) half_res(true) } }
    Bloom { intensity(0.8) emissive_scale(1.1) }
}

T.position(1.0, 2.5, 1.5) {
    DL { intensity(1.2) color(1.0, 0.98, 0.95) }
}

ED {
    // Floor and side markers provide room-scale motion reference.
    T.position(0.0, -0.85, -1.5).scale(6.0, 0.12, 6.0) {
        R.cube() { C.rgba(0.20, 0.22, 0.27, 1.0) }
    }
    T.position(-1.8, 0.0, -2.8).scale(0.25, 0.9, 0.25) {
        R.cube() { C.rgba(0.2, 0.8, 1.0, 1.0) EM.on() }
    }
    T.position(1.8, 0.0, -2.8).scale(0.25, 0.9, 0.25) {
        R.cube() { C.rgba(1.0, 0.35, 0.7, 1.0) EM.on() }
    }

    // Full-body mirror in front of the XR start pose.
    T.position(0.0, 1.25, -4.5).scale(2.4, 2.4, 0.08) {
        R.cube() { Mirror.quality(2048) {} }
    }

    T {
        InputXR.on() {
            InputXRGamepad { locomotion() speed(1.5) }
            T {
                name = "secondary_motion_xr_pose"
                AVC {
                    initial_yaw(3.14159)
                    left_arm_pole_direction([1, -0.35, 1])
                    right_arm_pole_direction([-1, -0.35, 1])
                    hand_rotation_smoothing(220.0)

                    T {
                        GLTF.new("assets/models/bisket.glb") {
                            EM.on()
                            bisket_colliders()
                            bisket_shirt_physics(false)
                        }
                    }

                    T.position(0.0, 0.08, 0.12) {
                        name = "secondary_motion_xr_camera"
                        CXR { Pointer {} }
                    }

                    XRHand.new(true, Left, GripAim).laser() {
                        T {
                            RestAttachment.new("[name='J_Bip_L_Hand']", "[name='J_Bip_L_Middle3']") {
                                Pointer {}
                            }
                        }
                    }
                    XRHand.new(true, Right, GripAim).laser() {
                        T {
                            RestAttachment.new("[name='J_Bip_R_Hand']", "[name='J_Bip_R_Middle3']") {
                                Pointer {}
                            }
                        }
                    }
                }
            }
        }
    }
}

// InputXR/CXR author the tracked pose and camera topology; XR.on() owns the
// OpenXR session lifecycle and requests headset presentation.
XR.on()

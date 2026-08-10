// XR VTuber slide-deck prototype.
//
// Derived from vtuber-mirror-example, with a deliberately lean EditorUI.
// The world and asset panels are omitted because their current draw/list work
// materially affects the VR performance this example is intended to preserve.
//
// Planned controls (manual Animation stepping backend is not implemented yet):
//   B = next slide
//   A = previous slide
//
// Run:
//   cargo run --release --example vtuber-slidedeck

import { bisket_shirt_physics } from "../assets/components/secondary_motion/bisket-shirt-physics.mms"
import { bisket_colliders } from "../assets/components/colliders/bisket.mms"

RendererSettings {
    window_size(640, 480)
}

BGC { C.rgba(0.90, 0.93, 0.98, 1.0) }
AL.rgb(0.20, 0.20, 0.24)
Clock.bpm(60) {}

RenderGraph {
    EmissivePass {
        BlurPass {
            radius_ndc(0.06)
            half_res(true)
        }
    }
    Bloom {
        intensity(0.90)
        radius_ndc(0.06)
        emissive_scale(1.2)
        half_res(true)
    }
}

T.position(0.15, -0.45, 1.0) {
    DL { intensity(1.0) color(1.0, 0.98, 0.95) }
}
T.position(-0.85, 0.55, 0.35) {
    DL { intensity(0.75) color(0.90, 0.94, 1.0) }
}
T.position(0.75, 0.35, -0.75) {
    DL { intensity(0.65) color(1.0, 0.82, 0.90) }
}

// One lightweight editor workspace. An explicit panel list prevents EditorUI
// from materializing its default assets and world panels.
T.position(-2.75, 2.8, -1.5) {
    EditorUI {
        panels([{
            panel = "settings"
            config = {
                show_armature = false
                show_bounds = false
                show_cameras = false
                show_colliders = false
                show_gltf_colliders = false
                show_spring_bones = false
            }
        }])
    }
}

// Temple floor, frame, and mirror retain the useful workload and visual context
// of vtuber-mirror-example without reproducing every decorative primitive.
ED.active() {
    T.position(0.0, -1.75, 0.0).scale(120.0, 0.12, 120.0) {
        Collision.static() {
            CollisionShape.cube([60.0, 0.06, 60.0])
        }
        R.cube() { C.rgba(0.73, 0.71, 0.70, 1.0) Raycastable.enabled() }
    }

    T.position(0.0, -1.55, -0.2).scale(8.0, 0.18, 10.5) {
        R.cube() { C.rgba(0.86, 0.84, 0.82, 1.0) }
    }

    T.position(0.0, 0.55, -4.08).scale(2.55, 2.85, 0.10) {
        R.cube() { C.rgba(0.66, 0.56, 0.34, 1.0) }
    }
    T.position(0.0, 0.55, -3.95).scale(2.30, 2.60, 0.08) {
        R.cube() {
            C.rgba(0.82, 0.88, 0.94, 1.0)
            Mirror.quality(1024) {}
        }
    }

    T.position(-1.1, -0.95, -1.7).scale(0.45, 0.9, 0.45) {
        R.cube() { C.rgba(0.75, 0.28, 0.26, 1.0) EM.on() Raycastable.enabled() }
    }
    T.position(0.0, -0.75, -1.25).scale(0.40, 1.3, 0.40) {
        R.cube() { C.rgba(0.15, 0.70, 0.98, 1.0) EM.on() Raycastable.enabled() }
    }
    T.position(1.1, -0.95, -1.7).scale(0.45, 0.9, 0.45) {
        R.cube() { C.rgba(1.0, 0.84, 0.18, 1.0) EM.on() Raycastable.enabled() }
    }
}

// Slide content is authored once and mounted beneath the controlled avatar
// hierarchy. Every keyframe keeps this same local placement near the model.
let slide_text = Text {
    name = "slide_text"
    "short form video creators hate it when you use this one simple trick!"
    font_size(0.72)
    TextureFiltering.linear()
}
let slide_color = C.rgba(1.0, 0.35, 0.78, 1.0) {
    name = "slide_color"
    EM.on()
    slide_text
}
let slide_root = T.position(-1.45, 0.15, -1.25).rotation(0.0, 3.14159, 0.0).scale(0.055, 0.055, 1.0) {
    name = "avatar_slide_root"
    slide_color
}

// Each slide is state-complete so previous() can reapply an earlier state.
// The transform remains constant: the text stays near the controlled avatar,
// while content, font size, and color change.
let slides = Animation.paused() {
    name = "short_form_slide_deck"

    Keyframe.at(0) {
        slide_root.update_transform([-1.45, 0.15, -1.25], [0.0, 3.14159, 0.0], [0.055, 0.055, 1.0])
        slide_text.set_text("short form video creators hate it when you use this one simple trick!")
        slide_text.set_font_size(0.72)
        slide_color.set_rgba(1.0, 0.35, 0.78, 1.0)
    }

    Keyframe.at(1) {
        slide_root.update_transform([-1.45, 0.15, -1.25], [0.0, 3.14159, 0.0], [0.055, 0.055, 1.0])
        slide_text.set_text("POV: your renderer stopped skinning the same cat five times")
        slide_text.set_font_size(0.66)
        slide_color.set_rgba(0.10, 0.95, 1.0, 1.0)
    }

    Keyframe.at(2) {
        slide_root.update_transform([-1.45, 0.15, -1.25], [0.0, 3.14159, 0.0], [0.055, 0.055, 1.0])
        slide_text.set_text("chat said add mirrors\nso we cached the vertices")
        slide_text.set_font_size(0.82)
        slide_color.set_rgba(1.0, 0.84, 0.18, 1.0)
    }

    Keyframe.at(3) {
        slide_root.update_transform([-1.45, 0.15, -1.25], [0.0, 3.14159, 0.0], [0.055, 0.055, 1.0])
        slide_text.set_text("the GPU has seen enough\none deformation pass is enough")
        slide_text.set_font_size(0.76)
        slide_color.set_rgba(0.42, 1.0, 0.55, 1.0)
    }

    Keyframe.at(4) {
        slide_root.update_transform([-1.45, 0.15, -1.25], [0.0, 3.14159, 0.0], [0.055, 0.055, 1.0])
        slide_text.set_text("like, follow, and subscribe\nfor more suspiciously fast cats")
        slide_text.set_font_size(0.70)
        slide_color.set_rgba(0.72, 0.48, 1.0, 1.0)
    }
}

let xr_gamepad = InputXRGamepad {
    locomotion()
    speed(1.5)
}

T {
    name = "avatar_locomotion_root"
    InputXR.on() {
        xr_gamepad
        T {
            name = "xr_pose"
            AVC {
                initial_yaw(3.14159)
                left_arm_pole_direction([1, -0.35, -1])
                right_arm_pole_direction([-1, -0.35, -1])
                hand_rotation_smoothing(220.0)

                T {
                    GLTF.new("assets/models/bisket.glb") {
                        EM.on()
                        PoseCapture { label("Bisket") asset_name("bisket") }
                        bisket_colliders()
                        bisket_shirt_physics(false)
                    }
                }

                // The slide follows the same locomotion/avatar root but remains
                // independent of individual animated bones.
                slide_root

                T.position(0.0, 0.08, 0.12) {
                    name = "xr_camera_wrapper"
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

slides

on(xr_gamepad, "XrButtonDown", fn(event) {
    if event.control == "ButtonB" {
        slides.next()
        print("vtuber-slidedeck: next (B)")
    } else if event.control == "ButtonA" {
        slides.previous()
        print("vtuber-slidedeck: previous (A)")
    }
})

// The XR runtime supplies both headset eyes. The window remains a companion
// surface rather than adding a separate desktop Camera3D scene view.
XR.on()

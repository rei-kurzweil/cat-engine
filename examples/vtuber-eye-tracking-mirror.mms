// VTuber eye-tracking mirror
//
// A minimal XR avatar scene for validating AVC automatic eye-bone tracking.
// `XREyeTracking` must remain a *direct child* of `AVC`: AVC consumes its
// retained gaze state while the existing XrEyeTrackingUpdated event remains
// available to scripts. In ALVR, configure the VRChat Eye OSC sink to send to
// 127.0.0.1:9000 (or use XREyeTracking.listen(host, port) below).
//
// The mirror is in front of the XR start pose. The bisket asset's humanoid map
// supplies the eye slots automatically; an avatar without mapped eye slots is
// left untouched.

RendererSettings { window_size(960, 720) }
BGC.rgba(0.05, 0.07, 0.12, 1.0)
AL.rgb(0.18, 0.18, 0.22)

RenderGraph {
    EmissivePass { BlurPass { radius_ndc(0.06) half_res(true) } }
    Bloom { intensity(0.8) emissive_scale(1.1) }
}

T.position(1.0, 2.5, 1.5) {
    DL { intensity(1.2) color(1.0, 0.98, 0.95) }
}

ED {
    T.position(0.0, -0.85, -1.5).scale(6.0, 0.12, 6.0) {
        R.cube() { C.rgba(0.20, 0.22, 0.27, 1.0) }
    }

    // Full-body mirror facing the avatar.
    T.position(0.0, 1.25, -4.5).scale(2.4, 2.4, 0.08) {
        R.cube() { Mirror.quality(2048) {} }
    }

    T {
        InputXR.on() {
            InputXRGamepad { locomotion() speed(1.5) }
            T {
                AVC {
                    // OpenXR's rest-forward is -Z.
                    initial_yaw(3.14159)

                    T {
                        GLTF.new("assets/models/bisket.glb") { EM.on() }
                    }

                    // AVC reparents this camera path to the mapped head.
                    T.position(0.0, 0.08, 0.12) {
                        CXR { Pointer {} }
                    }

                    // Direct child: enables automatic mapped left/right eye
                    // bone rotation. No callback or calibration is needed in
                    // Phase 1. For HTC packets, replace with XREyeTrackingHTC.on().
                    XREyeTracking.on()
                }
            }
        }
    }
}

XR.on()

// ALVR eye-tracking transport visualizer (OpenXR-only).
//
// In ALVR select the VRChat Eye OSC face-tracking sink, then run this scene.
// ALVR's “Local OSC port” (normally 9942) is ALVR's own source socket. The
// VRChat Eye OSC sink has a separate destination port; its default is 9000.
// This listener must match that sink destination, not ALVR's local OSC port.
//
// Red = horizontal look, green = vertical look, blue = forward look.  Neutral
// gaze is a muted blue-grey. Center gaze drives both squares; ALVR's per-eye
// LeftRightPitchYaw packet drives each square independently.

RendererSettings { window_size(800, 520) }
BGC { C.rgba(0.025, 0.035, 0.07, 1.0) }
AL.rgb(0.22, 0.24, 0.30)

// No desktop camera/editor panels: OpenXR owns presentation and this is the
// sole camera path. The squares are placed in front of the tracked HMD pose.
InputXR.on() {
    T {
        CXR {}
    }
}
T.position(0.0, 1.5, 2.0) { DL.color(1.0, 0.95, 0.90).intensity(1.2) }

// Reference floor and forward-facing gaze display, two metres in front of
// the tracked HMD. OpenXR cameras look toward negative Z.
T.position(0.0, -1.2, -2.0).scale(4.0, 0.03, 4.0) {
    R.cube() { C.rgba(0.08, 0.11, 0.18, 1.0) }
}

// Thin cuboids facing the HMD: their visible faces lie in the XY plane.
// CXR's local scene origin is already at eye height; floor is below at y < 0.
let left_square = T.position(-0.72, 1.0, -1.5).scale(0.48, 0.48, 0.08) {
    R.cube() { C.rgba(0.22, 0.28, 0.38, 1.0) Emissive.on() }
}
let right_square = T.position(0.72, 1.0, -1.5).scale(0.48, 0.48, 0.08) {
    R.cube() { C.rgba(0.22, 0.28, 0.38, 1.0) Emissive.on() }
}

// `let` retains handles for callbacks but does not itself attach the trees.
// Put both square trees into the authored scene graph.
T {
    left_square
    right_square
}

let eyes = XREyeTracking.on()

// Callback-isolation step: keep the handler registered but perform no world
// mutation. If this presents, `set_color`/render registration is the trigger.
on(eyes, "XrEyeTrackingUpdated", fn(event) {})

// Enable the OpenXR runtime (SteamVR/ALVR).
XR.on()

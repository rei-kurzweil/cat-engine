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

// Small cubes move across each square with the per-eye gaze vector. A pair
// of horizontal rails below shows openness from 0 (left end) to 1 (right).
// The OSC source supplies one combined openness value, so both indicators
// intentionally move together until a per-eye openness transport is used.
let left_vector_marker = T.position(-0.72, 1.0, -1.35).scale(0.07, 0.07, 0.07) {
    R.cube() { C.rgba(1.0, 0.9, 0.25, 1.0) Emissive.on() }
}
let right_vector_marker = T.position(0.72, 1.0, -1.35).scale(0.07, 0.07, 0.07) {
    R.cube() { C.rgba(1.0, 0.9, 0.25, 1.0) Emissive.on() }
}
let left_openness_marker = T.position(-1.12, 0.25, -1.35).scale(0.07, 0.12, 0.07) {
    R.cube() { C.rgba(0.25, 1.0, 0.45, 1.0) Emissive.on() }
}
let right_openness_marker = T.position(0.32, 0.25, -1.35).scale(0.07, 0.12, 0.07) {
    R.cube() { C.rgba(0.25, 1.0, 0.45, 1.0) Emissive.on() }
}

T.position(-0.72, 0.25, -1.5).scale(0.8, 0.025, 0.04) { R.cube() { C.rgba(0.16, 0.22, 0.30, 1.0) } }
T.position(0.72, 0.25, -1.5).scale(0.8, 0.025, 0.04) { R.cube() { C.rgba(0.16, 0.22, 0.30, 1.0) } }

// `let` retains handles for callbacks but does not itself attach the trees.
// Put both square trees into the authored scene graph.
T {
    left_square
    right_square
    left_vector_marker
    right_vector_marker
    left_openness_marker
    right_openness_marker
}

let eyes = VRChatOSCEyeTracking.on()

// Per-eye gaze drives each square's RGB color. ALVR's PitchYaw values become
// normalized look vectors: red/green encode horizontal/vertical gaze and blue
// encodes forwardness.
on(eyes, "XrEyeTrackingUpdated", fn(event) {
    let look = event.combined_look
    if look != null {
        let color = [(look[0] + 1.0) * 0.5, (look[1] + 1.0) * 0.5, (look[2] + 1.0) * 0.5, 1.0]
        left_square.set_color(color)
        right_square.set_color(color)
    }

    let left = event.left_look
    if left != null {
        left_square.set_color([(left[0] + 1.0) * 0.5, (left[1] + 1.0) * 0.5, (left[2] + 1.0) * 0.5, 1.0])
        left_vector_marker.update_transform([-0.72 + left[0] * 0.38, 1.0 + left[1] * 0.38, -1.35 + left[2] * 0.12], [0.0, 0.0, 0.0], [0.07, 0.07, 0.07])
    }

    let right = event.right_look
    if right != null {
        right_square.set_color([(right[0] + 1.0) * 0.5, (right[1] + 1.0) * 0.5, (right[2] + 1.0) * 0.5, 1.0])
        right_vector_marker.update_transform([0.72 + right[0] * 0.38, 1.0 + right[1] * 0.38, -1.35 + right[2] * 0.12], [0.0, 0.0, 0.0], [0.07, 0.07, 0.07])
    }

    let openness = event.combined_openness
    if openness != null {
        let x = (openness - 0.5) * 0.8
        left_openness_marker.update_transform([-0.72 + x, 0.25, -1.35], [0.0, 0.0, 0.0], [0.07, 0.12, 0.07])
        right_openness_marker.update_transform([0.72 + x, 0.25, -1.35], [0.0, 0.0, 0.0], [0.07, 0.12, 0.07])
    }
})

// Enable the OpenXR runtime (SteamVR/ALVR).
XR.on()

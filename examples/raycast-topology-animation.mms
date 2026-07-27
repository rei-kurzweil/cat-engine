RendererSettings { window_size(960, 720) }
BGC.rgba(0.05, 0.06, 0.09, 1.0)
AL.rgb(0.45, 0.45, 0.5)
Clock.bpm(120)

let target = T.position(0.0, 0.0, -2.0) {
    R.cube() {
        C.rgba(0.2, 0.8, 1.0, 1.0)
        Raycastable {}
    }
}
target
let raycaster = Raycast.event_driven()
T.position(0.0, 1.5, 5.0) { C3D { Pointer { raycaster } } }

Animation.looping().length(3.0) {
    Keyframe.at(0.0) {
        target.update_transform([-1.0, 0.0, -2.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
        raycaster.request_raycast()
    }
    Keyframe.at(1.0) {
        target.detach()
        target.update_transform([1.0, 0.5, -2.0], [0.0, 0.8, 0.0], [1.0, 1.0, 1.0])
        raycaster.request_raycast()
    }
    Keyframe.at(2.0) {
        target.set_color([1.0, 0.35, 0.5, 1.0])
        raycaster.request_raycast()
    }
}

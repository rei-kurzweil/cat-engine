RendererSettings { window_size(960, 720) }
BGC.rgba(0.04, 0.05, 0.08, 1.0)
T.position(0.0, 1.5, 6.0) { C3D { Pointer {} } }

let label = T.position(-2.2, 0.0, 0.0) {
    Text.new("singular intent recipients") { font_size(64) C.rgba(0.3, 0.8, 1.0, 1.0) }
}
label
Animation.looping().length(4.0) {
    Keyframe.at(0.0) {
        label.update_transform([-2.2, -0.5, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
        label.set_color([0.3, 0.8, 1.0, 1.0])
    }
    Keyframe.at(1.0) {
        label.update_transform([-2.2, 0.5, 0.0], [0.0, 0.0, 0.0], [1.1, 1.1, 1.1])
        label.set_color([1.0, 0.4, 0.6, 1.0])
    }
    Keyframe.at(2.0) {
        label.update_transform([-2.2, 0.0, 0.0], [0.0, 0.0, 0.0], [0.9, 0.9, 0.9])
        label.set_color([0.55, 1.0, 0.4, 1.0])
    }
    Keyframe.at(3.0) { label.set_color([0.8, 0.5, 1.0, 1.0]) }
}

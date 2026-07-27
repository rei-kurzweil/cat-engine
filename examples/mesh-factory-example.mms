RendererSettings { window_size(960, 720) }
BGC.rgba(0.06, 0.07, 0.1, 1.0)
AL.rgb(0.5, 0.5, 0.55)
T.position(0.0, 2.0, 8.0) { C3D { Pointer {} } }

let root = T {}
for i in range(0, 12) {
    let x = (i % 4) * 1.2 - 1.8
    let y = (i / 4) * 1.2 - 1.2
    let shape = T.position(x, y, 0.0).scale(0.45, 0.45, 0.45) {
        R.sphere() { C.rgba(0.15 + i * 0.05, 0.75, 1.0 - i * 0.05, 1.0) }
    }
    root.attach(shape)
}
root

Animation.looping().length(3.0) {
    Keyframe.at(0.0) { root.update_transform([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]) }
    Keyframe.at(1.0) { root.update_transform([0.0, 0.0, 0.0], [0.0, 0.7, 0.0], [1.0, 1.0, 1.0]) }
    Keyframe.at(2.0) { root.update_transform([0.0, 0.0, 0.0], [0.0, 1.4, 0.0], [1.0, 1.0, 1.0]) }
}

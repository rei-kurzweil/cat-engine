RendererSettings { window_size(960, 720) }
BGC.rgba(0.06, 0.07, 0.1, 1.0)
AL.rgb(0.45, 0.45, 0.5)
Clock.bpm(90)
T.position(0.0, 1.5, 6.0) { C3D { Pointer {} } }

let left = T.position(-1.5, 0.0, 0.0) { name = "left" }
let right = T.position(1.5, 0.0, 0.0) { name = "right" }
let cube = T { R.cube() { C.rgba(0.25, 0.8, 1.0, 1.0) } }
left.attach(cube)
left
right

let prefab = T.scale(0.35, 0.35, 0.35) {
    R.sphere() { C.rgba(1.0, 0.35, 0.55, 1.0) }
}

Animation.looping().length(4.0) {
    Keyframe.at(0.0) { left.attach(cube) }
    Keyframe.at(1.0) { cube.detach() right.attach(cube) }
    Keyframe.at(2.0) { right.attach_clone(prefab) }
    Keyframe.at(3.0) { right.remove_child(1) }
}

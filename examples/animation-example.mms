RendererSettings { window_size(960, 720) }
BGC.rgba(0.04, 0.05, 0.09, 1.0)
AL.rgb(0.35, 0.35, 0.42)
Clock.bpm(120)
T.position(3.0, 5.0, 3.0) { DL {} }
T.position(0.0, 1.5, 6.0) { C3D { Pointer {} } }

let cube = T {
    R.cube() { C.rgba(0.2, 0.7, 1.0, 1.0) EM.on() }
}
cube

let voice = AudioOscillator.square() { frequency(220) amplitude(0.12) enabled(false) }
AudioOutput { voice }

Animation.looping().length(4.0) {
    Keyframe.at(0.0) {
        cube.update_transform([-1.5, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
        cube.set_color([0.2, 0.7, 1.0, 1.0])
        MusicNote.c(3, 0.25, voice)
    }
    Keyframe.at(1.0) {
        cube.update_transform([0.0, 1.0, 0.0], [0.0, 0.8, 0.0], [1.2, 1.2, 1.2])
        cube.set_color([1.0, 0.3, 0.5, 1.0])
        MusicNote.e(3, 0.25, voice)
    }
    Keyframe.at(2.0) {
        cube.update_transform([1.5, 0.0, 0.0], [0.0, 1.6, 0.0], [1.0, 1.0, 1.0])
        cube.set_color([0.5, 1.0, 0.3, 1.0])
        MusicNote.g(3, 0.25, voice)
    }
    Keyframe.at(3.0) {
        cube.update_transform([0.0, -0.5, 0.0], [0.0, 2.4, 0.0], [0.8, 0.8, 0.8])
        cube.set_color([0.8, 0.4, 1.0, 1.0])
    }
}

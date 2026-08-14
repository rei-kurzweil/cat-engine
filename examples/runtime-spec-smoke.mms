// RuntimeSpec cutover smoke scene. This is also loaded by the graphical
// runtime-spec-emissive-cubes example.
mittens.smoke()

RendererSettings.window_size(960, 720) {}

BGC {
    C.rgba(0.015, 0.02, 0.04, 1.0) {}
}
AL.rgb(0.18, 0.20, 0.28) {}
RenderGraph {
    Bloom.intensity(1.2) {}
}

// `T` proves strict RuntimeSpec alias resolution.
T.position(0.0, 0.6, 6.0) {
    C3D {
        Pointer {}
    }
}

T.position(-1.4, 0.0, 0.0).scale(0.75, 0.75, 0.75) {
    R.cube() {
        C.rgba(1.0, 0.12, 0.30, 1.0) {}
        EM.on() {}
    }
}

T.position(0.0, 0.0, 0.0).scale(0.75, 0.75, 0.75) {
    R.cube() {
        C.rgba(0.10, 0.85, 1.0, 1.0) {}
        EM.on() {}
    }
}

T.position(1.4, 0.0, 0.0).scale(0.75, 0.75, 0.75) {
    R.cube() {
        C.rgba(0.75, 0.22, 1.0, 1.0) {}
        EM.on() {}
    }
}

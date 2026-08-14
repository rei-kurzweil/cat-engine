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
        enabled(true)
        fov(55.0)
        near(0.05)
        far(250.0)
        Pointer {}
    }
}

T.rotation(-25.0, -35.0, 0.0) {
    DirectionalLight {
        intensity(1.5)
        color(1.0, 0.92, 0.82)
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

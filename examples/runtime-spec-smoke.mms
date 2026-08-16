// RuntimeSpec cutover smoke scene. This is also loaded by the graphical
// runtime-spec-emissive-cubes example.
//
// The cube parts are deliberately bound with `let` before being assembled.
// That makes this a visible exercise of the live component lifecycle:
// RegisterComponent returns an opaque handle, and a later component body
// attaches that exact handle instead of rematerializing a legacy tree.
mittens.smoke()

RendererSettings.window_size(960, 720) {}

BGC {
    C.rgba(0.015, 0.02, 0.04, 1.0) {}
}
AL.rgb(0.18, 0.20, 0.28) {}
RenderGraph {
    Bloom.intensity(1.8) {}
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

let red_glow = EM.on() {
    name = "red-glow"
    intensity(3.0)
}
let red_mesh = R.cube() {
    name = "red-mesh"
    C.rgba(1.0, 0.12, 0.30, 1.0) {}
    red_glow
    Raycastable.enabled() {}
}
let red_cube = T.position(-1.4, 0.0, 0.0).scale(0.75, 0.75, 0.75) {
    name = "red-cube"
    red_mesh
}

let cyan_glow = EM.on() {
    name = "cyan-glow"
    intensity(2.4)
}
let cyan_mesh = R.cube() {
    name = "cyan-mesh"
    C.rgba(0.10, 0.85, 1.0, 1.0) {}
    cyan_glow
    Raycastable.enabled() {}
}
let cyan_cube = T.position(0.0, 0.0, 0.0).scale(0.75, 0.75, 0.75) {
    name = "cyan-cube"
    cyan_mesh
}

let violet_glow = EM.on() {
    name = "violet-glow"
    intensity(3.6)
}
let violet_mesh = R.cube() {
    name = "violet-mesh"
    C.rgba(0.75, 0.22, 1.0, 1.0) {}
    violet_glow
    Raycastable.enabled() {}
}
let violet_cube = T.position(1.4, 0.0, 0.0).scale(0.75, 0.75, 0.75) {
    name = "violet-cube"
    violet_mesh
}

// State 0 is the authored bright value; state 1 is dimmed. The table is
// captured by all three callbacks and must retain its identity between engine
// frames for repeated clicks to toggle correctly.
let glow_state = {
    a = 0.0
    b = 0.0
    c = 0.0
}

on(red_cube, "Click", fn(event) {
    if glow_state.a == 0.0 {
        glow_state.a = 1.0
        red_glow.set_intensity(0.15)
    } else {
        glow_state.a = 0.0
        red_glow.set_intensity(3.0)
    }
})

on(cyan_cube, "Click", fn(event) {
    if glow_state.b == 0.0 {
        glow_state.b = 1.0
        cyan_glow.set_intensity(0.15)
    } else {
        glow_state.b = 0.0
        cyan_glow.set_intensity(2.4)
    }
})

on(violet_cube, "Click", fn(event) {
    if glow_state.c == 0.0 {
        glow_state.c = 1.0
        violet_glow.set_intensity(0.15)
    } else {
        glow_state.c = 0.0
        violet_glow.set_intensity(3.6)
    }
})

T {
    name = "runtime-spec-cube-gallery"
    red_cube
    cyan_cube
    violet_cube
}

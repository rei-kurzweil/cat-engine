import { voxel_terrain } from "../assets/components/floors/voxel_terrain.mms"

let terrain_palette = [
    [0.30, 0.70, 0.90, 1.0],
    [0.30, 0.32, 0.38, 1.0],
    [0.42, 0.23, 0.12, 1.0],
    [0.42, 0.80, 0.36, 1.0],
]

// Desktop-only voxel terrain editor.
//
// Click the terrain to select it and use the editor gizmo. Move with
// WASD/RF/QE and hold the right mouse button to look around.
//
// Run with:
//   cargo run --release --example terrain-edit-desktop

RendererSettings {
    window_size(960, 640)
}

BGC.rgba(0.62, 0.80, 1.0, 1.0)
AL.rgb(0.18, 0.18, 0.22)

T.position(0.15, -0.45, 1.0) {
    DL {
        intensity(1.1)
        color(0.95, 0.9, 0.85)
    }
}

T.position(0, 1, 0) {
    DL {
        intensity(0.8)
        color(0.95, 0.95, 1.0)
    }
}

// Match the terrain placement used by bisket-vr-demo. Keeping it inside the
// active editor subtree makes the raycastable voxel terrain selectable.
ED.active() {
    T.position(0.0, -6.0, 0.0) {
        name = "voxel_terrain_root"
        voxel_terrain({ palette = terrain_palette })
    }
}

// Keep editor chrome outside the editable scene.
T.position(-2.75, 2.8, -1.5) {
    EditorUI {
        panels([{
            panel = "settings"
        }])
    }
}

// Desktop camera and pointer only; this example intentionally starts no XR
// runtime or XR input systems.
I.speed(3.0) {
    InputTransformMode.forward_z() {
        roll_axis_y()
        fps_rotation()
    }
    T.position(3.1, 1.45, 3.9) {
        name = "desktop_camera_rig"
        C3D {
            Pointer {}
        }
    }
}

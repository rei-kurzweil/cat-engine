# Rigging and controlling humanoid characters

This guide describes the current MMS patterns for controlling a humanoid with desktop input or OpenXR. The examples use `AvatarControl` (`AVC`) to connect cameras and tracked hands to named bones in an imported glTF armature.

## Before you start

The model must contain a skinned armature, and the bone names supplied to `AVC` must exactly match the imported glTF node names. Bisket uses:

```mms
AVC {
    head_bone("J_Bip_C_Head")
    camera_bone("J_Bip_C_Head")
    left_hand_bone("J_Bip_L_Hand")
    right_hand_bone("J_Bip_R_Hand")
}
```

`camera_bone` makes `AVC` place the camera at that bone. It also calibrates the model root vertically from the bone's rest pose. Hand bones allow `XRHand` to resolve the arm chains used by two-bone IK.

## Desktop control with `Input`

Place `Input` above the transform that should move. Put the desktop camera and, if desired, the avatar beneath that transform.

```mms
I.speed(2.0) {
    InputTransformMode.forward_z() {
        roll_axis_y()
        fps_rotation()
    }
    T.position(0.0, 1.6, 3.0) {
        C3D { Pointer {} }

        T.position(0.0, -1.6, 0.0) {
            GLTF.new("assets/models/avatar.glb") {
                EM.on()
            }
        }
    }
}
```

`Input` drives its descendant transform. `InputTransformMode` selects the movement and rotation convention. This is appropriate for keyboard/mouse camera rigs; see `examples/bisket-desktop-demo.mms` for a complete scene.

## XR head and avatar control with `InputXR`

`InputXR` supplies the HMD pose. A typical humanoid hierarchy is:

```text
rig Transform                 optional locomotion target
└── InputXR                   HMD input owner
    ├── InputXRGamepad        thumbstick/buttons
    └── driven Transform      HMD-driven avatar root
        └── AVC               humanoid control
            ├── GLTF          skinned avatar
            ├── CameraXR      reparented to camera_bone
            ├── XRHand Left   left arm IK target
            └── XRHand Right  right arm IK target
```

In MMS:

```mms
T {
    InputXR.on() {
        InputXRGamepad {
            locomotion()
            speed(1.5)
        }

        T {
            AVC {
                head_bone("J_Bip_C_Head")
                camera_bone("J_Bip_C_Head")
                left_hand_bone("J_Bip_L_Hand")
                right_hand_bone("J_Bip_R_Hand")
                initial_yaw(3.14159)

                T {
                    GLTF.new("assets/models/avatar.glb") { EM.on() }
                }

                T.position(0.0, 0.08, 0.12) {
                    CXR { Pointer {} }
                }

                XRHand.new(true, Left, Grip)  { T { Pointer {} } }
                XRHand.new(true, Right, Grip) { T { Pointer {} } }
            }
        }
    }
}

XR.on()
```

The inner transform is driven from the HMD. The outer transform is the persistent XR rig position and is moved by locomotion.

### The outer transform is required for locomotion

`InputXRGamepad` locomotion searches upward from its owning `InputXR` and moves the nearest ancestor `Transform`. Therefore this does **not** move:

```mms
ED {
    InputXR.on() {
        InputXRGamepad { locomotion() }
        T { /* avatar */ }
    }
}
```

`Editor` is not a transform, and the transform below `InputXR` is the HMD-driven transform rather than a locomotion target. Add a rig transform:

```mms
ED {
    T {
        InputXR.on() {
            InputXRGamepad { locomotion() speed(1.5) }
            T { /* avatar */ }
        }
    }
}
```

By default locomotion reads the left thumbstick when it is available, applies a deadzone, rotates movement by HMD yaw, and changes only X/Z. `speed` is in world units per second.

## `XRHand` versus `InputXRGamepad`

They serve different purposes and can be used together:

- `XRHand` consumes a tracked controller or hand pose. Under `AVC`, `GripAim` hands prefer active, valid controller actions and use grip position with aim rotation; wrist/palm tracking takes over automatically when those actions are inactive or unavailable.
- `InputXRGamepad` consumes controller buttons and analog axes. With `locomotion()`, it moves the rig transform. It can also emit XR button and axis events to MMS handlers.
- `InputXR` owns the HMD/controller input context. Both components must be descendants of the relevant `InputXR`.

Use `.laser()` for a controller-space ray. For an avatar-aligned hand and ray, declare the basis
beneath the avatar GLTF and the fingertip attachment independently beneath `XRHand`:

```mms
GLTF.new("avatar.glb") {
    JointRetargetBasis.new(
        "[name='J_Bip_L_Hand']",
        "[name='J_Bip_L_Middle1']",
        "[name='J_Bip_L_Middle3']",
        "[name='J_Bip_L_Little1']",
        "[name='J_Bip_L_Index1']",
    )
}

XRHand.new(true, Left, GripAim).laser() {
    T {
        RestAttachment.new("[name='J_Bip_L_Hand']", "[name='J_Bip_L_Middle3']") {
            Pointer {}
        }
    }
}
```

AVC and PointerSystem both query the retained basis for the resolved hand joint. The attachment
computes only the fingertip origin from immutable rest data. Missing, invalid, or conflicting
bases remain visible and do not fall back to inferred XR landmark geometry. Wrist/palm takeover
currently uses identity correction until articulated-hand retargeting has its own basis path.
Keep the driven `T` directly beneath `XRHand`; wrapping that transform in `RestAttachment` prevents
OpenXR and AVC from finding the tracked target.

For a seated experience, omit `InputXRGamepad` or disable locomotion. For head tracking without avatar arms, omit `XRHand`. `XR.on()` is still required to initialize the OpenXR runtime.

## Arm configuration and debugging

Pole directions are authored in body-local space and control which way elbows bend:

```mms
left_arm_pole_direction([1, -0.35, 1])
right_arm_pole_direction([-1, -0.35, 1])
hand_rotation_smoothing(220.0)
```

Set `CAT_DEBUG_XR_HAND_ALIGNMENT=1` to print rate-limited raw grip/aim, selected source, derived basis, applied correction, and final alignment diagnostics.

`ik_debug()` draws IK diagnostics and is useful while calibrating a rig, but it adds visible geometry and measurable runtime work. Do not leave it enabled when evaluating normal rendering performance.

Working references are `examples/bisket-vr-only-example.mms`, `examples/bisket-vr-demo.mms`, and `examples/input-xr-gamepad.mms`.

## Troubleshooting

- **Thumbstick events work but the rig does not move:** ensure a `Transform` is an ancestor of `InputXR`.
- **The camera tracks but the avatar does not:** ensure the driven transform and `AVC` are descendants of the same `InputXR`.
- **Hands track but elbows bend incorrectly:** adjust the body-local pole directions.
- **Wrists are twisted while using controllers:** use `GripAim` with a valid three-joint middle-finger chain and inspect `CAT_DEBUG_XR_HAND_ALIGNMENT=1` output.
- **No XR input:** include `XR.on()` and verify that the OpenXR runtime reports active input.

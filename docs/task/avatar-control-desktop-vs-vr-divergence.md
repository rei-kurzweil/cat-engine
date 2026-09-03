# Task: AvatarControl Desktop vs VR Divergence

Fix the "head-at-feet" anatomical collapse in desktop examples by making
`AvatarControlSystem` (init) and `HeadPoseBodyXzFollowSystem` (tick)
distinguish between VR/XR and Desktop input drivers.

## Problem statement

The current `AvatarControl` (AVC) implementation uses a **Rigid Eye-Driven** model
derived from VR requirements:

1.  The head bone is spliced out of the armature and parented to the pose driver.
2.  The body is translated every tick to sit under the head bone.

While correct for VR (where the HMD is the world anchor), this causes
**anatomical collapse** on Desktop:

- The `Input` (Desktop) driver sits at world `y=0`.
- The head is moved to `y=0`.
- The body follow logic moves the model root to sit under the head at `y=0`.
- Result: Feet at 0, Head at 0.

## Desktop-facing inventory (PC-Rei / `vtuber-desktop`)

This is also a facing-convention problem, not merely a vertical-placement
problem. The following observations were made with the PC-Rei VRoid model in
`examples/vtuber-desktop.mms` using this desktop driver:

```mms
I.speed(1.5) {
    InputTransformMode.forward_z() { fps_rotation() roll_axis_y() }
    T { AVC { /* model_root → GLTF */ } }
}
```

`InputTransformMode.forward_z()` maps W to local `-Z`. The three useful
reference configurations are:

| Configuration | Immediate visible result | Movement / later result |
|---|---|---|
| AVC defaults (no facing overrides) | Head and locomotion agree, but PC-Rei visibly faces away from the desktop camera. | W moves in the visible head-facing direction. |
| Extra `T.rotation(0, pi, 0)` between the input-driven `T` and `AVC` | The head immediately faces the expected direction. | The body and W direction do not immediately change; body yaw catches up only after the yaw-follow threshold/rate is exercised by input. |
| `AVC { initial_yaw(0) }` with no extra wrapper | The body initially faces the expected direction, toward the camera. The head remains 180 degrees away. | W remains aligned with the head-facing direction, so locomotion is internally coherent but the head is visually backward relative to the body/camera. |

These are observations, not a recommended scene configuration. In particular,
the wrapper rotation changes the head branch immediately but does not set the
body yaw-follow state; `initial_yaw` changes only that body state. The current
authoring surface therefore has no single supported desktop setting that
expresses “rotate this whole avatar, including its visible head, to face this
way at rest.”

### Confirmed current topology

At AVC initialization, the implementation currently creates two independent
runtime branches for **both** `InputComponent` and `InputXRComponent` ancestry:

```text
Input / InputXR
  └── driven_t
        ├── AVC
        │     └── TransformForkTRS → QuatYawFollow(initial_yaw) → model_root → GLTF body
        └── head_target → mapped J_Bip_C_Head
```

The mapped head is unconditionally reparented from the armature beneath
`head_target`; the body remains under the yaw-follow stream. The head mount
uses the driver branch directly, while the body rotation is stateful and
threshold/rate limited. This is why a parent/wrapper rotation can affect the
head immediately while the body retains its old initial yaw.

This is a transform-tree split, not evidence that the GLTF has two separately
authored heads. It also means the desktop problem cannot be solved robustly by
choosing only `initial_yaw`, only `forward_plus_z`, or only an extra wrapper
rotation: each affects a different layer of the runtime topology.

### Scene-only authoring candidate — not yet the engine default

We are **not blocked from making a focused desktop reference scene coherent**
without changing engine code. The important rule is that the initial rotation
must live on the `TransformComponent` that is the **direct child of `Input`**,
not on an additional transform inserted between that child and `AVC`.

The derived candidate for PC-Rei is:

```mms
I.speed(1.5) {
    InputTransformMode.forward_z() {
        fps_rotation()
        roll_axis_y()
    }
    // This is Input's controlled transform: InputSystem uses its rotation for W/S.
    T.rotation(0.0, 3.14159, 0.0) {
        AVC {
            // Seed the separate body-yaw branch to the body-facing result.
            initial_yaw(0.0)

            T.position(0.0, -1.6, 0.0) {
                GLTF.new("assets/models/pc-rei.hoodie.glb") { /* … */ }
            }
        }
    }
}
```

Why this candidate differs from the observed wrapper experiment:

```text
reported:  Input → T (controlled, identity) → T.rotation(pi) → AVC
candidate: Input → T.rotation(pi) (controlled) → AVC
```

`InputSystem` finds only the first direct transform child and uses that
transform's rotation both for FPS yaw state and for W/S translation. Therefore
the reported inner wrapper can turn the head branch without rotating the input
movement basis. Putting the rotation on the direct child makes the initial
head direction and W/S basis agree; `initial_yaw(0)` supplies the matching
initial value for the otherwise independent body yaw-follow branch.

This is a **testable authoring workaround**, not a claim that it is the desired
public contract. It should be verified with neutral pose, W/S before any mouse
input, and after mouse yaw passes the body-follow threshold. If it does not
remain coherent through that sequence, record the measured transform values
rather than adding another compensating wrapper.

### API sketch: do not start with `Input.initial_pose`

`InputComponent` currently owns only speed; its direct transform child already
holds the initial translation, rotation, and scale. An
`Input.initial_pose(TransformTrs)` builder would duplicate that state and
leave an ambiguous question: does it replace, compose with, or merely seed the
authored direct-child transform?

The immediate need is not a new Input API. It is to author the existing direct
child transform correctly. If a convenience API is later justified, it should
be a scene-construction shorthand for that direct child, not independent pose
state on `Input`.

The longer-term engine API belongs at AVC's semantic boundary: an explicit
initial avatar-facing intent would have to seed both the direct head-mount path
and the body yaw-follow state atomically. That can remain desktop-specific
internally while retaining the existing XR behavior and keeping advanced
per-asset overrides available.

### Current-code correction

The proposed strategy below still uses `AimConstraint` terminology, but the
current AVC initialization path does **not** install an AVC-owned head
`AimConstraint` for this splice. It creates a `head_target` transform and
attaches the mapped head directly beneath it. Any implementation should start
from that actual topology rather than assuming an existing constraint can be
switched from `copy_position: true` to `false`.

Relevant implementation points:

- `InputSystem` maps W to `-Z` for `forward_z`.
- `AvatarControlSystem::try_init_splices` creates the body yaw stream and the
  separate `driven_t → head_target → head` attachment for every driver.
- `HeadPoseBodyXzFollowSystem` currently has no desktop-mode early return.

## Target design: PoseDriverHandler Strategy

Instead of scattered conditional logic, AVC will delegate its driver-specific behavior to an internal strategy. This encapsulates how the system applies the pose driver to the avatar's hierarchy.

### AvatarControlPoseDriverHandler Lifecycle
Each handler implements two primary lifecycle methods:

1.  **`handle_init(world, avc, emit)`**:
    - Performed once when the system discovers the AVC needs initialization.
    - Handles **Hierarchy Topology**: How the `head_bone` is integrated (Splice vs In-place).
    - Installs **IK Constraints / Transform Streams**: Configures the head `AimConstraint`, body pipeline, etc.
    
2.  **`handle_pose(world, avc, emit, dt)`**:
    - Performed every tick.
    - Figures out how to manipulate all the transforms it owns based on the pose driver's current state.
    - For VR: Implements the head-rotation-compensated body follow rule.
    - For Desktop: Ensures the head and body stay synchronized with the local driver's translation.

---

### 1. Eye-Driven Handler (VR/XR)
- **Driver:** `InputXRComponent`
- **`handle_init`**: **Rigid Splice**. Re-parents the `head_bone` to a child of the pose driver (`head_target`). Configures `AimConstraint` with `copy_position: true`.
- **`handle_pose`**: Executes the `HeadPoseBodyXzFollowSystem` rule (`HMD - R_h * v_local`) to translate the body under the head.

### 2. Body-Driven Handler (Desktop)
- **Driver:** `InputComponent`
- **`handle_init`**: **Soft/In-place Splice**. Keeps `head_bone` as a child of the neck. Injects `head_mount` in-place. Configures `AimConstraint` with `copy_position: false`.
- **`handle_pose`**: Likely a no-op or simple sync. The body inherits translation from the `Input` driver (locomotion) via standard parenting; the head inherits from the body.

---

## Implementation Steps

### Phase 1 — Driver Identification & Handler Selection
- Add `driver_kind: AvatarDriverKind` to `AvatarControlComponent`.
- Enum `AvatarDriverKind { VR, Desktop }`.
- In `AvatarControlSystem::try_init_splices`, identify the driver type by ancestor lookup.

### Phase 2 — Conditional Hierarchy Setup
- Update `try_init_splices` to use the `driver_kind` to decide:
    - Whether to emit an `Attach` intent moving the head to `head_target`.
    - How to configure the `AimConstraint` (copy position or not).

### Phase 3 — Conditional Tick Logic
- Update `HeadPoseBodyXzFollowSystem::tick_one` to short-circuit if `driver_kind == Desktop`.
- Ensure `QuatYawFollow` (in the body pipeline) still runs for both.

### Phase 4 — Example Verification
- Verify `examples/vtuber-desktop.mms` works without `camera_bone` calibration.
- Verify `examples/vtuber-desktop-first-person.mms` allows the camera to be carried by the avatar's head.
- Verify `bisket-vr-demo.mms` still works correctly in VR.

## Acceptance Criteria

- `vtuber-desktop.mms` displays the avatar at normal height (head on shoulders).
- `bisket-vr-demo.mms` still works correctly in VR.
- Head rotation follows input in both modes.
- Desktop body yaw follow (lag/threshold) still works.
- First-person desktop camera inherits avatar head motion (including any animations/IK).

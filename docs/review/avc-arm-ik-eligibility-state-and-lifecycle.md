# AVC arm IK eligibility: state, materialization, and lifecycle

Date: 2026-08-12

Status: review of the implemented driver-gated arm IK path

## Short answer

`ArmIkBinding` is **not cached** on `AvatarControlComponent`.

It is a temporary, initialization-only value containing the five component IDs
that have already passed eligibility checks for one arm:

```rust
struct ArmIkBinding {
    controller: ComponentId,
    raw_target: ComponentId,
    upper_arm: ComponentId,
    lower_arm: ComponentId,
    hand: ComponentId,
}
```

Only `ArmIkEligibility::Eligible` carries a binding:

```rust
enum ArmIkEligibility {
    Eligible(ArmIkBinding),
    IncompleteArmMap,
    NoHandDriver,
    MalformedHandDriver,
}
```

The binding lives in local variables during `try_init_splices`. It is consumed
to create the hand target and `IKChainComponent`, and selected IDs from it are
copied into AVC's existing runtime fields. The binding and eligibility enum are
then dropped.

There is no new long-lived `arm_ik_enabled` field. Whether a side uses arm IK is
materialized in runtime topology and target IDs:

| Runtime result | Inactive side | Active side |
| --- | --- | --- |
| Cached hand bone ID | `None` | `Some(hand)` |
| Cached raw target ID | `None` | `Some(controller transform)` |
| Cached visual target ID | `None` | `Some(raw or correction target)` |
| Generated `TwoBoneIK` chain | absent | present |
| Bone behavior | imported FK/rest/animation | IK while the XR pose is valid |

The generated chain is the authoritative state consumed by `IKSystem`. The AVC
target fields support hand-basis correction and other runtime consumers; they
are not a separate enable switch.

## Why the binding exists

Before this change, arm bone availability and arm IK activation were treated as
the same thing. A mapped hand without a controller could receive a newly created
identity transform as its target. That transform sat at the world origin, so a
desktop avatar's arms were solved toward the origin.

`ArmIkBinding` creates a boundary between validation and mutation. Code that
constructs IK receives either:

- a complete bundle that contains every ID it needs; or
- no bundle, in which case it must not create a target or chain.

This avoids passing several independent `Option<ComponentId>` values deeper
into construction, where an absent target could accidentally acquire another
fallback.

## Eligibility inputs

`classify_arm_ik_eligibility` is non-mutating and runs independently for the
left and right sides. It checks:

1. The humanoid report resolves the side's upper arm.
2. The report resolves the side's lower arm.
3. The report resolves the side's hand.
4. AVC has a direct `ControllerXRComponent` child for that side.
5. The controller is enabled.
6. The controller has the direct tracked `TransformComponent` child expected by
   `OpenXRSystem`.

The result has these meanings:

| Classification | Meaning | Initialization action |
| --- | --- | --- |
| `Eligible(binding)` | Complete mapped arm and usable XR target topology | Prepare basis, create target, cache IDs, create chain |
| `IncompleteArmMap` | At least one required arm slot is unresolved | Leave side as FK/rest/animation |
| `NoHandDriver` | No matching controller, or it is disabled | Leave side as FK/rest/animation |
| `MalformedHandDriver` | Controller exists but lacks its direct tracked transform | Warn and leave side as FK/rest/animation |

These classifications describe **initialization capability**, not whether a
tracked pose is valid on the current frame.

## Initialization flow

```text
AvatarControlSystem::tick_one
  |
  | head_mount is None
  v
request HumanoidBoneMapReport and wait for head readiness
  |
  v
try_init_splices
  |
  +-- discover left/right direct ControllerXR children
  |
  +-- classify left side -----------------------------+
  |                                                   |
  |   ineligible                                      | Eligible(binding)
  |      |                                            v
  |      +--> no target, no cached hand IDs     prepare hand basis/correction
  |                                                   |
  |                                                   v
  |                                             resolve_hand_target
  |                                             (raw target or correction child)
  |                                                   |
  +-- classify right side independently               v
  |                                             cache selected runtime IDs
  |                                                   |
  |                                                   v
  |                                             create TwoBoneIK chain
  |                                             under AVC
  v
create head/body runtime topology and set head_mount
  |
  v
future ticks skip try_init_splices
```

`resolve_hand_target` cannot manufacture a target. It accepts an
`ArmIkBinding`, uses `binding.raw_target`, and may add only a rotation-correction
transform beneath that real tracked target.

## State after initialization

### Temporary state that does not survive initialization

- `ArmIkEligibility`
- `ArmIkBinding`
- the local `left` and `right` prepared-arm tuples

These values organize construction and enforce its preconditions. They are not
component state and are not queried on later ticks.

### Runtime state retained on `AvatarControlComponent`

For each eligible side, initialization fills:

- `left/right_hand_bone_id`;
- `left/right_hand_raw_target_id`;
- `left/right_hand_visual_target_id`;
- `left/right_hand_aim_correction`, when a retained correction exists.

For an ineligible side these remain `None`. No new fields were introduced by
the eligibility change; the existing fields now have stricter population
rules.

An aim correction being `None` does not mean IK is inactive. Some eligible rigs
use the raw target directly and require no correction. The reliable solver-side
evidence of active IK is the generated `IKChainComponent`.

### Runtime state retained in the world topology

An eligible side creates:

- optionally, a correction `TransformComponent` under the controller's tracked
  transform;
- one `IKChainComponent` configured with `IKSolver::TwoBoneIK`;
- a non-serialized marker beneath generated runtime components.

The IK chain stores the durable solver inputs that came from the binding:

- `root_joint_id = upper_arm`;
- `mid_joint_id = lower_arm`;
- `end_effector_id = hand`;
- `target_id = visual target`;
- `xr_pose_driver = owning ControllerXR`, discovered from the target ancestry.

The chain is parented beneath AVC for ownership and subtree cleanup. `IKSystem`
does not depend on that parent relationship when solving.

## Per-frame behavior is not reclassification

After initialization, AVC does not repeatedly decide whether arm IK exists.
Two narrower runtime mechanisms operate on the already-created state:

1. `update_hand_pose_corrections` reads the cached raw/visual target IDs. When
   the controller's active pose source changes, it applies either the retained
   hand-basis correction or identity to the correction target.
2. `IKSystem` checks the chain's `xr_pose_driver` pose validity. An invalid or
   temporarily unavailable XR pose prevents that frame's solve; it does not
   delete the chain or change eligibility.

This gives two separate concepts:

| Concept | Frequency | Effect |
| --- | --- | --- |
| Arm IK eligibility | Once at AVC initialization | Determines whether the side owns a chain at all |
| XR pose validity | Every solver tick | Determines whether an existing chain may write bone rotations this frame |

Temporary tracking loss therefore does not make the arm target the origin and
does not require rebuilding the avatar.

## Cache and refresh lifecycle

AVC initialization is latched by `head_mount`:

```rust
let needs_init = avc.head_mount.is_none();
```

Once `try_init_splices` completes and stores `head_mount`, eligibility is not
classified again for that AVC instance.

Current consequences:

- A desktop AVC initialized without XR hand children remains non-IK.
- Adding an XR hand controller after initialization does not add arm IK.
- Removing a controller after initialization does not tear down its chain, but
  the missing `xr_pose_driver` component makes the solver's validity lookup
  fail closed. Disabling an existing controller also does not tear down or
  reclassify the chain; because disabling currently does not itself clear a
  previously true `pose_valid`, it must not be treated as a reliable runtime
  arm-IK off switch.
- A newer `HumanoidBoneMapReport::generation` currently refreshes only the
  camera anchor path. It does not rebuild arm chains or replace their joint IDs.
- Replacing/removing and recreating the GLTF/AVC subtree creates a fresh AVC
  instance, so normal initialization and classification run again.

The current effective cache is therefore the materialized component topology
plus AVC's runtime IDs, with the lifetime of one initialized AVC subtree.

## What a future manual refresh must do

A refresh cannot safely consist of calling `classify_arm_ik_eligibility` again.
Classification is read-only, but construction creates components and caches
IDs. Re-running construction without cleanup could leave duplicate chains,
orphaned correction targets, or chains referencing old GLTF joints.

A proper refresh/rebind seam should, per side:

1. Identify and remove the old AVC-owned IK chain.
2. Remove the old generated correction target, if one exists.
3. Clear all cached hand IDs and corrections for that side.
4. Obtain the current humanoid-map generation and controller topology.
5. Classify again.
6. Materialize the new result exactly once.

That operation could later be triggered by explicit UI action, GLTF subtree
replacement, a supported asset-reload event, or a controller-topology
generation. Until such teardown exists, one-time classification is the safer
lifecycle.

## Concrete desktop and XR outcomes

For `vtuber-desktop` with PC-Rei:

```text
complete VRoid arm map + no ControllerXR child
  -> NoHandDriver
  -> no hand target
  -> no TwoBoneIK chain
  -> imported/startup pose owns the arms
```

For an XR AVC with valid left and right hand topology:

```text
complete arm map + enabled ControllerXR { direct tracked Transform }
  -> Eligible(ArmIkBinding)
  -> target IDs cached
  -> TwoBoneIK chain created
  -> IKSystem solves only on frames where that XR pose is valid
```

Mixed configurations are supported: one side can be eligible while the other
remains entirely FK-driven.

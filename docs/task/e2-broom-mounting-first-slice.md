# Task: E2 broom attachment first slice

## Status and outcome

Planned, 2026-09-07. Documentation only so far; the proposed API and behavior
below are not implemented. Part of
[interaction zones and vehicle mounting](release-zones-sockets-and-vehicle-mounting.md).

Use the actual broom in [E2](../../examples/e2.mms) to validate one complete
attachment loop: grab the broom, bring it near the avatar's legs, release to
mount, inspect the alignment in the mirror, and dismount so the test repeats.
This first slice validates attachment and ownership. Flight physics and a
finished riding animation are subsequent work, not acceptance requirements.
The [flight follow-up](broom-flight-followup.md) tracks the velocity driver,
keyboard/gamepad events, and a separate vehicle example based on E2.

## Verified scene baseline

- `broom` is a transform at `(2, 5, -2)` with `Grabbable {}` and
  [broomstick.glb](../../assets/models/broomstick.glb). The asset is present.
- Bisket has desktop avatar control, WASD locomotion, mouse FPS rotation,
  and a first-person camera attached to the head after `GLTFInitialized`.
- `e2_first_person_camera` contains `C3D { Pointer {} }`. The avatar input is
  `e2_desktop_avatar_input`, with `e2_avatar_head_driver` beneath it.
- The scene has a full-body mirror and no XR or live hand tracking. This is
  the untracked humanoid case, not the camera-only case.

Launch from the repository root:

```sh
cargo run --release -- load examples/e2.mms
```

These facts come from scene inspection; the mounting flow has not been run.

## Smallest component contract

Keep `Grabbable` and add `Mountable` to the same broom parent transform. Each
owns one attachment configuration. They may share an internal `AttachmentRule`
struct, with the owning component determining attachment direction; sharing
the struct does not require identical defaults or irrelevant mandatory fields.
Existing `Grabbable {}` pickup behavior must remain valid.

The broom's mount configuration specifies:

- activation: intentional release of this user's actively held broom;
- eligibility: a broom-local authored probe point inside this user's leg zone
  and outside their torso exclusion volume;
- alignment: this user's authored rider anchor aligns in position and
  orientation to the broom's authored riding anchor;
- participants: the held broom and the explicitly associated avatar movement
  root, not the pointer camera or an arbitrary humanoid in the scene.

Zones own geometry and their coordinate frame/placement. The attachment
configuration on `Mountable` owns zone references, activation, eligibility
conditions, and anchor references. Zone geometry does not decide the action.
Use ordinary named transforms for the anchors and probe. A single authored
probe avoids requiring full broom-bounds containment in a small leg zone.
For this slice, test point containment in authored boxes; exclusion wins at
the boundary. Tune dimensions and transforms against the actual asset and
Bisket rather than assuming its root origin or model axis is suitable.

Keep multiple alternative configurations per component open as a future
extension, for example release near legs versus grabbing a mount handle.
Do not require a rule list, public `InteractionAction`, proximity activation,
or car-entry support to deliver this slice. Exact MMS builder names remain
open; implement one declarative configuration on each component rather than
example-specific event handlers that bypass the attachment contract.

## Implementation boundary

1. Resolve the initiating pointer's avatar and the actual movement authority
   in E2's input/AVC topology. Document the chosen root and source anchor;
   do not assume `e2_avatar_head_driver` alone owns all movement. Wait for
   required GLTF/rig references to resolve before enabling mounting.
2. Author the leg volume, torso exclusion, broom probe, and alignment anchors
   in E2. Add toggleable visible zone/probe/anchor markers and eligible/mounted
   feedback that do not intercept pointer hits. Position the broom so pickup
   and mirror inspection are practical and repeatable.
3. Make desktop positioning sufficient to reach the leg zone. Use the live
   scroll-distance work from
   [grab placement](grab-hand-relative-bounds-placement.md) as a prerequisite
   or a bounded dependency of this slice. Verify that looking down and changing
   distance can place the probe in the volume; depth adjustment alone is not
   proof of reachability. Do not require polished hand IK before this test.
4. While held, entering the volume only previews eligibility. On intentional
   release, revalidate the rule and choose exactly one outcome: ordinary drop
   or mount. Cancellation and pointer removal never request mounting.
5. On mount, detach the broom from its hand/pointer basis, preserve its world
   pose, then align the avatar's rider anchor to its riding anchor. Prevent
   structural and effective-transform cycles and any intermediate restoration
   to the old grab parent. Failed release handoffs leave a valid ordinary drop.
6. For this attachment-only slice, keep the broom stationary in an independent
   world basis while mounted. Suspend competing avatar translation/body-turn
   drivers; preserve usable camera look without moving the mount or feeding
   motion back through the held relationship. Do not claim flight support.
   A small debug translation/rotation of the mounted broom must carry the rider
   and camera correctly; this verifies following rather than a one-time snap.
7. Provide an explicit documented desktop dismount binding. Dismount restores
   movement ownership and a valid independent avatar world pose, with an
   authored exit offset clear of the broom. Leave the broom available to grab
   again. Target/anchor removal and scene teardown must clear attachment state
   and restore control. Regrabbing an occupied broom is rejected in this slice;
   dismount first, then grab again.

## Manual acceptance in E2

Record the final bindings and authored anchor/zone values with the implementation
so another person can repeat this flow without adjusting source code.

1. Launch E2. Grab the broom using the configured desktop grab gesture. It
   follows the pointer as before; other grabbable scene props still work.
2. Use look and scroll to bring it toward the legs. Confirm visible eligibility
   agrees with the probe and zone markers. Holding it there does not mount.
3. Move outside the leg zone, or into the torso exclusion, and release. It
   drops normally. Repeat with a valid leg-zone release: exactly one mount
   occurs, the hand attachment ends, and the rider aligns to the riding anchor.
4. Inspect in the mirror and first-person view. Alignment uses authored source
   and destination anchors, including orientation, rather than either root
   origin. No cycle, jump back to the grab parent, or camera feedback occurs.
5. Apply a debug translation and rotation to the broom. Rider and camera follow.
   Check that normal movement input cannot separately drive the mounted avatar.
6. Dismount using the documented binding. Movement and look work again, and
   the broom can be picked up. Repeat the complete loop at least three times.
7. Cancel an eligible grab, then test removal of a required mount reference.
   Neither produces a partial mount or leaves controls stuck. These failure
   paths may use focused lifecycle tests if awkward to trigger in the scene.

## Deferred and completion boundary

Defer flight forces, vehicle collision policy, velocity inheritance, XR input,
camera-only riders, automatic proximity mounting, car affordances, multiple
rules/seats/riders, mouth sockets, and finished reaching/riding pose transitions.
Preserve their design paths without making them prerequisites here.

Completion requires the repeatable E2 loop and focused verification of the
handoff, fallback, and cleanup. It does not close the broader mounting task or
the grab-placement/pose tasks. A successful static mount is not evidence that
physics-driven riding is implemented.

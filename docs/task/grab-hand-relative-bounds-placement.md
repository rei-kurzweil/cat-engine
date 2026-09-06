# Task: hand-relative, bounds-aware grab placement

## Status and outcome

Planned. Part of [grabbing, poses, and release zones](epic/grabbing-poses-and-release-zones.md).

Pointing at and grabbing a grabbable component tree brings it toward the
initiating hand. At rest, the boundary of its measured bounds meets the hand
anchor at zero clearance, or leaves an explicitly configured gap. This works
for XR and desktop, including desktop without hand tracking.

## Current implementation and gap

See [GrabbableSystem](../../src/engine/ecs/system/grabbable_system.rs) and
[PointerComponent](../../src/engine/ecs/component/pointer.rs).

The system already measures subtree-local bounds, projects their transformed
corners along the pointer ray, and uses `Pointer.min_grab_distance` to place
the nearest projected boundary at a clearance from the ray origin. Defaults
are 0.05 m for controller-driven pointers and 0.75 m otherwise. It preserves
the target origin's transverse displacement and eases translation toward a
destination calculated at grab start.

This is useful groundwork, but a supporting plane along the ray is not
necessarily contact with the hand point. The ray origin can differ from the
hand, and transverse displacement can leave the object beside it. The current
distance control exists; this ticket defines hand-relative placement and its
authoring contract rather than introducing the same setting again.

## Proposed contract

- Resolve the movable transform for the selected grabbable component tree.
  Preserve attachment to the initiating pointer's nearest ancestor transform.
- Resolve a hand/contact anchor independently from the pointing ray. XR uses
  an appropriate tracked/controller hand reference; desktop uses an explicit
  synthetic anchor coordinated with the grab pose. Specify coordinates and
  how the anchor moves while held.
- Measure the whole movable tree, including descendant transforms, and place
  a selected boundary contact point at the anchor plus configured clearance.
  Account for rotation, scale, and off-center origins. Define lateral alignment
  as well as depth; projected depth alone does not guarantee point contact.
- Reconcile the existing pointer clearance setting with this anchor contract.
  Permit zero clearance and retain configurable distance. Final API and default
  migration are open; avoid silently changing existing authored distances.
- Preserve orientation initially unless an authored grip requests another
  orientation. Allow an authored grip/bounds override for unsuitable automatic
  contact, such as long-handled props.
- Specify whether bounds/contact are frozen during a grab or refreshed when
  geometry, scale, or the anchor changes. Avoid jitter and discontinuities.
- Missing bounds must have an explicit fallback, such as an authored grip or
  documented origin placement; do not report a guessed placement as flush.

Static/rigid bounds are the first target. Skinned humanoid bounds may poorly
fit the current pose; accurate deformed-body contact is deferred. Bounding-box
contact also does not promise exact mesh-surface or finger contact.

## XR stick adjustment and shared automatic behaviors

Verified existing configuration: [InputXRGamepadComponent](../../src/engine/ecs/component/input_xr_gamepad.rs)
already combines XR gamepad input with automatic locomotion, enabled by default.
Rust exposes `locomotion()`, `locomotion_enabled(bool)`, `hand(...)`, `speed(...)`,
and `deadzone(...)`; MMS exposes the locomotion toggle as `.locomotion(bool)`.
The [input system](../../src/engine/ecs/system/input_xr_gamepad_system.rs) moves
the nearest ancestor transform above the owning `InputXR`. AvatarControl's
automatic movement-target resolution uses that same resolver when under
`InputXR`; it does not select an arbitrary avatar elsewhere in the tree.
Default/either hand preference chooses the left stick if available, otherwise
the right; explicit left/right preferences are supported.

Decision: extend `InputXRGamepad` with builder options for minimum grab
levitation distance adjustment. It is already general enough to configure
automatic locomotion and interaction together; no additional automatic-behaviors
component is needed. Do not introduce one component per behavior. Exact builder
method names and whether adjustment is enabled by default remain open, but it
must be configurable.

While a grab is active, holding the other stick up/down continuously changes
the active pointer's effective `min_grab_distance`: proposed mapping is up
moves the held tree farther away, down brings it closer. Normally locomotion
uses left and distance uses right; derive the assignment from the configured
locomotion stick rather than hard-coding right. Resolve unavailable-controller
fallbacks without letting the same axis both move the rig and change distance.
If no spare stick exists, require a remap or leave automatic adjustment inactive.

- Apply a deadzone, configurable distance-change rate, elapsed-time scaling,
  and finite distance limits with zero as the minimum allowed clearance.
- Adjust only while holding a successfully grabbed tree; idle stick input must
  not change grab distance. Release/cancellation stops adjustment immediately.
- Recompute the held destination as distance changes and move smoothly. The
  current destination is captured at grab start, so changing the pointer field
  alone does not deliver this behavior.
- Scope adjustment to the intended pointer/grab within the same XR rig. Define
  explicit arbitration for simultaneous two-hand grabs; one shared stick must
  not accidentally adjust both. Selection policy remains open.
- Specify whether the adjusted distance persists for the next grab or is a
  temporary override of the authored pointer setting; do not silently mutate
  the authored default. This lifetime policy remains open.
- Reserve only the required axis while grabbing and define conflicts with other
  automatic interactions or custom bindings in the shared configuration.

Acceptance: with locomotion assigned left, right-stick vertical input adjusts
a held tree's distance while left-stick locomotion still works. Swapping the
locomotion assignment swaps the adjustment stick. Centering stops adjustment;
release stops it; idle input leaves distance unchanged. Verify deadzone,
frame-rate independence, limits, two-hand selection, and controller loss.

## Acceptance criteria

- XR and untracked desktop grabs use the same hand-contact semantics.
- Small and large trees with offset origins, nested geometry, rotation, and
  nonuniform scale settle with the selected bounds contact at the hand anchor.
- Zero and nonzero clearance behave predictably; distant grabs approach smoothly.
- Desktop hand posing and object placement agree without a feedback loop where
  the hand chases the object while the object chases the hand.
- Missing bounds and skinned approximations follow the documented fallback.
- Ordinary release preserves world pose, and zone release can claim the handoff.

## Related work

- [Component-tree bounds measurement](../draft/component-tree-bounds-measurement-v1.md)
- [Grab animation and pose transitions](grab-animation-and-pose-transitions.md)

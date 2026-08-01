# XR controller/hand pose basis and laser alignment

Date: 2026-08-01

Status: active pre-0.8 investigation

## Problem

In Mittens XR examples, the hand laser now agrees with the rendered hand and
finger orientation, but that whole hand-and-laser result can point roughly 90
degrees outward from the forward direction shown by SteamVR and overlays such
as `wlx-overlay-s`/WayVR for the same physically held controller.

This makes the laser a useful witness, not the likely source of the error. The
current hypothesis is that Mittens selects the wrong controller pose for the
hand/wrist, applies a wrong pose-basis conversion, or applies an incorrect
avatar wrist/rest-pose correction. The defect must not be hidden by adding a
90-degree correction to the laser.

The observation may not occur on every start or configuration. Reproduction
must record runtime, interaction profile, hand/controller source, and whether
the session was recentered.

## Expected pose responsibilities

OpenXR exposes distinct controller concepts that should remain distinct:

- the **grip pose** normally places and orients the held controller or hand;
- the **aim pose** normally defines the pointing ray supplied by the runtime;
- articulated hand tracking supplies wrist/palm/joint poses that may need a
  model-specific rest-basis conversion; and
- an avatar-finger pointer may deliberately follow a finger after the hand is
  oriented correctly.

These are investigation assumptions, not proof of the current cause. Mittens
already has separate `Aim` and `Grip` controller pose kinds and OpenXR action
spaces; the task is to verify the complete source-to-avatar routing and every
coordinate conversion.

## Reproduction record

For every observed mismatch, record:

- OpenXR runtime and headset;
- active interaction profile and controller model;
- left or right hand;
- controller tracking versus articulated hand tracking;
- whether SteamVR, `wlx-overlay-s`, or WayVR supplies the comparison ray;
- cold start, warm restart, and recenter state;
- the physical controller pose used for comparison; and
- approximate angle and axis of the mismatch.

Check both hands. A mirrored sign error, a common 90-degree yaw, and a
hand-specific model-rest error point to different causes.

## Instrumentation

- [ ] Log raw located poses and validity/tracking flags for left/right aim and
      grip spaces.
- [ ] Log the active interaction profile, selected fallback/source, reference
      space, and action activity.
- [ ] Log raw OpenXR quaternions and positions beside the post-conversion
      engine transforms.
- [ ] Log wrist/palm or chosen hand-joint poses when hand tracking is active.
- [ ] Log avatar-control/IK wrist corrections and the final hand, finger
      mount, pointer, and laser world transforms.
- [ ] Render temporary colored axes/rays for raw aim, raw grip, raw
      wrist/palm, final wrist/hand, finger direction, and final pointer ray.

The debug view should make a wrong source distinguishable from a wrong basis:
if raw aim and grip agree with the runtime overlay but the converted axes do
not, conversion is suspect; if only one source agrees, routing is suspect.

## Isolation matrix

- [ ] Drive a debug controller mesh directly from grip pose with avatar and IK
      corrections disabled.
- [ ] Draw the runtime aim ray directly from aim pose with no hand/finger
      parenting.
- [ ] Compare grip-driven hand root versus aim-driven hand root.
- [ ] Compare controller tracking with articulated wrist/palm tracking.
- [ ] Enable avatar rest-pose and wrist corrections one stage at a time.
- [ ] Compare local, stage, and any application/avatar reference transforms.
- [ ] Repeat for both hands and supported interaction profiles.
- [ ] Compare at least SteamVR's own ray and `wlx-overlay-s`/WayVR behavior in
      the same session where practical.

## Likely implementation boundary

If the evidence confirms conventional routing, the controller/hand model
should take placement from grip (or the appropriate articulated wrist/palm
pose), while a runtime-style pointer takes direction from aim. A finger-driven
pointer may continue to follow the finger, but only after the hand/wrist basis
is correct.

Any model-specific rest-basis correction belongs at the model/avatar retarget
boundary and must document source and destination axes. It must not be baked
into the generic pointer or laser presentation.

## Regression coverage

- [ ] Unit-test aim and grip source routing independently.
- [ ] Test OpenXR-to-engine basis conversion with fixed poses and expected
      forward/up axes.
- [ ] Test left/right handedness and mirrored transforms.
- [ ] Test that an avatar/model rest correction is applied exactly once.
- [ ] Keep the laser aligned with its actual pointer source without adding a
      compensating world-space yaw.
- [ ] Capture VR verification evidence for each supported source/profile in
      the reproduction matrix.

## Acceptance criteria

- [ ] With a physically forward-held controller, the Mittens hand/controller
      orientation agrees with the runtime's grip presentation.
- [ ] A runtime-aim pointer agrees with the SteamVR/overlay aim ray within an
      explicitly recorded tolerance.
- [ ] Articulated hands have a natural wrist/palm orientation and a
      finger-driven ray follows the intended finger.
- [ ] Neither hand has the observed right-angle outward yaw across repeated
      starts and recentering.
- [ ] The fix is made at pose selection, basis conversion, or avatar
      retargeting—not as a laser-only correction.
- [ ] Existing laser origin and non-selectability fixes remain intact.

## Related documents

- [XR hand laser selection and origin](../bugs/xr-hand-laser-is-selectable-and-origin-is-past-fingertip.md)
- [XR avatar pose grounding](../analysis/xr-avatar-pose-grounding.md)
- [OpenXR runtime investigation matrix](../analysis/openxr-runtime-investigation-matrix.md)
- [OpenXR controller actions and default locomotion](openxr-controller-actions-and-default-stick-locomotion.md)
- [OpenXR interaction-profile default bindings](openxr-interaction-profile-default-bindings.md)
- [Controller XR armature targeting](refactor/controller-xr-armature-targeting.md)
- [OpenXR input refactor](refactor/openxr-input.md)
- [Mittens MMS ownership cutover and 0.8 release](mittens-mms-ownership-cutover-and-0.8-release.md)


# Imported humanoid pose-basis detection and conversion

Date: 2026-08-04

Status: implemented for controller-driven avatar hands; broader humanoid retargeting review

## Purpose

Imported humanoid rigs do not share a dependable bone-local axis convention.
One exporter may place finger length on local `+Y`, another on `+X`, and either
may roll the hand bone differently without changing the visible rest pose.
Runtime tracking APIs, by contrast, define semantic pose frames such as OpenXR
Aim and Grip. Connecting the two requires deriving an anatomical frame from the
imported geometry rather than assuming that a named bone axis has universal
meaning.

This review records the mechanism used by avatar controller hands and the rules
that should guide similar retargeting work elsewhere in the engine.

## Why one direction is insufficient

A normalized direction constrains two rotational degrees of freedom. Rotation
around that direction remains free. Aligning a middle finger with controller
Aim can therefore produce a zero forward error while leaving the palm rolled by
90 degrees.

The previous forward-only path used the minimum-arc quaternion from canonical
`-Z` to the final middle-finger segment. That is deterministic, but "minimum
arc" is a mathematical choice rather than an anatomical roll convention. On
Bisket it preserved the hand-width axis along Aim `X`, making the pinky-to-thumb
span horizontal even though the laser pointed exactly along Aim.

A complete orientation needs two non-collinear semantic directions.

## Hand frame construction

The controller-hand path uses immutable GLTF rest-pose positions expressed
relative to the configured hand bone. Its preferred full-palm construction is:

- `forward`: normalized whole middle-finger direction from `Middle1` to `Middle3`;
- `knuckle_width`: position of `Index1` minus position of `Little1`;
- `up`: `knuckle_width` projected onto the plane perpendicular to `forward`,
  then normalized;
- `back`: `-forward`;
- `right`: normalized `up × back`.

`up` is recomputed as `back × right` after projection so the result is an
orthonormal, right-handed frame. Its columns are `[right, up, back]`, meaning
that the resulting quaternion maps canonical controller axes as follows:

- canonical `-Z` to avatar finger-forward;
- canonical `+Y` to avatar little-to-index knuckle width;
- canonical `+X` to the remaining orthogonal hand axis.

AVC applies the inverse quaternion at the runtime visual hand target. With an
OpenXR Aim orientation on that target, the avatar's finger-forward reproduces
Aim `-Z` and its little-to-index direction reproduces Aim `+Y`. Left and right hands
use the same semantic rule; no mirrored authored quaternion is required.

The four-landmark middle-finger/thumb API remains as a fallback for models that
do not configure the preferred index/little knuckle pair. The thumb-root vector
can be a poor palm-width proxy when the thumb root sits substantially wristward
of the middle root, so it should not be preferred when proximal knuckle
landmarks are available.

The fingertip laser mount uses the derived transform rather than a second copy
of the conversion, so the visible hand and ray share one derived basis.

## Degenerate and fallback behavior

The basis is rejected when:

- a selector does not resolve to exactly one spawned GLTF node;
- the middle joints are not an ancestral chain under the hand;
- the final finger segment has zero length;
- a configured palm landmark is not beneath the hand; or
- projected palm width is collinear with finger-forward.

The three-landmark `.laser_from_avatar_finger(...)` API remains a compatible
forward-only fallback. It deliberately cannot promise palm roll. The four-
landmark `.laser_from_avatar_hand(...)` API should be used when a full hand pose
is required.

Runtime controller action activity and avatar basis validity are separate
questions. Active Aim/Grip actions select the controller-driven pose; missing
controller actions fall back to wrist/palm tracking. A basis failure affects
avatar retargeting and laser mounting, not OpenXR pose validity.

## General retargeting guidance

The same pattern applies to other imported humanoid segments:

1. Identify semantic landmarks or bone-to-bone directions in rest/model space.
2. Use at least two non-collinear directions when full orientation matters.
3. Project and orthonormalize instead of trusting imperfect authored geometry.
4. Construct one explicit source basis and one explicit target basis.
5. Apply `target_basis × inverse(source_basis)` at a single documented layer.
6. Keep position, forward alignment, and roll alignment separately observable.
7. Reject degenerate input rather than hiding it with an arbitrary axis or an
   avatar-specific Euler/quaternion adjustment.

Bone names identify landmarks; they do not define axes. This distinction is
important for hands, feet, eyes, head look direction, weapon sockets, and any
future articulated-hand retargeting.

## Diagnostics and validation

`CAT_DEBUG_XR_HAND_ALIGNMENT=1` reports whether the avatar basis is
`forward-only`, `forward+thumb-up`, or `whole-forward+knuckle-up`, the raw Aim and Grip rotations, the
Grip-to-Aim delta, the derived canonical-to-hand quaternion, the applied inverse
correction, and the predicted final basis-to-Aim error.

Useful validation poses are:

- controller Aim held forward: finger direction is forward and little-to-index is up;
- controller rolled around Aim: the avatar hand follows that roll continuously;
- controller pitched through the Grip/Aim offset: no discontinuity or source
  switch occurs;
- left and right hands together: both use little-to-index `+Y` without authored
  mirrored corrections;
- controller actions becoming inactive: wrist/palm fallback applies identity
  until articulated-hand basis retargeting is implemented.

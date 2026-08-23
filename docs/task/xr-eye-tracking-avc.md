# AVC automatic eye-bone tracking — Phase 1

## Contract

`XREyeTracking` and `XREyeTrackingHTC` become AVC pose drivers when they are
direct children of an `AvatarControl`. AVC reads the newest valid normalized
head-local gaze sample (`-Z` is forward) and drives the mapped `left_eye` and
`right_eye` slots independently. Existing scoped eye-tracking events remain
unchanged.

The written pose is absolute: AVC creates the shortest-arc correction from
canonical forward and composes it with the bone's immutable GLTF rest rotation.
It does not accumulate a correction from the prior frame. AVC owns an eye only
while it has both a valid gaze and a mapped target; on loss of gaze, tracker
removal, or a map target replacement/removal it restores the previously owned
bone to its rest rotation.

## Completed implementation

- Both tracker components retain their latest normalized left/right gaze sample
  and a receive sequence from a counter shared across protocol types.
- Standard OSC uses a per-eye vector when present, otherwise its combined gaze;
  HTC uses its per-eye look values.
- AVC considers only direct tracker children and resolves the newest valid
  sample per eye across both sources.
- AVC consumes the retained humanoid-map report and can drive one mapped eye
  even when the other eye is absent or invalid.
- Rest-relative correction and ownership/restoration are implemented as
  runtime-only state; no MMS calibration surface was added.

## Phase-1 caveats and Phase 2

Gaze directions are assumed to be finite, nonzero, head-local vectors in the
canonical `-Z` eye basis. Openness, pupil position, squeeze, and other face
channels remain event-only. Noncanonical eye axes need calibration and are not
silently guessed in Phase 1.

Phase 2 will add an explicit per-rig eye calibration API (basis/offset or
equivalent) for those noncanonical rigs.

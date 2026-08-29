# Eye tracking appears to counteract head rotation

Status: open / needs headset reproduction and coordinate-space diagnosis

## Observed behavior

While wearing a VTuber avatar with automatic eye tracking enabled, *actively rotating* the HMD/head
can cause the avatar's eyes to rotate strongly in the apparent opposite direction. The effect is
transient: once head rotation stops, the interference stops and eye tracking returns to its normal
quality at the held head pose. Head motion alone can therefore produce a large eye response even
when the wearer intends to keep looking straight ahead.

This has been observed with the automatic direct-child `XREyeTracking` → `AVC` path used by
`examples/vtuber-eye-tracking-mirror.mms`.

## Current implementation evidence

`AvatarControlSystem::update_one_eye_tracking` currently:

1. accepts the transport's normalized gaze vector directly;
2. constructs a shortest-arc quaternion from fixed forward `[0, 0, -1]` to that vector; and
3. writes `correction * eye_rest_rotation` as the target eye bone's local rotation.

The calculation does not currently transform the gaze vector through the current head/eye-parent
basis, nor does it account for head angular velocity, transport timestamps, or gaze/HMD sample
alignment. A static basis mismatch remains possible, but the fact that the problem disappears at a
held rotated pose makes a motion-time mismatch more likely: the tracker may emit gaze in a frame
whose HMD orientation is from an earlier/later instant, or the engine may combine current head pose
with delayed/filtered gaze.

The source transport's coordinate space is not yet verified. The symptom could originate in:

- headset/ALVR eye-tracking output whose gaze and HMD pose are sampled or filtered at different
  times during rotation;
- axis/sign/basis conversion at OSC decoding;
- AVC's gaze-to-local-eye-bone conversion or update ordering; or
- a combination of those factors.

## Desired behavior

With gaze held neutral relative to the wearer’s head, rotating the head should not create an
additional opposite eye rotation. The eyes should retain their intended head-relative gaze both
during rotation and after settling, subject only to the avatar rig's normal hierarchy motion.

## Proposed calibration surface

Add an explicit eye-tracking head-rotation compensation setting rather than silently assuming one
tracker convention. It should be a builder setting on both tracker components, for example:

```text
XREyeTracking.on().head_rotation_compensation("off")
XREyeTrackingHTC.on().head_rotation_compensation("cancel")

Off                    use the raw tracker vector exactly as today
CancelHeadRotation     transform/counter-rotate gaze by the current head rotation before
                        converting it into the target eye bone's local basis
```

The exact builder and enum spellings remain open, but the ownership does not: the setting is per
tracker instance because it describes that source transport's coordinate/timing convention. AVC
remains the consumer that applies the resolved policy to its direct-child tracker sample. This is a
candidate mitigation, not the assumed root cause: a timestamp/latency problem needs sample
alignment rather than merely a static inverse head rotation.

## Investigation and acceptance checks

1. Add diagnostics that record raw left/right tracker vectors, HMD/head orientation and angular
   velocity, arrival/frame timestamps, resolved eye-bone parent/world orientation, and final
   written local eye rotations.
2. Reproduce with an avatar whose eye rest pose and axes are known, recording these four cases:
   stationary neutral head/neutral gaze; held rotated head/neutral gaze; actively rotating
   head/neutral gaze; and actively rotating head/intentional gaze.
3. Compare gaze and HMD sample timing during rotation. Establish the OSC source convention from
   ALVR/headset documentation or controlled recordings: head-relative, HMD-local,
   stage/world-relative, handedness/forward axis, and timestamp/filter behavior.
4. If the source and HMD pose are temporally aligned, implement the selected compensation only in
   the gaze-to-eye-local conversion, preserving the
   immutable eye rest pose and existing absent-tracker restoration behavior.
5. If timing is not aligned, prefer timestamped interpolation/extrapolation or a documented
   synchronized sample policy before adding a static compensation multiplier.
6. Verify that `Off` preserves current output byte-for-byte for a fixed sample, while any selected
   mitigation removes the in-motion counter-rotation without inverting intentional
   left/right/up/down gaze.
7. Test both mapped eye bones and a rig without eye slots; the latter must remain unchanged.

## Related code and scene

- `src/engine/ecs/system/avatar_control_system.rs` — current gaze-to-eye-bone calculation.
- `src/engine/ecs/system/xr_eye_tracking_system.rs` — OSC/HTC gaze transport decoding.
- `examples/vtuber-eye-tracking-mirror.mms` — headset reproduction scene.

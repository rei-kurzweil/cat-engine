# Unified two-eye tracking normalization and AVC routing

Status: slice 1 and slice 2 implemented / headset validation pending

Parent: [Eye and face tracking epic](epic/eye-and-face-tracking.md)

## Outcome

`AvatarControlSystem` consumes one transport-independent, two-eye semantic state whether input
arrives through `XREyeTracking` (VRChat OSC) or `XREyeTrackingHTC` (ALVR binary UDP). Transport
systems own parsing and source-specific normalization; AVC only resolves freshness/arbitration and
applies left/right semantic channels to the corresponding avatar targets.

The first delivered slice generalizes gaze and blink closure. Pupil position, diameter, wide, and
squeeze use the same representation afterward without creating another source-specific AVC path.

## Why HTC sends openness

HTC's `XR_HTC_eye_tracker` geometric API names the native field `eye_openness`. ALVR currently
preserves that native meaning in `EyeDataHtc` and in the Mittens v1 UDP packet:

```text
0 = closed
1 = open
```

VRChat Eye OSC sends `EyesClosedAmount`, whose semantic direction is the inverse:

```text
0 = open
1 = closed
```

### Settled transport-boundary decision

Keep every HTC-specific field in its native HTC meaning through the ALVR client, `EyeDataHtc`, and
the Mittens binary UDP packet. ALVR must not rename, invert, clamp, combine, or otherwise normalize
`eye_openness`, `eye_wide`, `eye_squeeze`, pupil position, or pupil diameter for AVC. Validity remains
explicit rather than being encoded as a replacement numeric value.

Mittens owns all conversion from the HTC-native representation into engine semantics. The useful
consistency boundary is the first normalized Mittens state, not the source wire format. Normalize
exactly once after decoding and before storing the state consumed by AVC:

```text
OSC closure = clamp(EyesClosedAmount, 0, 1)
HTC closure = clamp(1 - eye_openness, 0, 1)
```

Events may continue to expose source-native `openness` for diagnostics. AVC must not know which
formula produced normalized `closure`.

This preserves a lossless diagnostic view of the hardware data, avoids making ALVR aware of avatar
semantics, and lets future consumers reinterpret HTC fields without undoing an irreversible sender
conversion.

## What ALVR's VRCFaceTracking sink does

The `VrcFaceTracking` mode is another binary UDP output, sent to fixed port `0xA1F7` (41463):

- `EyesQuat` carries two gaze quaternions, or `CombQuat` carries one combined quaternion;
- `Face2Fb`, `FacePico`, or `EyesHtc`/`LipHtc` carries the vendor expression array;
- on Focus 3, the custom client synthesizes the HTC eye-expression array from geometric openness:
  index `0` is left blink and index `2` is right blink, each `clamp(1 - openness, 0, 1)`;
- it is intended for VRCFaceTracking software and does not carry the custom Mittens pupil
  position/diameter fields.

So VRCFaceTracking already receives HTC blinking as closedness-like expression coefficients. The
missing behavior is specific to Mittens' consumer state: `XREyeTrackingHTC` decodes openness but
currently retains only gaze for AVC.

## Prior divergence (resolved)

Before this task, `XREyeTrackingComponent` owned:

- retained left/right gaze;
- one combined closure sample from OSC.

while `XREyeTrackingHtcComponent` owned:

- retained left/right gaze;
- no retained openness/closure; HTC geometric fields are event-only.

AVC also had a source-specific function named `update_generic_osc_blink`. The first implementation
slice removed that divergence: both components now retain the same normalized left/right gaze and
closure samples, and source-specific interpretation remains in `XREyeTrackingSystem`.

## Normalized representation

Use explicit left/right storage for every semantic channel. Do not encode “combined” as a third eye
inside AVC.

Conceptually:

```rust
struct EyePair<T> {
    left: Option<T>,
    right: Option<T>,
}

struct EyeChannelSample<T> {
    eyes: EyePair<T>,
    sequence: u64,
    // Runtime diagnostics may also retain source, arrival time, validity,
    // and whether a side was exact, combined, or mirrored fallback.
}

struct NormalizedEyeState {
    gaze: EyeChannelSample<[f32; 3]>,
    closure: EyeChannelSample<f32>,
    pupil_position: EyeChannelSample<[f32; 2]>,
    pupil_diameter: EyeChannelSample<f32>,
    wide: EyeChannelSample<f32>,
    squeeze: EyeChannelSample<f32>,
}
```

The concrete Rust types may be smaller, but the semantics must remain explicit. Gaze and closure
need independent sequences because OSC sends them in separate datagrams. Later channels must not
borrow gaze freshness merely because they arrived in the same HTC packet.

### Canonical channel semantics

- `gaze`: finite, nonzero, normalized head-local direction; canonical forward is `[0, 0, -1]`.
- `closure`: finite scalar clamped to `[0, 1]`; `0` means fully open and `1` fully closed.
- `pupil_position`: per-eye source position with a declared coordinate convention; do not silently
  treat it as gaze direction.
- `pupil_diameter`: non-negative per-eye source value with documented units once HTC behavior is
  verified.
- `wide` and `squeeze`: per-eye semantic coefficients; clamp only after confirming the HTC range.
- An absent or invalid channel is `None`, never a numeric sentinel.

## Expanding input into two eyes

Normalize cardinality before storing a channel:

1. If explicit left and right values are valid, preserve them independently.
2. If one combined value is supplied, copy it to both left and right.
3. If a source supplies one valid explicit eye and the other is absent, copy the valid value to the
   missing side as a `mirrored fallback`, so AVC still receives a complete pair.
4. If no valid value exists, both sides are absent and AVC releases ownership for that live channel.

Explicit per-eye data always beats combined or mirrored fallback. Never average two explicit eyes
before AVC. Preserve fallback provenance for diagnostics because copying a unilateral blink can turn
a wink into a bilateral blink; it is a predictable availability policy, not invented sensor data.
If headset testing shows unilateral HTC invalidity is common and mirroring is misleading, make the
fallback policy configurable at the tracker boundary without changing AVC.

Examples:

| Source update | Normalized left | Normalized right |
|---|---:|---:|
| OSC combined closure `0.7` | `0.7` | `0.7` |
| OSC combined gaze `g` | `g` | `g` |
| OSC left `l`, right `r` | `l` | `r` |
| HTC openness `[0.2, 0.9]` | closure `0.8` | closure `0.1` |
| HTC left gaze `l`, right invalid | `l` | `l` (mirrored fallback) |
| no valid closure | absent | absent |

## Transport normalization

### `XREyeTracking` / VRChat OSC

- Decode combined or per-eye gaze into `EyePair<Look>` using the expansion rules.
- Treat `EyesClosedAmount` as closure directly and copy it to both eyes.
- Clamp closure and reject non-finite values before storing.
- Preserve the existing OSC update event fields for script compatibility.

### `XREyeTrackingHTC` / Mittens binary UDP

- Convert each valid gaze quaternion to a normalized head-local forward direction.
- Convert valid per-eye openness to closure with `1 - openness`, then clamp.
- Expand unilateral valid values according to the shared fallback rule.
- Retain normalized closure on `XREyeTrackingHtcComponent`; keep native openness in
  `XrEyeTrackingHtcUpdated` for diagnostics and scripts.
- Later, normalize and retain pupil position, diameter, wide, and squeeze through the same two-eye
  structure.

Do not change the Mittens UDP field from openness to closure merely to simplify AVC. Future protocol
versions must continue to carry HTC fields with their native meanings; version changes may add
timestamps, provenance, or fields, but must not silently redefine existing HTC values.

## AVC consumption and arbitration

Replace `update_generic_osc_blink` with source-neutral eye-channel routing:

1. Inspect direct-child `XREyeTracking` and `XREyeTrackingHTC` components.
2. Resolve the newest valid value independently for each semantic channel and eye using the shared
   receive sequence.
3. Apply left gaze only to the mapped left-eye bone and right gaze only to the right-eye bone.
4. Apply left closure only to `left_eye_blink` and right closure only to `right_eye_blink`.
5. Set the morph driver to `None` when that eye has no fresh live closure, restoring the imported or
   editor-authored base value.
6. Do not let a newer gaze sample make an older closure sample fresh, or vice versa.
7. Keep gaze rotation limits/head-motion policies independent from blink and pupil routing.

When both tracker types coexist, newest valid value wins per eye and per channel. A source that has
new gaze but no closure must not erase another source's newer valid closure unless its closure has
explicitly become invalid/stale under the selected freshness policy.

## Freshness

Gaze remains retained until replaced or the tracker/component is removed, matching current AVC
behavior. Closure is a live override and must not leave an avatar permanently blinking.

First preserve current behavior: a closure is live only for the receive tick in which its field was
observed. Clear both normalized closure sides before polling that component, then repopulate valid
fields. If headset testing shows visible one-frame dropouts from UDP loss, add a short explicit
timeout with tests; do not silently make closure indefinitely retained.

Removal, failed bind, invalid field flags, and stale timeout must all release the corresponding morph
driver. One eye may release while the other remains driven when exact per-eye data is available.

## Implementation slices

### Slice 1 — shared two-eye closure (implemented)

- Generalize `EyeClosureSample` from one `Option<f32>` to left/right values.
- Add normalized closure state to `XREyeTrackingHtcComponent`.
- Make OSC combined closure populate both sides.
- Make HTC openness populate per-eye closure.
- Rename and generalize AVC blink routing across both component types.
- Add focused transport, arbitration, morph application, and stale-release tests.

Code and fixture coverage are complete. Live headset validation remains: switching ALVR between
VRChat OSC and Mittens UDP should produce the same bilateral blink for the same physical blink,
while HTC also preserves distinct left/right values.

### Slice 2 — shared normalized gaze ownership (implemented)

- Factor OSC combined/per-eye and HTC per-eye fallback into one tested expansion helper.
- Store the same gaze sample type on both components.
- Preserve existing gaze limits, freeze policy, rest-relative application, and source arbitration.
- Add unilateral and combined fixtures to prevent source-specific AVC branches from returning.

### Slice 3 — remaining per-eye channels

- Retain pupil position, diameter, wide, and squeeze with independent validity/freshness.
- Define explicit morph/rig mapping channels before AVC applies them.
- Keep raw pupil position separate from derived gaze direction.
- Add diagnostics showing exact versus combined/mirrored provenance and active source.

## Test matrix

- OSC closure: open `0`, partial, closed `1`, out-of-range clamp, NaN rejection, missing packet.
- HTC openness: open `1`, partial, closed `0`, out-of-range clamp, NaN rejection, invalid flags.
- Per-eye HTC: distinct left/right values, left-only, right-only, and neither valid.
- Gaze: combined, two explicit eyes, unilateral fallback, zero quaternion/vector, non-finite input.
- Arbitration: OSC and HTC coexist with alternating newest gaze and closure sequences.
- Morph routing: left and right target labels differ; one missing mapping does not affect the other.
- Lifecycle: packet loss, socket rebind, component removal, source switch, and morph base restoration.
- Compatibility: existing OSC scenes and source-native update events retain their behavior.

## Acceptance

- AVC has no OSC-specific or HTC-specific blink application function.
- Both tracker components expose the same normalized two-eye gaze and closure contract internally.
- Combined input drives both eyes; explicit two-eye input remains independent; unilateral input uses
  the documented fallback.
- HTC physical blinking visibly drives the mapped left/right blink morphs.
- Missing/stale closure restores base morph values rather than sticking or forcing zero.
- Gaze limits and head-motion experiments do not alter closure or other semantic channels.
- The v1 HTC packet and script update events remain compatible unless an explicitly versioned
  migration is approved.
- ALVR preserves HTC-native fields and validity without AVC-oriented inversion or normalization;
  all conversion to closure and other engine semantics occurs in Mittens.

## Related files

- `src/engine/ecs/component/xr_eye_tracking.rs`
- `src/engine/ecs/system/xr_eye_tracking_system.rs`
- `src/engine/ecs/system/avatar_control_system.rs`
- `src/engine/ecs/component/morph_target.rs`
- `examples/vtuber-eye-tracking-mirror.mms`
- `../ALVR/ALVR/alvr/client_openxr/src/interaction.rs`
- `../ALVR/ALVR/alvr/server_core/src/tracking/face.rs`

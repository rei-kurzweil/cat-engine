# AVC mouth-open fallback from Amplitude

Date: 2026-09-01
Status: implemented; awaiting live HMD and multi-microphone acceptance

## Goal

Provide an explicit AVC-owned fallback that drives one configured mouth-open
morph from a retained `AmplitudeComponent` measurement.

The feature is intentionally authored on AVC, not on `Amplitude`:

```mms
let microphone = AudioInput {}
let amplitude = Amplitude.rolling_window(0.250).from(microphone) {}

AVC {
    mouth_open_from_amplitude(amplitude)
}
```

`Amplitude` remains a generic source observer. It does not know that AVC,
morph targets, mouth animation, or visemes exist.

## Preconditions

- [x] `docs/task/audio-amplitude-avc-first-slice.md` provides current,
  generation-safe retained RMS snapshots from a live source.
- [ ] AVC can resolve enabled direct-child amplitude observations for
  diagnostics and later policy consumers.
- [ ] The avatar has a `MorphTargetMap` mapping the selected logical mouth slot
  to a GLTF morph target.

## First-slice API

- [x] Add `AvatarControl.mouth_open_from_amplitude(amplitude_ref)`.
- [x] Accept a live component handle, UUID reference, or selector using the
  existing durable `ComponentRef` rules.
- [x] Resolve and cache the source like other AVC-owned references; changing,
  disabling, removing, or failing it clears the mouth contribution.
- [x] Serialize the authored reference; never serialize filtered weights or
  other runtime state.

The initial target is the logical `viseme_aa` slot. If that slot is absent or
unresolved, log one AVC diagnostic and make no morph write.

Calibration is explicit and serialized so different microphones can be tuned
without putting device-specific gain in the morph system:

```mms
AVC {
    mouth_open_from_amplitude(amplitude)
    mouth_open_rms_floor(0.015)
    mouth_open_rms_ceiling(0.12)
    mouth_open_smoothing(18.0) // exponential response rate, 1/seconds
}
```

## Runtime policy

```text
Amplitude retained RMS ──► AVC mouth-open binding ──► viseme_aa morph weight
```

- [x] Consume only a valid, finite, current-generation retained RMS sample.
- [x] Map RMS to `[0, 1]` using explicitly named floor, ceiling, and smoothing
  constants/configuration; do not hard-code unexplained gain in the morph
  system.
- [x] Neutral, stale, disabled, missing, or failed amplitude state eases the
  contribution to zero and never holds a stale open mouth.
- [x] AVC writes only its own named amplitude contribution, so later animation
  / viseme policies can arbitrate without replacing generic morph ownership.
- [x] Never access a worker, callback, or audio queue from AVC.

## Visemes coexistence decision

Selected: **Visemes wins.** The primary morph driver has priority over AVC's
named `amplitude_mouth_open` fallback. Removing the primary driver reveals the
current fallback again without replacing the authored/imported base value.

Alternatives considered:

1. **Visemes wins:** while an enabled direct `Visemes` child produces valid
   output, AVC suppresses the amplitude mouth-open contribution.
2. **Mutually exclusive:** AVC reports a configuration error when both are
   enabled.
3. **Blend:** AVC combines amplitude and viseme contributions through an
   explicitly defined blend rule.

The amplitude path is activated only by the explicit AVC builder; it is not
inferred from the absence or presence of a future `Visemes` child.

## Required automated coverage

- [x] MMS construction and round-trip of `mouth_open_from_amplitude`;
- [x] reference resolution, source replacement/removal, and stale-generation
  clearing;
- [x] valid RMS maps predictably to `viseme_aa` contribution;
- [x] neutral/invalid samples clear the contribution;
- [x] missing morph slot is harmless and diagnosed once;
- [x] selected Visemes coexistence policy.

## Live acceptance

In the mirror scene, speak into the microphone and verify a stable open-mouth
response from the configured `viseme_aa` target. Verify silence, source
disable/re-enable, and device loss settle the mouth closed without a held value.

## Explicitly deferred

- phoneme classification and multi-vowel viseme driving;
- generic `Amplitude.value()` reads in MMS;
- multiple named amplitude bindings or component-expression-valued AVC slots;
- automatic fallback behavior when no binding is authored;
- loudness normalization and calibration UI.

## Exit

One explicitly authored AVC binding can safely drive one mouth-open morph from
a live amplitude observation, with a documented Visemes coexistence policy.

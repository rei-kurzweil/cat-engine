# Viseme detection backend evaluation

Date: 2026-09-01
Status: ready for investigation; selection required before live viseme slice completion

## Decision to make

Select one streaming implementation that turns capture PCM into the engine's
canonical viseme vector for the first microphone-viseme slice.

This tracker deliberately does not choose the capture protocol, `AudioInput`,
or AVC routing. Those have a single engine-owned contract regardless of the
backend:

```text
PCM block → preallocated analysis unit → timestamped canonical VisemeFrame
```

No backend phoneme name or model-specific class crosses this boundary.

## First-slice target

The backend must produce a useful five-vowel reduction:

| Canonical channel | Avatar target example | Intended use |
| --- | --- | --- |
| `viseme_aa` | `Fcl_MTH_A` | open /a/-like mouth |
| `viseme_ih` | `Fcl_MTH_I` | /i/-like mouth |
| `viseme_ou` | `Fcl_MTH_U` | rounded /u/-like mouth |
| `viseme_e` | `Fcl_MTH_E` | /e/-like mouth |
| `viseme_oh` | `Fcl_MTH_O` | rounded /o/-like mouth |

Silence is represented by releasing temporary morph drivers, not requiring a
`viseme_sil` target. Other canonical channels remain valid map keys but are
out of scoring scope until a backend supports them.

## Candidate tracks

### A. Energy/formant heuristic

Use a fixed-rate feature extractor: RMS/noise gate, high-pass filter, short
window spectrum or filter-bank energies, estimated first/second formants, and
speaker-independent rules to distribute five vowel weights.

- Advantages: no model download, predictable memory/CPU, Fundsp can supply
  filters and metering primitives, easy deterministic tests.
- Risks: poor speaker/language robustness; cannot distinguish many consonant
  poses; should never be presented as speech recognition.
- Required spike: demonstrate stable, distinct five-vowel output for recorded
  single-speaker fixtures and graceful neutralization for noise/silence.

### B. Streaming phoneme/speech backend plus reduction

Run a local streaming recognizer that exposes timestamped phonemes or suitable
acoustic posteriors, then reduce its private labels to canonical visemes.

- Advantages: better coarticulation and consonant opportunity; model has
  already learned speaker variation.
- Risks: model size, startup/lifecycle complexity, platform support, licensing,
  latency, CPU and shutdown behavior.
- Required spike: verify incremental output without holding the ECS/world and
  record the exact model, license, and deploy size.

### C. Direct viseme classifier

Use a local streaming model whose output is already a mouth-pose distribution,
then adapt and calibrate its classes to the canonical registry.

- Advantages: no phoneme-to-viseme loss if the labels map well.
- Risks: candidate availability, label semantics, license, and little control
  over language/rig conventions.
- Required spike: prove its labels can map one-to-one or by documented merge to
  the five-vowel target without exposing its private vocabulary to MMS.

## Shared benchmark protocol

Build a small recorded PCM corpus committed or reproducibly generated under a
test asset license that permits repository use:

1. clean sustained `a`, `i`, `u`, `e`, `o` utterances;
2. ordinary short spoken phrases from at least two speakers;
3. silence/noise-floor recordings;
4. keyboard/fan/background noise;
5. a discontinuity and a deliberate queue-gap case.

For every candidate, feed identical fixed-size PCM blocks at the recognizer
rate and record:

- time to first non-neutral frame;
- stable update rate;
- time from confirmed silence to neutral;
- per-channel output during labelled vowel fixtures;
- false activation during silence/noise;
- CPU, allocated memory/model size, and shutdown time;
- behavior after a discontinuity and a generation reset.

The first visual pass must additionally be observed in
`examples/vtuber-eye-tracking-mirror-eye-stabilize.mms` using Bisket's five
mapped mouth targets.

## Decision gates

| Gate | Requirement |
| --- | --- |
| Streaming | No synchronous main-thread inference; worker owns all model state. |
| RT safety | Capture callback only copies/normalizes into bounded queues. |
| Latency | First visible mouth response ≤100 ms on target hardware. |
| Cadence | Stable canonical output ≥30 Hz while voiced. |
| Release | Mouth begins releasing ≤150 ms after confirmed silence. |
| Robustness | Silence/noise does not leave a held mouth pose; discontinuity resets temporal state. |
| Deployment | Supported desktop targets, acceptable license, model size, and memory. |
| Semantics | Adapter owns reduction; canonical registry remains backend-independent. |

## Decision record

Do not mark this complete until this table is filled with measured values.

| Candidate | Fixture quality | First response | CPU / memory | Model size / license | Platform result | Decision |
| --- | --- | --- | --- | --- | --- | --- |
| A. Energy/formant heuristic | pending | pending | pending | none | pending | pending |
| B. Streaming phoneme backend | pending | pending | pending | pending | pending | pending |
| C. Direct viseme classifier | pending | pending | pending | pending | pending | pending |

## Non-goals

- speech-to-text, transcript display, or voice commands;
- cloud-dependent recognition;
- auto-mapping arbitrary avatar rigs;
- exposing raw backend labels to MMS;
- general audio-node metering, tracked separately in
  [audio-node-metering.md](audio-node-metering.md).

## Exit

Choose one candidate, record why it passed the gates, retain the deterministic
fake backend for lifecycle tests, and link the selected implementation from
`audio-input-and-visemes-first-slice.md`.

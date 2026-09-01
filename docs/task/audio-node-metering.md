# Audio-node metering

Date: 2026-09-01
Status: queued after [audio amplitude observation](audio-amplitude-observation.md)

## Goal

Provide one general, retained measurement surface for a compiled audio-graph
node. The source-level RMS path is now tracked separately as
[audio amplitude observation](audio-amplitude-observation.md); this task
extends that model to graph nodes and post-effect/output measurements.

## First proposed retained metrics

- `peak`: maximum absolute sample in the most recent bounded window;
- `rms`: root-mean-square amplitude in that window;
- `timestamp` and `sequence`: identify freshness and discontinuities;
- `sample_count`: makes the aggregation window observable.

These are linear, unitless sample amplitudes. They are not LUFS and are not a
replacement for offline clip loudness normalization.

## Architecture constraints

- Metering attaches to a compiled graph-node output, after that node's own
  processing and before its parent mix/effect chain.
- A meter's RT state is fixed-size and updates without allocation, locks,
  logging, waiting, or ECS access.
- The audio thread publishes bounded snapshots; the main thread retains the
  newest valid snapshot by component identity and graph generation.
- A missing, disconnected, disabled, or underrun source reports a fresh
  neutral/discontinuous state rather than preserving an old loud value.
- MMS live reads, visual bars, automation, and diagnostics consume the same
  main-thread-retained snapshot. Do not synchronously query the RT thread.

## Deferred API decision

Choose whether authoring uses an explicit `AudioMeter` graph child, a
diagnostic system-owned meter, or both. `Amplitude.from(source)` remains the
source-level API; it must not silently change to post-effect semantics.

## Acceptance

- A deterministic fake source proves peak/RMS at a source and after an effect.
- A queue/graph generation reset cannot leave a stale nonzero reading.
- Metering an input, oscillator, and clip uses one implementation and one
  retained result model.

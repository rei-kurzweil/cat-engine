# MMS Keyframe Block Example Migration Status

## Status

The migration is complete. `ActionComponent`, `ActionSystem`, the `Action.*`
MMS constructors, and `Animation.resolve_targets(...)` no longer exist.
Executable `Keyframe.at(...) { ... }` blocks are the sole deferred-animation
model and capture live MMS component handles.

The final Rust-authored examples now have live MMS scenes and thin launchers:

- `animation-example`
- `animation-for-topology`
- `audio-graph-example`
- `mesh-factory-example`
- `raycast-topology-animation`
- `text-animation`

Their keyframes use direct live methods for color, transforms, detach,
attach-clone, child removal, raycast requests, and band-pass center frequency.
Audio notes remain separate keyframe beats and run through the existing
audio-lookahead callback mode.

## Intent recipient model

Every intent dispatch carries one recipient. Scalar `component_id`, `parent`,
and `child` fields replace recipient vectors. `Signal.scope` remains separate
routing context, while semantic collections such as selection entries,
visualization roots, headers, and query results remain vectors.

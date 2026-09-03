# Audio input and visemes: first slice

Date: 2026-09-01
Status: ready for implementation planning

## Goal

Ship the smallest end-to-end path in which:

1. MMS creates an `AudioInput` using the default microphone.
2. `AudioInput` is a first-class `AudioSource` and compiles directly to
   `AudioGraphNodeKind::InputSource` when attached to an audio graph.
3. `Visemes.from(microphone)` provisions a preallocated analysis unit on a
   dedicated audio-input processing thread.
4. The worker returns timestamped canonical viseme weights to the main thread.
5. A `Visemes` component attached directly beneath `AVC` drives mapped glTF
   morph targets.
6. Silence, failure, disable, and removal release those temporary morph
   drivers.

Before viseme recognition is selected, implement the shared observable capture
path in [audio amplitude observation](audio-amplitude-observation.md). It is
both a microphone diagnostic gate and a useful fallback when recognition is
not available.

An initialized enabled `Amplitude` observer is an audio-input consumer even
when detached from `AudioOutput`; placing it directly beneath `AVC` makes its
retained main-thread measurement eligible for avatar-side use, not audible.

The authoritative API contract is
[audio input and visemes](../spec/audio-input-and-visemes.md). The broader
design and deferred work are tracked by the
[visemes epic](epic/visemes.md).

## Locked decisions

- `AudioInput` is an `AudioSource`, alongside `AudioOscillator` and
  `AudioClip`.
- The compiled graph variant is `AudioGraphNodeKind::InputSource` plus an
  equivalent RT node kind.
- There is no `AudioInputSource.new(microphone)` wrapper component.
- `AudioInput {}` selects the default input device.
- `AudioInput.device_number(index) {}` is a session-local device-selection
  constructor.
- Device enumeration is `AudioInput.devices() -> string[]`.
  on the `AudioInput` component name.
- `Visemes.from(audio_input)` creates a component holding an explicit reference
  to one live `AudioInput`.
- `Visemes` configuration remains authored in its component body:
  `language`, `attack_ms`, `release_ms`, `silence_release_ms`, and
  `min_confidence`.
- A direct `Visemes` child of `AVC` is eligible to drive that avatar.
- Speech analysis consumes raw capture PCM before audio-graph effects.
- Capture-to-render and capture-to-analysis use independent bounded queues.
- CPAL callbacks never run inference, access ECS, allocate, log, lock, wait, or
  perform a blocking send.
- Worker results are generation-tagged and retained on the main thread before
  AVC consumes them.
- Backend phoneme names never become public morph-map keys.
- `MorphTargetMap` remains the one semantic-channel-to-target-label component.
- A detached `AudioInput` referenced by `Visemes` is active but inaudible.
- Attaching the same input beneath `AudioOutput` makes monitoring explicit.

## Target MMS scene

The first slice is complete against this authoring shape:

```mms
let microphone = AudioInput {}

let visemes = Visemes.from(microphone) {
    language("en")
    attack_ms(35)
    release_ms(80)
    silence_release_ms(140)
    min_confidence(0.25)
}

AVC {
    visemes

    T {
        GLTF.new("avatar.glb") {
            MorphTargetMap.new()
                .slot("viseme_aa", "Fcl_MTH_A")
                .slot("viseme_ih", "Fcl_MTH_I")
                .slot("viseme_ou", "Fcl_MTH_U")
                .slot("viseme_e", "Fcl_MTH_E")
                .slot("viseme_oh", "Fcl_MTH_O")
        }
    }
}
```

Optional explicit monitoring uses the same component handle:

```mms
AudioOutput {
    microphone
}
```

## Runtime path

```text
AudioInputComponent
    │ resolve/start default CPAL device
    ▼
CPAL input callback
    ├── fixed PCM blocks -> render SPSC -> RT InputSource (when graph-attached)
    └── fixed PCM blocks -> analysis SPSC
                                      │
                                      ▼
                         audio-input processing thread
                         preallocated VisemeUnit
                                      │
                                      │ VisemeFrame
                                      ▼
                              main VisemeSystem
                         retained VisemeSample/status
                                      │
                                      ▼
                            AvatarControlSystem
                                      │
                                      ▼
                    MorphTargetMap -> GLTF morph drivers
```

## Ordered implementation checklist

### 1. Canonical viseme channel registry

- [ ] Locate and record any existing canonical mouth-channel names.
- [ ] If no complete list exists, choose the canonical first-slice set and
  ordering explicitly.
- [ ] Define one engine-owned registry used by `MorphTargetMap`, worker output,
  AVC routing, diagnostics, and tests.
- [ ] Extend `MorphTargetMapComponent::slot(...)` validation beyond the two
  existing blink channels.
- [ ] Preserve MMS serialization and round-trip behavior for the new slots.
- [ ] Define the first five-vowel reduction mapping for partial VRM-style rigs.

Gate: a mouth slot can be authored, validated, serialized, and restored without
using a backend-specific phoneme string.

### 2. `AudioInput` component and MMS catalog

- [ ] Add `AudioInputComponent` with device selector, enabled state, requested
  format hints, and observable status.
- [ ] Register `AudioInput` in the component registry and runtime catalog.
- [ ] Add bare/default, `default()`, and `device_number(index)` constructors.
- [x] Add the `AudioInput.devices() -> string[]` static host API.
- [ ] Validate constructor arguments and serialize the authored device choice.
- [ ] Add focused MMS materialization/round-trip tests.

Gate: MMS can enumerate devices and construct a detached live input component
using the default device or a valid session-local index.

### 3. Capture runtime and lifecycle

- [ ] Add `AudioInputSystem` as owner of CPAL input streams and capture runtime
  state keyed by component identity and generation.
- [ ] Start capture only while the input is enabled and has at least one live
  graph or analysis consumer.
- [ ] Negotiate the device-native format and expose the negotiated status.
- [ ] Implement disable, reconfigure, device-loss, final-consumer removal, and
  component-removal teardown.
- [ ] Ensure teardown never waits from inside a CPAL callback.
- [ ] Add a fake capture endpoint so lifecycle tests require no microphone.

Gate: repeated enable/disable/reconfigure/removal cannot leak a stream, retain
a stale consumer, or update a newer component generation.

### 4. Bounded capture protocol

- [ ] Define a fixed-capacity PCM block carrying source identity, generation,
  sequence, captured-frame timestamp, format, and valid-frame count.
- [ ] Preallocate callback staging and queue storage.
- [ ] Normalize device samples to `f32` only through bounded allocation-free
  callback code.
- [ ] Perform channel remix and recognizer-rate conversion off the callback.
- [ ] Give render and analysis consumers independent SPSC queues.
- [ ] Drop bounded work rather than block when a queue is full.
- [ ] Record overrun counters and observable sequence gaps.
- [ ] Reset consumer temporal state after generation changes or discontinuity.

Gate: callback-path tests prove no allocation/blocking and queue-full tests
prove deterministic recovery with a visible discontinuity.

### 5. Audio graph `InputSource`

- [ ] Add `AudioInput` to the authored `AudioSource` vocabulary.
- [ ] Add `AudioGraphNodeKind::InputSource`.
- [ ] Add the equivalent RT graph-node variant and state.
- [ ] Teach `AudioGraphCompiler` to compile an attached `AudioInput` directly.
- [ ] Render from the input's dedicated capture-to-render queue.
- [ ] Define underrun behavior as silence without blocking.
- [ ] Route the source through existing graph effects and output mixing.
- [ ] Do not respond to `AudioSchedulePlay`; input is a continuous source.

Gate: an attached fake input reaches output through the compiled graph, while
the same input detached from `AudioOutput` remains inaudible.

### 6. `Visemes` component and worker unit protocol

- [ ] Add `VisemesComponent` with source reference, enabled/configuration
  fields, retained sample, generation, and status.
- [ ] Register `Visemes.from(audio_input)` and its configuration builders in
  MMS.
- [ ] Reject a `.from(...)` receiver which is not an `AudioInput`.
- [ ] Add a named audio-input processing worker with explicit start/shutdown.
- [ ] Preallocate the supported number of `VisemeUnit` slots before enabling
  capture delivery.
- [ ] Add main-to-worker `RegisterUnit`, `UpdateUnit`, `ResetUnit`,
  `RemoveUnit`, and `Shutdown` messages.
- [ ] Permit one active `Visemes` unit per `AudioInput` in the first slice.
- [ ] Make registration failure observable instead of allocating or blocking
  on the callback.

Gate: constructing, attaching, disabling, and removing `Visemes` produces the
expected generation-safe worker lifecycle with a deterministic fake unit.

### 7. Recognition backend spike and adapter

Evaluation tracker: [viseme detection backend evaluation](viseme-detection-backend-evaluation.md).

- [ ] Define the streaming backend trait behind the preallocated unit.
- [ ] Add a deterministic fake backend for tests.
- [ ] Build a small recorded PCM fixture corpus covering voiced speech,
  silence, noise, and discontinuity.
- [ ] Evaluate candidate backends for streaming output, latency, CPU, memory,
  model size, platform support, licensing, and shutdown behavior.
- [ ] Select one backend only after recording the comparison.
- [ ] Convert backend output to the canonical ordered viseme vector.
- [ ] Apply configured confidence threshold, attack, release, coarticulation,
  and silence release on the worker.

Gate: the selected backend or deterministic fake turns fixture PCM into
timestamped canonical `VisemeFrame`s without ECS access.

### 8. Worker-to-main retained state

- [ ] Add bounded worker events for ready, frame, overrun, failure, and stopped
  states.
- [ ] Tag every event with `Visemes` identity and generation.
- [ ] Drain all available events once per main-thread tick.
- [ ] Reject stale, removed, disabled, or superseded generations.
- [ ] Retain only the newest valid `VisemeSample` and status for AVC.
- [ ] Clear to neutral on silence deadline, overrun reset, backend failure,
  input failure, disable, and removal.
- [ ] Never synchronously query the worker from the main thread.

Gate: retained state follows deterministic event fixtures and never freezes a
voiced mouth pose after a lifecycle or failure event.

### 9. Morph-driver ownership decision

- [ ] Audit the current single anonymous `MorphFactorState::driver` slot.
- [ ] Either add named driver ownership/composition or explicitly enforce one
  owner per target for this slice.
- [ ] Diagnose conflicting mappings once rather than overwriting silently.
- [ ] Preserve imported base factors when a viseme driver releases.
- [ ] Prove releasing speech cannot clear a blink/manual driver owned
  elsewhere.

Gate: driver ownership and release are deterministic under conflict, silence,
and component removal.

### 10. AVC routing

- [ ] Consume only enabled direct `Visemes` children of an AVC.
- [ ] Select the newest valid retained sample when more than one is present.
- [ ] Resolve the authoritative AVC-owned glTF.
- [ ] Resolve and cache canonical map slots to stable `MorphTargetKey`s.
- [ ] Refresh cached routing only when the glTF/map generation changes.
- [ ] Combine duplicate semantic channels targeting one label using the
  documented policy.
- [ ] Apply weights as temporary morph drivers.
- [ ] Release drivers on every neutral/stale/failure/teardown path.
- [ ] Diagnose missing map, missing label, duplicate label, and ownership
  conflict once rather than every frame.

Gate: fake retained frames drive only the intended synthetic morph targets and
every invalidation path restores the correct base/other-driver result.

### 11. Focused example and end-to-end validation

- [ ] Add a focused MMS microphone/VTuber example using the target surface.
- [ ] Include an explicit five-vowel `MorphTargetMap`.
- [ ] Keep microphone monitoring absent by default.
- [ ] Add an optional explicit `AudioOutput { microphone }` validation mode.
- [ ] Surface selected device, negotiated format, worker status, overruns, and
  estimated latency in useful diagnostics.
- [ ] Validate desktop, mirror, and XR consumers use the same cached morph
  deformation result.
- [ ] Measure capture-to-visible response, worker CPU, main-thread cost, and
  render impact on target hardware.

Gate: speaking moves the example avatar's lower face, silence releases it,
monitoring is opt-in, and the target scene remains stable under repeated
enable/disable and device-loss tests.

## Required automated coverage

- `MorphTargetMap` canonical-mouth slot validation and MMS round-trip.
- `AudioInput` constructor/catalog and invalid-device behavior.
- capture consumer reference counting and generation-safe teardown.
- partial callback block, queue-full, sequence-gap, and consumer-disappearance
  behavior.
- direct `AudioInput -> InputSource` graph compilation.
- render underrun produces silence without blocking.
- `Visemes.from(...)` validates the referenced component type.
- worker unit capacity, registration, reset, removal, and shutdown.
- deterministic PCM-to-canonical-weight fixture output.
- stale worker result rejection.
- silence/failure/removal neutralization.
- AVC direct-child eligibility and newest-sample selection.
- partial mapping, missing labels, duplicate mapping, and driver conflict.
- base-factor restoration and preservation of other driver ownership.

## Live acceptance record

Record the following before marking the slice complete:

- hardware, OS/audio host, input device, sample format, and backend/model;
- end-to-end capture-to-visible latency;
- stable output update rate, targeting at least 30 Hz;
- first visible response, targeting at most 100 ms;
- release start after confirmed silence, targeting at most 150 ms;
- worker CPU and memory use while speaking and silent;
- main/render frame-time comparison with analysis disabled and enabled;
- queue overrun and recovery exercise;
- device disconnect/reconnect or disable/re-enable exercise;
- desktop, mirror, and XR visual agreement where available.

## Explicitly deferred to v2

The first slice does not expose retained weights to MMS code. V2 adds live
host-dispatched receiver methods:

```mms
let all = visemes.weights() // f32[]
let aa = visemes.weight(0)  // f32
let names = visemes.names() // string[]
let name = visemes.name(0)  // string
```

Those calls follow this lookup path:

```text
MMS live ComponentHandle
    -> validate session, generation, and declared method
    -> resolve host mapping to engine ComponentId
    -> look up and downcast VisemesComponent
    -> copy its latest main-thread-retained snapshot
    -> return ordinary MMS f32/string data
```

The raw engine `ComponentId` never crosses into MMS. Removed or reused
components produce typed stale-handle errors. Reads do not message the worker,
block for a frame, or expose worker-owned memory.

## Other non-goals for this slice

- speech-to-text or voice commands;
- recording or network voice transport;
- echo cancellation or noise suppression;
- automatic microphone monitoring;
- post-effect graph analysis without an explicit future tap;
- automatic mapping for every avatar convention;
- multiple analysis backends consuming one input;
- mutable MMS access to viseme state;
- a general facial-expression mixer beyond the ownership required for safe
  viseme release.

## Stop condition

Stop after the focused MMS scene passes the automated and live gates above.
Do not expand the slice into v2 receiver reads, recording, processed graph taps,
speech-to-text, device-management UI, or general face-driver composition unless
one of those is separately approved.

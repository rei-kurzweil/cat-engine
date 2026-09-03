# Audio input and visemes

Date: 2026-09-01
Status: proposed specification

## 1. Purpose

This specification defines the authored and runtime boundary for live audio
input and microphone-driven avatar visemes.

The core decisions are:

- `AudioInput` is an `AudioSource`, equivalent at the authored graph layer to
  `AudioOscillator` and `AudioClip`.
- the compiled runtime graph represents it as
  `AudioGraphNodeKind::InputSource`.
- `Visemes.from(audio_input)` creates an attachable component which subscribes
  to raw PCM from that input.
- a `Visemes` component attached directly beneath `AVC` supplies retained
  semantic mouth weights to `AvatarControlSystem`.
- `MorphTargetMap` maps canonical viseme names to model-specific glTF morph
  target labels.
- v2 adds host-dispatched read methods on the live `Visemes` component so MMS
  code can inspect its latest retained data.

The related execution plan is
[the microphone-driven visemes epic](../task/epic/visemes.md). The implementation
checklist for v1 is
[audio input and visemes: first slice](../task/audio-input-and-visemes-first-slice.md).

## 2. Vocabulary

| Term | Meaning | Layer |
| --- | --- | --- |
| `AudioSource` | Any component which supplies audio samples to a compiled graph | architecture |
| `AudioInput` | Live host/device PCM source | ECS + MMS |
| `InputSource` | Runtime audio-graph node compiled from `AudioInput` | audio rendering thread |
| `Visemes` | Speech-analysis component referencing an `AudioInput` | ECS + MMS |
| viseme unit | Preallocated worker-owned analysis state for one live `Visemes` component | audio-input processing thread |
| viseme frame | Timestamped canonical weight vector returned by the worker | worker -> main protocol |
| `MorphTargetMap` | Canonical semantic channel to imported target-label map | avatar/glTF authoring |

An `AudioInput` is not a separate device-resource wrapper around an audio
source. It is the source.

## 3. MMS v1 surface

### 3.1 Default input and viseme analysis

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

`AudioInput {}` selects the default host input device. Assigning it to `let`
registers one live component handle. `Visemes.from(microphone)` stores an
explicit reference to that handle. Referencing `visemes` in the `AVC` body
attaches that exact live component beneath the avatar controller.

The `AudioInput` does not need to be an AVC child. The reference selects the
audio source; `Visemes` topology selects the avatar consumer.

### 3.2 Device selection

Named constructor forms create `AudioInput` components:

```mms
let default_microphone = AudioInput {}
let explicit_default = AudioInput.default() {}
let numbered_microphone = AudioInput.device_number(1) {}
let named_microphone = AudioInput.device("USB Microphone") {}
```

These calls look like type-level methods, but in MMS they are component
constructors. Each returns a new `AudioInput` component expression/live handle;
they are not methods on an existing input instance.

`device_number` is session-local and intended primarily for setup tools and
experiments. Persisted scenes should prefer a stable device identifier once
the supported CPAL hosts provide an adequate identity contract. A display name
alone may be ambiguous.

Device enumeration is a host API because it returns data instead of a
component. Component types can also own static host APIs, so the discovery
calls live beside their corresponding component types:

```mms
let devices = AudioInput.devices()         // string[]
let output_devices = AudioOutput.devices()  // string[]
```

Component types may expose static host APIs as well as component constructors.
`AudioInput.devices()` and `AudioOutput.devices()` are the adopted first slice;
the broader static-API design remains documented in
`docs/draft/audio-input-output-static-apis.md`.

### 3.3 Audio graph use

`AudioInput` is a peer source:

```text
AudioSource
├── AudioOscillator
├── AudioClip
└── AudioInput
```

It may be attached directly into an audio graph:

```mms
let microphone = AudioInput {}
let visemes = Visemes.from(microphone)

AudioOutput {
    microphone
}

AVC {
    visemes
}
```

The two uses are independent:

- graph attachment compiles the microphone as an audible `InputSource`;
- `Visemes.from(microphone)` subscribes the analysis worker to raw source PCM;
- without graph attachment, the microphone can still feed viseme analysis but
  is inaudible;
- without a `Visemes` subscriber, graph attachment can still monitor or
  process the microphone.

There is no `AudioInputSource.new(microphone)` wrapper component.

An `AudioInput` becomes active while it has at least one enabled consumer: a
compiled graph node, a live `Visemes` subscription, or a future explicit
capture consumer. Removing its final consumer allows `AudioInputSystem` to
stop the device stream. This consumer-driven lifecycle avoids opening an
unused microphone merely because a detached component was declared.

## 4. Component contracts

### 4.1 `AudioInputComponent`

Conceptual authored state:

```rust
pub struct AudioInputComponent {
    pub device: AudioInputDeviceSelector,
    pub requested_sample_rate: Option<u32>,
    pub requested_channels: Option<u16>,
    pub enabled: bool,
    pub status: AudioInputStatus,
}
```

The component does not own CPAL objects or PCM buffers. `AudioInputSystem`
owns runtime streams and queues keyed by the component's generation-safe
identity.

Required constructors/builders:

| Surface | Result/effect |
| --- | --- |
| `AudioInput {}` | default-device component |
| `AudioInput.default() {}` | explicit default-device component |
| `AudioInput.device_number(index) {}` | session-local enumerated device |
| `AudioInput.device(id_or_name) {}` | selected device identity |
| `enabled(bool)` | durable capture intent |

### 4.2 `VisemesComponent`

Conceptual authored and retained state:

```rust
pub struct VisemesComponent {
    pub source: ComponentRef,
    pub enabled: bool,
    pub backend: VisemeBackendSelector,
    pub language: Option<String>,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub silence_release_ms: f32,
    pub min_confidence: f32,
    pub sample: Option<VisemeSample>,
    pub status: VisemesStatus,
}
```

Required constructor/builders:

| Surface | Result/effect |
| --- | --- |
| `Visemes.from(audio_input)` | component referencing one `AudioInput` |
| `enabled(bool)` | analysis lifecycle intent |
| `backend(name)` | backend preset |
| `language(tag)` | recognizer language hint |
| `attack_ms(value)` | attack/coarticulation policy |
| `release_ms(value)` | release/coarticulation policy |
| `silence_release_ms(value)` | stale/silence release deadline |
| `min_confidence(value)` | uncertain-output rejection threshold |

The constructor must reject a component which is not an `AudioInput`. The
first slice permits one active `Visemes` analyzer per input; broader fan-out
requires an explicit capacity and scheduling policy.

## 5. Runtime graph contract

The first audio-input slice extends both authored and runtime source enums:

```rust
pub enum AudioGraphNodeKind {
    OscillatorSource { voices: usize },
    ClipSource,
    InputSource,
    // effects...
}
```

The RT form may contain a compact capture-runtime key, channel policy, and a
consumer for a dedicated bounded queue. It must not contain CPAL device
enumeration or speech-recognition state.

The capture callback fans out fixed PCM blocks to independently bounded
consumers:

```text
CPAL input callback
├── capture -> render SPSC queue     (only when graph InputSource is active)
└── capture -> viseme SPSC queue     (only when Visemes is active)
```

No consumer may stall another. Queue saturation is observable and nonblocking.
The render consumer and analysis consumer receive separate queues; an SPSC
consumer is never shared.

Speech analysis reads raw capture PCM in v1. Graph effects do not alter the
analysis feed. A future explicit graph-tap API may analyze a processed signal,
but that is not implicit behavior of `Visemes.from(audio_input)`.

## 6. Viseme worker and main-thread handoff

Creating and enabling `Visemes.from(microphone)` causes `VisemeSystem` to
reserve and configure a preallocated viseme unit on the audio-input processing
thread.

```text
main VisemeSystem
    RegisterUnit { visemes_id, generation, input_id, config }
                       │
                       ▼
audio-input processing thread
    preallocated VisemeUnit
    recognizer + temporal state
                       │
                       │ VisemeFrame { id, generation, time, weights }
                       ▼
main VisemeSystem
    latest retained VisemeSample on VisemesComponent
                       │
                       ▼
AvatarControlSystem -> MorphTargetMap -> GLTF morph drivers
```

The worker never accesses `World`, `ComponentId` topology, `AVC`, glTF target
labels, or morph factors. Results are tagged with component identity and
generation. The main thread rejects stale results after disable, reconfigure,
removal, or device restart.

The worker's canonical output is an ordered, fixed-size weight vector. Its
order and names come from the same authoritative semantic-channel registry
used by `MorphTargetMap`; the backend does not define the public order.

## 7. AVC relationship

A `Visemes` direct child of `AVC` is eligible to drive that avatar. Per tick,
after worker results are drained:

1. `AvatarControlSystem` selects the newest valid retained direct-child sample.
2. It resolves the AVC-owned glTF and its child `MorphTargetMap`.
3. It maps canonical viseme indices/names to stable `MorphTargetKey`s.
4. It applies the current weights as temporary morph drivers.
5. Silence, timeout, disable, source failure, or removal releases those drivers
   to their imported base values.

Attaching `Visemes` beneath AVC does not reparent its `AudioInput`. The source
relationship remains the explicit component reference captured by `.from(...)`.

## 8. V2: host-dispatched viseme reads

V2 exposes the retained canonical data to MMS through methods on the live
`Visemes` instance:

```mms
let microphone = AudioInput {}
let visemes = Visemes.from(microphone)

let all = visemes.weights() // f32[]
let aa = visemes.weight(0)  // f32
let names = visemes.names() // string[]
```

The receiver uses ordinary `.` method syntax. These are not constructor calls,
builder calls, local table reads, or copied fields captured when the component
was created.

### 8.1 Host-object semantics

`visemes` is already a live, host-owned `ComponentObject` with ECS topology
privileges. V2 does not need to convert it into a separate non-component host-
object class. Reading live analysis data uses the configured component-method
host protocol:

```text
MMS evaluator/session
    InvokeComponentMethod {
        receiver: live Visemes component handle,
        method: weights / weight / names,
        args,
    }
             │
             ▼
Mittens host on the main thread
    validate generation and component type
    snapshot retained VisemeSample
             │
             ▼
HostResponse containing copied MMS data
```

Each call therefore messages the host and reads the latest main-thread-retained
sample. MMS never receives a pointer or borrow into worker/ECS storage. Arrays
returned by `weights()` and `names()` are script-owned snapshots; changing them
does not modify the component or worker.

This follows the general host-owned receiver direction in
`crates/meow-meow-script/docs/draft/host-values-resources-and-bound-receivers.md`,
while retaining the component's scene-topology identity.

### 8.2 V2 method contract

| Method | Return | Contract |
| --- | --- | --- |
| `visemes.weights()` | `f32[]` | snapshot of all canonical weights in canonical order |
| `visemes.weight(index)` | `f32` | weight at one canonical index; typed out-of-range error |
| `visemes.names()` | `string[]` | canonical names aligned one-to-one with `weights()` |
| `visemes.name(index)` | `string` | canonical name at one index; typed out-of-range error |

Indices are zero-based. The array lengths are stable for one engine/version
canonical registry and do not shrink to only the avatar's mapped targets.
The following invariants hold:

```text
length(names()) == length(weights())
name(i) corresponds to weight(i)
```

Before the first valid frame, after a stale timeout, or during silence, weight
reads return the neutral canonical vector (all zeroes). They do not expose the
previous voiced frame indefinitely. Device/backend failure remains observable
through component status/diagnostics; a read does not block waiting for a new
frame.

Each method call is a distinct snapshot. Two calls separated by an engine tick
may observe different frames. If scripts later require atomic access to weights
plus timestamp/sequence/status, add one `snapshot()` result rather than
pretending several host round trips are coherent.

### 8.3 V2 implementation requirements

- register the methods and result signatures in the MMS runtime catalog;
- dispatch through the canonical host-side component method implementation;
- return arrays through ordinary MMS/transport values;
- validate the live handle and `Visemes` component generation;
- perform no worker round trip from a read method;
- never block awaiting capture or inference;
- copy only the retained main-thread snapshot;
- test top-level calls and calls inside runtime closures;
- test neutral, live, stale, disabled, removed, and out-of-range behavior.

The absence of a worker round trip is important: “host-dispatched” means the
MMS session asks the Mittens host for host-owned data. The host answers from
state already handed back to the main thread.

## 9. First-slice boundary

V1 includes:

- `AudioInput` component constructors and lifecycle;
- `AudioInput.devices()` enumeration;
- `AudioGraphNodeKind::InputSource` and direct audio-graph compilation;
- detached-but-consumed microphone capture for `Visemes`;
- `Visemes.from(audio_input)` and its configuration builders;
- preallocated worker unit registration and bounded thread protocols;
- retained main-thread viseme frames;
- direct-child AVC routing through `MorphTargetMap`;
- silence/failure/removal release behavior.

V2 includes:

- `weights()`, `weight(index)`, `names()`, and `name(index)` live receiver
  methods;
- host-dispatched copied snapshots and typed method signatures;
- optional later `snapshot()` if coherent metadata plus weights is required.

V2 does not move inference into MMS, expose worker-owned memory, or make the
`Visemes` component itself part of the audio graph.

## 10. Non-goals

- making microphone input audible merely because `AudioInput` exists;
- analyzing post-effect graph output without an explicit future tap;
- returning backend-specific phoneme labels as canonical names;
- exposing mutable references to retained weights;
- blocking MMS until a new recognition frame arrives;
- making `Visemes` an audio source or audio effect;
- using a second wrapper component around `AudioInput` for graph compilation.

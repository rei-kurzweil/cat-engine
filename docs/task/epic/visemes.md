# Epic: microphone-driven visemes

Date: 2026-09-01
Status: proposed

## Outcome

Mittens can capture a microphone without blocking either the main thread or the
audio rendering callback, recognize speech into a small stream of semantic
mouth poses, and drive a VTuber avatar's mapped morph targets with predictable
latency and release behavior.

The public model has four separate responsibilities:

- `AudioInput` is a live-capture `AudioSource`, alongside `AudioOscillator` and
  `AudioClip`.
- `Visemes` configures speech analysis and retains its latest semantic
  output.
- `AvatarControlSystem` routes retained semantic channels to the controlled
  avatar.
- `MorphTargetMap` maps those channels to model-specific morph target labels.

The first audio-input slice compiles an attached `AudioInput` as
`AudioGraphNodeKind::InputSource`. Graph attachment makes it audible; a
detached input referenced by `Visemes` remains usable for mouth movement
without being routed to speakers.

## Why this is a separate epic

The repository already has most of the downstream path:

- CPAL output runs in its own real-time callback and receives bounded messages
  from the main thread.
- clip decoding runs on a named worker with an explicit request/completion
  protocol.
- the audio graph compiler lowers authored audio components into runtime-only
  nodes.
- `MorphTargetMapComponent` maps semantic channels to imported glTF target
  labels.
- `AvatarControlSystem` already consumes retained eye-tracking samples and
  applies mapped blink drivers without putting transport logic in the renderer.
- the render path already treats a morph driver as temporary state and restores
  the imported base factor when the driver becomes `None`.

What is missing is the capture and speech-analysis side, a retained viseme
sample, and a generic mouth-channel bridge parallel to the current blink bridge.

One important current-state correction: `MorphTargetMapComponent::slot(...)`
only accepts `left_eye_blink` and `right_eye_blink` today. The engine may have
canonical model morph labels elsewhere, but the semantic mouth-channel
vocabulary is not yet represented by this component. This epic must extend the
existing map deliberately instead of assuming mouth slots already work.

## Architectural decision

Use a capture callback plus a dedicated speech-analysis worker. Do not run
phoneme recognition in the CPAL input callback, the CPAL output callback, or the
main thread.

```text
                         control / lifecycle
                 Main thread ───────────────────────┐
                     ▲                              │
                     │ timestamped SpeechFrame      ▼
                     │                       Speech analysis worker
                     │                              ▲
                     │                              │ bounded PCM blocks
                     │                              │
AvatarControlSystem ─┘                     CPAL input callback
        │                                          │
        │ semantic channel weights                 │ independent bounded tap
        ▼                                          ▼
MorphTargetMap -> GLTF morph drivers        RT `InputSource`
```

These are distinct execution contexts:

1. **CPAL input callback** — an OS/audio-host callback. It converts/copies input
   into preallocated fixed-size blocks and pushes them to bounded queues. It
   performs no inference, ECS access, allocation, logging, locking, or waiting.
2. **Speech-analysis worker** — a named ordinary worker thread. It resamples or
   remixes to the recognizer format if needed, runs voice activity and the
   recognition backend, performs phoneme-to-viseme reduction and temporal
   smoothing, then publishes timestamped semantic frames.
3. **Main thread** — owns components and lifecycle, drains worker results,
   rejects stale generations, stores the latest retained sample, and applies it
   through the avatar/morph path.
4. **CPAL output callback** — continues to own audio rendering. It does not
   participate in speech recognition. A compiled `InputSource` reads its own
   separate capture queue.

The capture callback is already a thread from the engine's perspective. We do
not add another thread merely to rename it an “input thread”; the new engine-
owned thread is specifically the speech-analysis worker.

## Ownership boundaries

| Layer | Owns | Must not own |
| --- | --- | --- |
| `AudioInputComponent` | authored device/config intent and observable status | PCM buffers, recognizer model, glTF state |
| capture runtime | CPAL stream, negotiated format, sequence counter, bounded producers | ECS mutation, speech inference |
| speech worker | recognizer backend and its temporal state | `World`, `ComponentId` lookup, morph labels |
| `VisemesComponent` | source reference, analysis policy, latest retained frame/status | worker implementation details |
| `AvatarControlSystem` | selection of the newest direct tracker and avatar routing | device access, phoneme recognition |
| `MorphTargetMapComponent` | semantic channel -> model target label | tracker/backend-specific names |
| `GLTFComponent` | per-instance morph factor state | capture or analysis lifecycle |

As with the current audio graph, authored ECS components and runtime worker
objects are allowed to differ. ECS is scene vocabulary; runtime structs are
chosen for bounded queues and low-latency processing.

## Thread protocol

### Capture to analysis

Use a bounded single-producer/single-consumer queue, following the audio render
queue's `rtrb` pattern. The item should be a fixed-capacity PCM block rather
than an allocated `Vec<f32>`.

Conceptual shape:

```rust
struct CapturedAudioBlock {
    source: AudioInputRuntimeId,
    generation: u64,
    sequence: u64,
    capture_frame_start: u64,
    sample_rate: u32,
    channels: u16,
    valid_frames: u16,
    samples: [f32; CAPTURE_BLOCK_SAMPLES],
}
```

Rules:

- the callback never blocks when the queue is full;
- newest audio is not silently presented as contiguous with dropped audio;
- an overrun increments an atomic counter and creates a sequence gap;
- the worker resets backend temporal state when it observes a generation
  change or an unexplained sequence gap;
- timestamps are based on a monotonic captured-frame counter, not main-thread
  arrival time;
- device-native samples are normalized to `f32` in the callback only if that
  conversion is bounded and allocation-free;
- channel remix and recognizer-rate conversion belong on the worker.

If the input is later consumed by both speech analysis and the audio graph,
give each consumer its own bounded SPSC queue. An SPSC consumer cannot be
shared, and neither consumer may stall the other. The capture runtime may copy
each fixed block to enabled subscribers; fan-out limits must be explicit.

### Main to analysis control

Control traffic is low rate and does not originate in the capture callback, so
it can use a normal channel:

```rust
enum SpeechWorkerCommand {
    Configure {
        source: AudioInputRuntimeId,
        generation: u64,
        recognizer: RecognizerConfig,
        smoothing: VisemeSmoothingConfig,
    },
    Reset { generation: u64 },
    Shutdown,
}
```

The first implementation should permit one active `Visemes` analysis
session per `AudioInput`. Supporting multiple recognizers per input requires an
explicit fan-out policy and is not accidental behavior.

### Analysis to main

The worker publishes semantic data, status, and diagnostics. It does not send
closures that execute on the main thread and it does not carry model-specific
morph labels.

```rust
enum SpeechWorkerEvent {
    Ready { generation: u64, backend: String },
    Frame(SpeechFrame),
    Overrun { generation: u64, dropped_blocks: u64 },
    Failed { generation: u64, reason: String },
    Stopped { generation: u64 },
}

struct SpeechFrame {
    generation: u64,
    sequence: u64,
    capture_frame_start: u64,
    capture_frame_end: u64,
    voiced: bool,
    confidence: f32,
    phonemes: Vec<PhonemeObservation>,
    visemes: VisemeWeights,
}
```

The wire/runtime form of `SpeechFrame` should be bounded. The `Vec` above is
explanatory; use fixed-capacity storage or an owned result allocated on the
ordinary worker and transferred through a bounded result queue after measuring.
No such allocation may occur in either CPAL callback.

The main thread drains all available events once per engine tick and retains
the newest valid frame for each component. Events from older generations are
discarded. Queue overflow, backend failure, device removal, component removal,
and timeout all clear the retained driver rather than freezing the last mouth
pose.

## Phonemes versus visemes

The recognition backend and the rig use different vocabularies:

- a **phoneme observation** is backend/language evidence with a time range and
  confidence;
- a **viseme** is a language-reduced visual mouth pose suitable for animation;
- a **morph target label** is a model-specific imported glTF name.

Do not map backend phoneme strings directly to glTF labels. The pipeline is:

```text
backend phonemes/probabilities
    -> canonical viseme weights
    -> coarticulation / attack / release policy
    -> MorphTargetMap semantic slots
    -> model-specific target labels
```

The first backend may produce viseme probabilities directly. It should still
publish through the same canonical `VisemeWeights` boundary. Raw phoneme
observations are useful for diagnostics and future language-specific mapping,
but avatar routing consumes visemes.

### Semantic channel set

Before implementation, inventory the repository's intended canonical mouth
names and make one registry authoritative for `MorphTargetMap` validation,
serialization, MMS signatures, and diagnostics. Do not create a second list in
the speech backend.

If no complete canonical set exists, adopt the common 15-shape set below as
the engine semantic vocabulary:

```text
viseme_sil, viseme_pp, viseme_ff, viseme_th, viseme_dd,
viseme_kk, viseme_ch, viseme_ss, viseme_nn, viseme_rr,
viseme_aa, viseme_e, viseme_ih, viseme_oh, viseme_ou
```

This is a semantic interface, not a requirement that every avatar contain 15
targets. Missing slots are ignored and diagnosed once. A five-vowel VRM-style
avatar may map only `aa/e/ih/oh/ou`; a reduction preset can fold consonant
visemes into those available channels. Silence is normally expressed by
releasing mouth drivers, not by requiring a `viseme_sil` target.

The mapping policy must define how multiple active semantic channels combine
when several slots resolve to the same target label. The proposed default is
`max`, clamped to the valid driver range, because summing can overdrive a
single target during coarticulation.

## Proposed ECS components

### `AudioInputComponent`

Authored capture endpoint:

```rust
pub struct AudioInputComponent {
    pub device: AudioInputDeviceSelector,
    pub requested_sample_rate: Option<u32>,
    pub requested_channels: Option<u16>,
    pub enabled: bool,
    pub status: AudioInputStatus,
}
```

`device` should support `Default` first and a stable host/device selector later.
The negotiated format and recoverable diagnostics are observable, but the CPAL
stream and queues live in `AudioInputSystem` runtime state keyed by component.

Removing, disabling, or materially reconfiguring the component increments its
generation and tears down the old stream. Shutdown must not wait from an audio
callback.

### `VisemesComponent`

Tracker/analysis declaration:

```rust
pub struct VisemesComponent {
    pub source: ComponentRef, // must resolve to AudioInput
    pub enabled: bool,
    pub backend: SpeechBackendSelector,
    pub language: Option<String>,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub silence_release_ms: f32,
    pub min_confidence: f32,
    pub sample: Option<VisemeSample>,
    pub status: VisemesStatus,
}
```

Like eye tracking, a `Visemes` direct child of `AVC` is eligible to drive
that avatar. Its `source` is an explicit handle; topology does not imply which
microphone to use. If several enabled direct speech trackers exist, select the
newest valid sample by monotonic sequence/time, matching the retained-sample
approach used for eye tracking.

Keep smoothing policy on this component, not on `MorphTargetMap`: smoothing is
about the temporal signal, while the map is static rig semantics.

### `MorphTargetMapComponent`

Extend the existing component; do not add a viseme-specific map component.

```mms
MorphTargetMap.new()
    .slot("left_eye_blink", "BlinkLeft")
    .slot("right_eye_blink", "BlinkRight")
    .slot("viseme_aa", "Fcl_MTH_A")
    .slot("viseme_ih", "Fcl_MTH_I")
    .slot("viseme_ou", "Fcl_MTH_U")
    .slot("viseme_e", "Fcl_MTH_E")
    .slot("viseme_oh", "Fcl_MTH_O")
```

The map remains a direct child of the owning `GLTF`. Values remain imported
target labels; keys are canonical semantic channels.

No new public “set morph by string every frame” API is needed for this epic.
The avatar bridge should resolve slots to stable `MorphTargetKey`s when the
GLTF/map generation changes, cache that routing, and update driver state by key.

## Proposed MMS surface

Component construction and builder configuration should follow the current MMS
component registry/runtime catalog model:

```mms
let microphone = AudioInput {}

AVC {
    Visemes.from(microphone) {
        language("en")
        attack_ms(35)
        release_ms(80)
        silence_release_ms(140)
        min_confidence(0.25)
    }

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

Proposed constructors/builders:

| Component | Surface | Meaning |
| --- | --- | --- |
| `AudioInput` | bare `{}` or `.default()` | default host input device |
| `AudioInput` | `.device_number(index)` | session-local enumerated input device |
| `AudioInput` | `.device(name_or_id)` | explicit device identity |
| `AudioInput` | `.enabled(bool)` | capture lifecycle intent |
| `Visemes` | `.from(audio_input)` | explicit capture source |
| `Visemes` | `.backend(name)` | backend preset; optional in the first slice |
| `Visemes` | `.language(tag)` | backend language hint |
| `Visemes` | `.attack_ms/.release_ms(...)` | coarticulation envelope |
| `Visemes` | `.silence_release_ms(...)` | stale/silence mouth release |
| `Visemes` | `.min_confidence(...)` | reject uncertain observations |

Runtime methods such as `.start()` and `.stop()` are not needed initially;
`enabled(...)` is durable authored state and lifecycle follows component attach,
update, and removal. If imperative methods are later added, they must use the
canonical host-side component method dispatch rather than evaluator-only logic.

Device enumeration is a host API rather than a component constructor:

```mms
let devices = Audio.input_devices() // string[]
```

The separate `Audio` namespace avoids making `AudioInput` ambiguous between a
component type and a built-in/host API namespace.

## Relationship to the audio graph

`AudioInput` is directly an `AudioSource`:

- `AudioOscillator` synthesizes samples;
- `AudioClip` reads decoded PCM;
- `AudioInput` reads live captured PCM.

The same live input handle may be attached to an audio graph and referenced by
`Visemes`:

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

The first slice extends `AudioGraphNodeKind` with `InputSource` and compiles an
attached `AudioInput` directly to that runtime node. There is no separate
`AudioInputSource.new(microphone)` wrapper.

Graph and analysis consumers receive separate bounded capture queues. A
detached input referenced by `Visemes` remains inaudible. Attaching it beneath
`AudioOutput` makes monitoring explicit and therefore carries the ordinary
feedback risk. Recording, echo cancellation, noise suppression, spatialization,
and network voice transport are separate workstreams.

## V2 live data access from MMS

V2 makes a live `Visemes` instance readable as a host-owned component receiver:

```mms
let microphone = AudioInput {}
let visemes = Visemes.from(microphone)

let all = visemes.weights() // f32[]
let aa = visemes.weight(0)  // f32
let names = visemes.names() // string[]
```

These use ordinary `.` receiver syntax and the MMS component-method host
protocol. Each call messages the Mittens host, validates the live component
handle, and returns a copied snapshot of the newest viseme data retained on the
main thread. It never reads worker memory directly and never blocks waiting for
inference.

`weights()` and `names()` use one authoritative canonical ordering;
`weight(i)` and `name(i)` address that same ordering. Before the first valid
frame and after silence/staleness, weight reads return the neutral zero vector.
V2 is specified fully in
[`docs/spec/audio-input-and-visemes.md`](../../spec/audio-input-and-visemes.md).

## Avatar-control integration

Extend the existing retained-tracker pattern rather than creating a second
system that searches for avatars globally.

Per tick, after speech-worker events have been drained:

1. `AvatarControlSystem` finds enabled direct `Visemes` children.
2. It selects the newest valid retained sample.
3. It requests/resolves the AVC-owned glTF through the same authoritative
   avatar mapping path used by blink routing.
4. It resolves canonical viseme slots from the glTF-child `MorphTargetMap` to
   stable target keys, refreshing only when the glTF or map changes.
5. It writes semantic weights into each target's temporary `driver`.
6. Silence, timeout, invalid source, worker failure, disable, or removal writes
   `None`, restoring imported base factors.

Blink and speech may eventually target overlapping expression shapes. Driver
ownership therefore cannot remain a single anonymous `Option<f32>` forever.
Before adding the mouth bridge, choose one of:

- restrict each morph target to one live semantic driver and diagnose conflicts
  in the first slice; or
- replace the anonymous driver with small named driver slots and an explicit
  composition policy.

The second direction is the durable design, but it should be a focused morph-
driver task rather than hidden inside microphone work. Until it lands, the
viseme slice must prove that clearing speech cannot erase an eye or manual
driver on the same target.

## Backend selection criteria

Do not choose a recognition dependency only from offline transcription
quality. The spike must measure:

- streaming or bounded-window inference rather than whole-utterance waits;
- phoneme or viseme timing/probability output;
- algorithmic lookahead and end-to-end mouth-motion latency;
- CPU use on the target VR machine while rendering;
- model size, warm-up time, and steady-state allocation;
- license and redistributability of code and model weights;
- supported platforms and whether GPU inference creates renderer contention;
- deterministic shutdown and recovery after malformed input/device loss;
- useful behavior for silence, noise, singing, and non-English speech.

The backend belongs behind an engine-owned trait so tests can use a deterministic
fake:

```rust
trait StreamingVisemeBackend: Send {
    fn configure(&mut self, format: RecognizerFormat) -> Result<(), String>;
    fn reset(&mut self);
    fn push_audio(&mut self, mono_pcm: &[f32], frame_start: u64)
        -> Result<BackendOutput, String>;
}
```

Backend selection is a build/runtime packaging decision. It must not leak model-
specific phoneme strings into MMS or `MorphTargetMap`.

## Work breakdown

### Task 1: canonical semantic channel inventory

- locate the existing canonical mouth/morph naming source, if any;
- define one typed/constant registry shared by map validation and diagnostics;
- decide whether the 15-shape fallback above is the canonical set;
- document reductions for five-vowel rigs;
- extend `MorphTargetMap` serialization and MMS validation tests.

Exit: a mouth slot can be authored and round-tripped without adding backend-
specific names to the component API.

### Task 2: audio input component and device lifecycle

- add `AudioInputComponent` and MMS registration/catalog signatures;
- add `Audio.input_devices()` host API enumeration;
- add `AudioInputSystem` runtime ownership;
- negotiate the default CPAL input format and expose status/diagnostics;
- implement generation-safe start, disable, reconfigure, removal, and shutdown;
- use a fake capture source in automated tests.

Exit: fixed PCM blocks arrive off the callback with bounded behavior, and
component teardown cannot leave a live stream or blocked join.

### Task 3: audio graph `InputSource`

- add `AudioInput` to the authored `AudioSource` vocabulary;
- add `AudioGraphNodeKind::InputSource` and its RT equivalent;
- compile an attached `AudioInput` directly, without a wrapper component;
- give render and analysis consumers independent bounded queues;
- prove a detached analysis-only input is inaudible and an attached input is
  rendered through ordinary graph effects.

Exit: `AudioInput` is a first-class compiled audio source in the initial slice.

### Task 4: bounded capture protocol

- implement the fixed-block SPSC queue and preallocated callback staging;
- carry source generation, sequence, and captured-frame timestamps;
- expose overrun counters and sequence-gap behavior;
- test queue-full, partial callback block, format conversion, device restart,
  and consumer disappearance.

Exit: callback code is allocation-free/non-blocking and discontinuities are
observable.

### Task 5: backend spike and fixture corpus

- define the backend trait and deterministic fake;
- compare candidate streaming recognizers using recorded fixtures;
- record accuracy, latency, CPU, memory, packaging, and license evidence;
- select one backend or explicitly stop if no candidate meets the budget.

Exit: the epic has an evidence-backed backend choice and reproducible fixtures,
not only live-microphone anecdotes.

### Task 6: speech worker and retained component state

- add `VisemesComponent`, control/events, worker lifecycle, and statuses;
- perform worker-side remix/resample, inference, phoneme-to-viseme mapping,
  smoothing, and silence detection;
- drain events in `VisemeSystem` and reject stale generations;
- clear retained output on timeout, overflow reset, disable, failure, and
  removal.

Exit: deterministic PCM fixtures produce deterministic timestamped canonical
viseme frames without touching an avatar.

### Task 7: morph driver ownership/composition

- audit the current single `MorphFactorState::driver` ownership assumption;
- add named driver ownership/composition or enforce and diagnose exclusive
  ownership for the first slice;
- preserve imported base-factor restoration;
- ensure one tracker clearing cannot clear another active owner.

Exit: blink, speech, editor/manual control, and future face tracking have a
documented conflict policy.

### Task 8: AVC viseme routing

- consume only direct eligible `Visemes` children;
- resolve AVC glTF and cached map slots to structural keys;
- apply weights and release them on every invalid/stale lifecycle path;
- diagnose missing map, missing labels, duplicate labels, and driver conflicts
  once rather than every frame.

Exit: a fake speech backend drives a synthetic mapped avatar and releases to
base state on silence/removal.

### Task 9: MMS scene and live validation

- add a focused VTuber microphone example;
- show a five-vowel map and the selected reduction preset;
- expose status/overrun/latency diagnostics useful during live setup;
- validate desktop, mirror, and XR views use the same cached deformation result;
- measure capture-to-visible-mouth latency and render impact on target hardware.

Exit: speaking into the selected microphone moves the example avatar's mouth,
silence closes/releases it, and audio/render threads remain stable.

### V2 task: live viseme data access

- register `weights() -> f32[]`, `weight(index) -> f32`,
  `names() -> string[]`, and `name(index) -> string` on live `Visemes`;
- dispatch through canonical host-side component methods;
- return copied snapshots from retained main-thread state, never worker memory;
- define neutral, stale, disabled, removed, and index-error behavior;
- verify top-level and runtime-closure calls use identical host semantics.

## Validation and acceptance criteria

- no recognition, resampling, heap allocation, logging, mutex acquisition, or
  blocking send occurs in a CPAL callback;
- capture, analysis, main, and render execution contexts are independently
  testable with fake endpoints/backends;
- queue saturation drops bounded work, increments diagnostics, and recovers
  without deadlock or presenting discontinuous audio as continuous;
- stale worker generations cannot update a restarted/replaced component;
- silence and every teardown/failure path release morph drivers to base state;
- a missing/partial `MorphTargetMap` changes only the slots that resolve;
- five-vowel and fuller viseme rigs have deterministic reduction behavior;
- backend-specific phoneme names never become morph-target API keys;
- the same authored MMS surface works at initial load and after component
  disable/re-enable;
- microphone capture alone is inaudible; monitoring requires explicit graph
  authoring;
- measured capture-to-visible response and CPU cost are recorded for the target
  VTuber scene before the epic is marked complete.

Initial latency targets for the backend spike, subject to measurement:

- first visible response within 100 ms of voiced input;
- stable pose updates at 30 Hz or better;
- release begins within 150 ms of confirmed silence;
- no frame-time regression attributable to inference on the main/render
  threads, because inference never runs there.

## Non-goals

- speech-to-text or command recognition;
- microphone permission UI beyond surfacing host errors in the first slice;
- recording, network voice chat, echo cancellation, or noise suppression;
- automatic morph-label inference for every avatar format;
- making microphone input audible by default;
- running a recognizer in the audio graph or on either audio callback;
- a generic cross-avatar facial-expression mixer hidden inside this epic.

## Open decisions

1. Where is the authoritative existing canonical mouth-name list? Current
   `MorphTargetMap` code only proves the two blink channels.
2. Which streaming backend meets the latency, packaging, language, and license
   requirements on target hardware?
3. Should canonical output be 15 visemes, five vowels plus jaw/open, or a
   richer probability vector with named reduction presets?
4. Is one speech analyzer per input sufficient for the first supported API?
5. Does driver composition need to land before AVC routing, or can exclusive
   ownership be an explicitly diagnosed first-slice constraint?
6. What stale-sample timeout should apply independently of backend-declared
   silence?
7. Which device identity is stable enough to serialize across Linux, Windows,
   and VR-host changes?

## Related documents and code

- `docs/spec/audio-input-and-visemes.md` — authoritative v1/v2 component,
  graph, thread, AVC, and live-read API contract.
- `docs/draft/audio_decoding_thread.md` — existing worker/thread ownership
  model.
- `docs/spec/audio-sources.md` — audio vocabulary and authored-versus-runtime
  graph boundary.
- `docs/task/epic/eye-and-face-tracking.md` — retained tracker samples and AVC
  routing precedent.
- `docs/review/morph-target-lifecycle.md` — current morph ownership, driver
  release, and render-cache path.
- `src/engine/ecs/system/audio_decode_thread.rs` — request/completion/shutdown
  protocol precedent.
- `src/engine/ecs/system/audio_system.rs` and `audio_system_fundsp.rs` — bounded
  main-to-render queue and CPAL output callback constraints.
- `src/engine/ecs/system/avatar_control_system.rs` — current blink-to-map bridge.
- `src/engine/ecs/component/morph_target.rs` — `MorphTargetMap` and morph driver
  state.

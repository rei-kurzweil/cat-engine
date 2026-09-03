# Draft: static APIs on audio component constructor namespaces

Status: proposal only — no implementation or compatibility change

## Current surface and terminology

`AudioInput.device_number(n) {}` is a component-expression constructor. It
creates a new `AudioInput` component whose capture selector is the `n`th item
in the current host session's input-device enumeration. The index is not a
stable persisted identity: `0` means the first item enumerated for that host
session, `1` the second, and so on.

```mms
let default_input = AudioInput {}
let numbered_input = AudioInput.device_number(1) {}
```

`select_device_number(n)` is different: it is an instance method on an
existing live `AudioInput` component. It changes that component's capture
selector in a retained scene.

```mms
numbered_input.select_device_number(2)
```

The present `Audio` namespace is a built-in host-API table. At present it has
only `Audio.input_devices()`; it is not yet a complete public home for every
audio capability. It was introduced because device enumeration returns data,
rather than creating a component.

## Proposed surface

Allow a component constructor name to also expose static, type-level host
APIs. This would make audio-device discovery live beside the component that
consumes the result.

```mms
let input_devices = AudioInput.devices()
let output_devices = AudioOutput.devices()

// Set the session default used by inputs that have no explicit selector.
AudioInput.select_device_number(1)

let microphone = AudioInput.device_number(1) {}
microphone.select_device_number(2)
```

The same name would have two unambiguous roles:

- `AudioInput {}` and `AudioInput.device_number(n) {}` are component
  expressions/constructors;
- `AudioInput.devices()` is a static host call returning data; and
- `AudioInput.select_device_number(n)` sets the session default for
  `AudioInput` instances that use the implicit/default selector; and
- `microphone.select_device_number(n)` is an instance call on a live
  component.

`AudioOutput.devices()` would be the matching static query for output
endpoints. It does not imply that an `AudioOutput` component or graph-routing
contract is already implemented.

### Default-selection semantics

`AudioInput.select_device_number(n)` changes the session's preferred input for
all `AudioInput {}` instances whose selector remains implicit. It does not
overwrite an explicit `AudioInput.device_number(m) {}` selection and does not
change an input previously retargeted through its instance method.

The static call should reconfigure existing implicit inputs as well as govern
new implicit inputs created later in the session. This makes it useful for a
global microphone picker while preserving deliberate per-source choices.

An explicit way to return an instance to that session default may be useful in
a later slice:

```mms
microphone.use_default_device()
```

That is intentionally not part of this initial proposal.

## Why prefer this form

- Discovery, construction, and live retargeting are grouped by the type that
  owns their meaning.
- It avoids a broad catch-all `Audio` namespace used only for scattered static
  methods.
- It scales naturally to type-specific static queries, such as default-device
  metadata or supported formats, without suggesting that every audio graph
  operation belongs in one global table.

## Required runtime-language change

Today, the strict MMS runtime rejects one global identifier being both a
component type and a host namespace. Therefore `AudioInput.devices()` cannot
be added merely as another host binding.

The runtime would need a first-class **component static API** surface:

1. a component spec may register static methods in addition to constructors,
   builders, and instance methods;
2. call resolution recognizes `AudioInput.devices()` as a static call without
   treating `AudioInput` as a runtime component instance; and
3. type checking keeps the three forms distinct: component construction,
   static calls, and instance calls.

`AudioInput.select_device_number(n)` additionally needs session-owned audio
input preference state. Capture provisioning must resolve an implicit input
through that preference, and signal/reconfigure all existing implicit inputs
when it changes. Explicit per-instance selectors remain untouched.

The design should work generically for other component types, not special-case
audio.

## Compatibility and migration

Do not change the current API in the first implementation. Add
`AudioInput.devices()` alongside `Audio.input_devices()`, then deprecate the
latter only after the static-component mechanism is stable.

Possible future aliases:

```mms
AudioInput.default_device()
AudioOutput.default_device()
AudioInput.device_details(index)
AudioOutput.device_details(index)
```

These should return structured session-local device data once a stable device
identity model exists. Until then, names and numeric indices remain discovery
and testing tools, not portable scene configuration.

## Non-goals

- changing the current `AudioInput` or capture implementation;
- adding an `AudioOutput` component prematurely;
- defining audio graph routing syntax; or
- treating display names as stable device identifiers.

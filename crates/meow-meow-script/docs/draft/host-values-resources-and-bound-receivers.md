# Host values, resources, and bound receivers

Status: design draft; no crate implementation yet

Related:

- [Configure a script host](../how_to/configuring_a_script_host.md)
- [Transform component accessors](transform-component-accessors.md)
- [Compound MMS types](compound-types.md)
- [Checker and registry](checker-and-registry.md)
- [Mittens world-TRS implementation review](../../../../docs/review/world-trs-snapshot-and-vtuber-slidedeck.md)

## Problem

MMS already distinguishes ordinary script data from live component handles, but more host-backed
value shapes are inevitable:

```mms
transform.world.trs()
audio.playback.rate()
style.layout.width()

let image = ImageData.load("cat.png")
let audio = AudioData.load("meow.wav")
```

These examples are not all components:

- `transform.world` is a method interface bound to an existing transform component;
- `ImageData` may be decoded pixels with no world/topology identity;
- `AudioData` may be decoded samples shared by several playback components;
- a copied TRS is immutable data and retains no relationship to its source;
- a `Texture` or `AudioClip` component still has ordinary ECS lifecycle and topology semantics.

The first Mittens implementation routes `transform.world` through component-method machinery with
the synthetic component-type string `"TransformWorld"`. That proves the syntax and behavior, but
it is not the general semantic model. The language crate should not require every host-backed
receiver to pretend to be a component or add a dedicated `Value` enum variant.

## Goals

- Represent host-backed values which can be stored in MMS variables, arrays, tables, objects, and
  closure captures.
- Support member access which returns another host-backed receiver.
- Support methods on components, resources, immutable host values, and bound interfaces through
  one receiver protocol.
- Preserve the semantic difference between copied data, resource identity, bound capability, and
  ECS component identity.
- Keep engine-specific types such as `ComponentId`, `TransformTrs`, `TransformSpace`, image
  decoders, and audio buffers outside `meow-meow-script`.
- Keep handles session-scoped, opaque, generation-safe, and valid across the host boundary.
- Let the runtime catalog validate members, methods, argument types, result types, and required
  capabilities before dispatch.
- Avoid copying large image/audio buffers into general `Vec<Value::Number>` representations.

## Non-goals

- Define concrete image, texture, audio, transform, or style APIs.
- Make arbitrary Rust objects directly visible to scripts.
- Treat all host resources as components.
- Give bound receivers automatic serialization across sessions or processes.
- Add JavaScript prototype mutation or arbitrary property injection to host values.
- Decide the final typed-array/buffer API.
- Require plugins or dynamic libraries in the first implementation.

## Semantic categories

All four categories may expose methods, but they have different ownership and mutation semantics.

### Script-owned data

Ordinary MMS values are fully owned by the language runtime:

```text
null, bool, number, string, arrays, tables, records, tuples, nominal structs
```

Small structured results should prefer these forms once the type system can express them. For
example, a future public TRS value may be a nominal immutable struct or tuple rather than a
permanent host-specific runtime variant.

Copying script-owned data follows the language's normal value/reference rules. It does not require
host dispatch.

### Immutable host values

An immutable host value is data whose representation or operations remain host-defined:

```text
TransformTrs during the untyped migration
compiled regular expression
parsed path
immutable geometry descriptor
```

It has value semantics even if the implementation shares storage behind an opaque handle:

- methods cannot mutate the existing value;
- an operation which changes it returns another value;
- retaining it does not retain a relationship to the component/resource which produced it;
- deleting the source does not invalidate it.

The first generic implementation may define equality only for identical handles and leave
cross-handle semantic equality unsupported. Before MMS exposes general `==` for host values, the
host type contract must explicitly choose identity equality, structural equality, or no equality.

### Host resources

A host resource has identity and host-managed storage but is not an ECS component:

```text
decoded image pixels
decoded audio samples
font data
GPU-independent mesh data
network/file streams
```

Large resources should normally be represented by opaque handles, possibly backed by `Arc` or a
host resource arena. Copying the MMS value copies the handle, not the full byte/sample buffer.

Whether resource methods mutate the resource, return a new resource, or return a script-owned
snapshot is part of the registered host type contract.

### Bound receivers

A bound receiver selects an interface on an existing host owner:

```mms
let world = transform.world
let playback = audio.playback

world.trs()
playback.rate(1.25)
```

Conceptually:

```rust,ignore
BoundReceiver {
    owner: HostOwner,
    interface: HostTypeId,
}
```

It is not:

- a clone of the owner;
- copied owner data;
- a child component;
- a Rust borrow stored inside the MMS heap;
- a string masquerading as a component type.

It is a host-owned, session-scoped capability object. Its methods operate on the bound owner under
the selected interface.

Bound receivers should be first-class and storable. This is both useful and simpler than inventing
an expression-only value category:

```mms
let world = transform.world
world.trs()
```

Storing the receiver does not convert it into a copied transform. If its owner is removed, later
calls fail with a stale-owner/receiver error and must never silently retarget a reused native ID.

### Components

A component is a host object with additional structural privileges:

- it can be emitted, attached, detached, queried, and addressed by scene topology;
- component expressions materialize or register it;
- component-only APIs can require component receiver class.

Components may participate in the general member/method protocol, but a resource or bound
receiver must not gain component lifecycle operations merely because both use opaque handles.

## Proposed runtime shape

The crate should own opaque identities, not host payloads:

```rust,ignore
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostObjectHandle(u64);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct HostTypeId(String);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HostObjectClass {
    ImmutableValue,
    Resource,
    BoundReceiver,
}

pub enum Value {
    // Existing script values...
    ComponentObject {
        id: ComponentHandle,
        component_type: String,
    },
    HostObject {
        handle: HostObjectHandle,
        host_type: HostTypeId,
        class: HostObjectClass,
    },
}
```

The exact public representation can be smaller—for example, class and type metadata can live in
the session registry—but the runtime must be able to validate class and type without asking the
script to supply trusted strings.

Keep `ComponentHandle` during the first migration because component materialization and topology
already depend on it. A later cleanup may unify both opaque handle representations internally while
retaining component-versus-non-component class checks at the API boundary.

Do not add one enum variant per host domain:

```rust,ignore
// Avoid an indefinitely growing language-core enum:
Value::TransformWorld(...)
Value::AudioData(...)
Value::ImageData(...)
Value::StyleLayout(...)
```

Those are host type registrations, not universal language primitives.

## Host type catalog

Add a host-object type specification alongside component and free/namespace API specifications:

```rust,ignore
HostObjectSpec::new("TransformWorld", HostObjectClass::BoundReceiver)
    .method("trs", ValueSignature::overloads([
        signature([], host("TransformTrs")),
        signature([host("TransformTrs")], ValueType::Null),
    ]))

HostObjectSpec::new("ImageData", HostObjectClass::Resource)
    .method("width", signature([], ValueType::Number))
    .method("height", signature([], ValueType::Number))
```

Component specifications need declared host-backed members:

```rust,ignore
ComponentSpec::new("Transform")
    .host_member("world", host("TransformWorld"))
```

Host-object specifications may also expose members returning other bound receivers:

```rust,ignore
HostObjectSpec::new("AudioData", HostObjectClass::Resource)
    .host_member("samples", host("AudioSamples"))
```

The catalog owns script-visible names, signatures, member/method distinction, and capabilities.
The host owns implementation and native storage.

This makes `transform.world()` unambiguously invalid when `world` is registered as a member but
not a method. It also permits `let world = transform.world` because member access returns a normal
first-class `Value::HostObject`.

## Receiver protocol

Use a typed receiver descriptor rather than passing a synthetic component type string:

```rust,ignore
pub enum HostReceiver {
    Component {
        handle: ComponentHandle,
        component_type: String,
    },
    Object {
        handle: HostObjectHandle,
        host_type: HostTypeId,
        class: HostObjectClass,
    },
}
```

The core operations are:

```rust,ignore
HostRequest::GetMember {
    receiver: HostReceiver,
    member_id: HostMemberId,
}

HostRequest::InvokeMethod {
    receiver: HostReceiver,
    method_id: HostMethodId,
    args: Vec<TransportValue>,
}
```

Operation IDs should come from the validated catalog. Human-readable names remain available for
diagnostics, but the host should not infer semantics from a fake `component_type` string.

The host returns script-owned data, a component, or another host object:

```rust,ignore
HostResponse::Transport(TransportValue)
HostResponse::Component { ... }
HostResponse::HostObject {
    handle: HostObjectHandle,
    host_type: HostTypeId,
    class: HostObjectClass,
}
```

## Evaluation semantics

Dot access is resolved by receiver category:

1. Script table/object: read the script-owned field.
2. Built-in namespace: resolve the registered built-in member.
3. Component or host object: look up a declared host member and dispatch `GetMember`.
4. Otherwise: report that the value has no readable member.

Method calls similarly dispatch on the evaluated receiver:

1. Script function/method behavior, when the object model supports it.
2. Built-in receiver.
3. Component or host object through `InvokeMethod`.
4. Otherwise: report that the value is not a method receiver.

Member chaining needs no special parser rule:

```mms
component.interface.subinterface.method()
```

Each dot evaluates the previous receiver and may return another host object. The terminal call uses
the type registered for the final receiver.

## Storage, lifetime, and stale handles

Host objects may appear anywhere a normal `Value` can appear:

- lexical bindings;
- arrays and tables;
- heap objects;
- module exports;
- closure captures;
- arguments and results.

Every handle must be scoped to one MMS session. `HostContext` should allocate or adopt host-object
handles just as it currently owns component and callback handles. Requests containing a foreign
session handle fail before host dispatch.

Initial lifetime policy can retain host-object entries until session teardown. This is predictable
and sufficient for short-lived sessions, though it may retain large resources longer than needed.
A later tracing/release protocol can reclaim host objects when no MMS value references them.

Recommended ownership behavior by class:

| Class | Host storage | Effect of owner deletion |
| --- | --- | --- |
| Immutable value | Independent immutable entry or shared immutable payload | Remains valid |
| Resource | Resource arena entry/strong shared handle | Remains valid until released/session ends |
| Bound receiver | Owner handle plus interface, without a borrowed Rust reference | Fails as stale if owner is gone |
| Component | Host/ECS generation-checked handle | Fails as stale if component is gone |

A bound receiver should not keep an ECS component alive merely because the receiver is stored in
MMS. It retains the owner's identity, not ownership of the ECS node.

## Transport boundary

Add a corresponding opaque transport form so host objects can be passed as API arguments without
exposing MMS heap objects:

```rust,ignore
TransportValue::HostObject(HostObjectHandle)
```

The session already owns the handle-to-type/class metadata, so an untrusted script cannot forge a
different host type for the same handle. Hosts returning a handle must allocate or adopt it through
`HostContext` before the evaluator accepts it.

Large resource payloads do not cross this boundary on every method call. Operations use their
handles; explicit APIs may return small copied snapshots or future typed-buffer views.

Callbacks remain `CallbackHandle`s. Components remain component handles during the initial
migration because they have topology-specific operations.

## Image and audio examples

### Image data versus texture component

```mms
let image = ImageData.load("cat.png") // Host resource
let size = image.size()               // Script-owned small value

Texture.from_image(image) {}          // Component/resource configuration using it
```

`image` should not become a `Texture` component merely to live in the MMS heap. Nor should decoded
pixels become a general number array unless the author explicitly requests a copied array.

### Audio data versus playback component

```mms
let samples = AudioData.load("meow.wav")
let duration = samples.duration()

AudioClip.from_data(samples) {
    loop(true)
}
```

The audio data is a reusable resource. `AudioClip` is a component with playback lifecycle and scene
behavior. A future `audio.playback` subreceiver may bind playback-control methods to a component or
clip instance without becoming a new component.

## Transform migration example

Current Mittens plumbing conceptually does this:

```text
Value::TransformWorld { component_id }
    -> InvokeComponentMethod(component_type = "TransformWorld")
```

After the generic receiver protocol:

```text
Value::ComponentObject(transform)
    -- GetMember("world") -->
Value::HostObject(TransformWorld bound receiver)
    -- InvokeMethod("trs") -->
Value::HostObject(TransformTrs immutable value)
```

The Mittens host arena entry might contain:

```rust,ignore
enum MittensHostObject {
    TransformWorld {
        transform: ComponentId,
    },
    TransformTrs(TransformTrs),
}
```

That enum belongs to Mittens. The language crate sees only opaque handles, registered host types,
and semantic classes.

It is also reasonable for the Mittens adapter to keep `TransformTrs` as a temporary direct runtime
variant while migrating `.world` first. The end state should avoid requiring every application
embedding MMS to compile engine-specific value variants into the crate.

## Error model

Distinguish these failures:

- unknown member on a known host type;
- unknown method on a known host type;
- member used as a method, such as `transform.world()`;
- wrong argument type or arity before dispatch;
- foreign-session or unadopted handle;
- stale host resource;
- stale bound-receiver owner;
- host operation unavailable despite catalog registration;
- host operation failure after valid dispatch;
- unsupported conversion across the transport boundary.

Errors should name the registered receiver type and operation:

```text
Transform.world.trs(): stale Transform component receiver
ImageData.pixels(): resource is no longer available
Transform.world: member is not callable
```

Do not silently return `null` for unsupported or stale host operations.

## Equality, hashing, and serialization

The first implementation should keep rules conservative:

- components, resources, and bound receivers compare by opaque identity only if MMS exposes
  equality for them;
- immutable host values have no general equality until their `HostObjectSpec` declares one;
- host values are hashable only when their registered type explicitly permits it;
- no host object serializes into authored MMS by default;
- REPL/debug formatting uses a safe type/handle summary and never dumps image/audio payloads.

Script-owned records remain the preferred representation when structural equality and authored
serialization are important.

## Security and capability checks

- Catalog registration determines which members and methods exist.
- Each member/method can require a host capability.
- The runtime validates the receiver's session, class, and registered type before dispatch.
- The host validates the native owner/resource generation again before use.
- Scripts cannot construct raw handles, change a handle's registered type, or ask for an arbitrary
  operation string.
- Returning a bound receiver does not grant component lifecycle capabilities.

## Implementation sequence

### Phase 1: host-object catalog and handles

- Add `HostObjectHandle`, `HostObjectClass`, and a host-object type specification.
- Extend `ValueType` with a named host-object type rather than one enum case per domain.
- Let component and host-object specs declare typed host-backed members.
- Add catalog conflict, alias, capability, and signature tests.

### Phase 2: generic member and method dispatch

- Add `HostReceiver`, `GetMember`, and generic `InvokeMethod` requests.
- Add `HostResponse::HostObject` and opaque host-object transport.
- Teach evaluation to resolve host members and chain returned receivers.
- Validate receiver class/type and arguments before host dispatch.
- Make host objects storable in arrays, tables, objects, modules, and closures.

### Phase 3: lifetime and diagnostics

- Extend `HostContext` ownership checks to host-object handles.
- Initially retain adopted objects for the session lifetime.
- Add stale owner/resource and foreign-session tests.
- Add REPL formatting which shows type and opaque identity only.

### Phase 4: migrate Mittens transform receivers

- Register `Transform.world` as a host-backed member returning `TransformWorld`.
- Register `TransformWorld.trs` with zero/one-argument overloads.
- Replace `Value::TransformWorld` and the synthetic `"TransformWorld"` component-type dispatch.
- Preserve the current world-TRS snapshot behavior and focused slide-deck regression.
- Decide separately whether temporary `TransformTrs` becomes a host immutable value or a typed MMS
  struct/tuple.

### Phase 5: prove one non-component resource

- Implement a small resource fixture in the crate example host before committing to image/audio
  engine APIs.
- Return a resource, store it in an MMS heap object, call a method, and pass it back through a host
  API.
- Prove that it has no component lifecycle privileges and that a foreign handle is rejected.

## Acceptance criteria

- A host can register a non-component receiver type with members and methods.
- A component member can return a bound receiver without claiming it is a component.
- A bound receiver can be stored and called later against the same generation-checked owner.
- Removing its owner produces a stale-receiver error rather than retargeting.
- A non-component resource can live in the MMS heap without becoming a component or copying its
  full payload into generic arrays.
- Component-only attach/detach/query operations reject non-component host objects.
- Chained host receivers require no domain-specific parser behavior.
- The crate contains no Mittens-specific transform, audio, image, style, or ECS types.
- Mittens can remove the synthetic `"TransformWorld"` component-type dispatch after adopting the
  protocol.

## Open questions

- Should immutable host values use the same handle arena as identity resources, or should the
  runtime gain an inline opaque immutable-value representation?
- When should the runtime add tracing/release instead of retaining host objects for the full
  session?
- Should bound receivers have stable identity per `(owner, interface)`, or may repeated member
  reads create observationally equivalent handles?
- Does a future typed-buffer view count as an immutable value, resource, or its own borrowed-view
  class with stricter lifetime rules?
- Should components eventually become one `HostObjectClass::Component`, or should their separate
  `ComponentHandle` remain permanently explicit?
- How should declared immutable-host-value equality integrate with `PartialEq` on the Rust runtime
  `Value` without calling into the host?

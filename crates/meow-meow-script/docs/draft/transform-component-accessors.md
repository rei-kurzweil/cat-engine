# Transform component accessors

Status: in progress; engine-side values/reads and opaque local MMS `trs()` pass-through exist;
world mutation and the receiver-bound `world` table remain.

Related:

- [VTuber slide-deck detached world-TRS implementation](../../../../docs/task/vtuber-slidedeck-detached-world-trs.md)
- [Mittens engine transform accessor API](../../../../docs/draft/transform-component-accessors-engine-api.md)
- [Type-expression grammar and AST](type-expressions.md)
- [Compound MMS types](compound-types.md)
- [MMS type-system epic](type-system-epic.md)
- [VTuber slide-deck XR placement and controls](../../../../docs/task/vtuber-slidedeck-xr-placement-and-controls.md)

## Goal

Make reading and copying transform state feel like ordinary MMS code. Authors should not need a
special-purpose operation named after the feature that happens to need the transform.

The target surface is small, granular, and composable:

```mms
let translation = some_transform.translation()
let rotation = some_transform.rotation()
let scale = some_transform.scale()
let trs = some_transform.trs()

let world_translation = some_transform.world.translation()
let world_rotation = some_transform.world.rotation()
let world_scale = some_transform.world.scale()
let world_trs = some_transform.world.trs()
```

When type annotations and tuple type expressions exist, the intended shapes are:

```mms
let translation: [f32; 3] = some_transform.translation()
let rotation: [f32; 4] = some_transform.rotation()
let scale: [f32; 3] = some_transform.scale()
let trs: ([f32; 3], [f32; 4], [f32; 3]) = some_transform.trs()
```

Rotation is a normalized quaternion in engine `xyzw` order. Returning a quaternion avoids a lossy
Euler conversion when one component's pose is copied to another.

## No transform cloning

This API does not need to clone `TransformComponent` objects. A scene owns the live source and
target transforms; accessors copy only plain data values between them:

```mms
let pose = presentation_anchor.world.trs()
slide_root.world.trs(pose)
```

Here `pose` is a copied TRS value. It is not registered in the ECS, has no component identity, and
does not retain a reference or relationship to `presentation_anchor`. `slide_root` is an existing,
separately authored live component. The setter emits a mutation for that existing target.

Avoid adding an accessor that creates a detached, registered transform clone merely to expose
world-space data. That would turn a read into allocation and lifecycle work, make ownership less
obvious, and risk leaking temporary scene components. If a general component-cloning feature is
ever desired, it should remain separate from transform access and coordinate-space conversion.

## Method syntax

Use methods for the terminal accessors rather than reading or assigning transform fields:

```mms
some_transform.rotation()
```

This matches the existing `translation()` getter and makes evaluation explicit. `world` is the
intentional namespace/table selection in the middle of the chain; `translation`, `rotation`,
`scale`, and `trs` remain methods.

For each live transform accessor, zero arguments select the getter and one value argument selects
the setter:

```mms
let rotation = some_transform.rotation()
some_transform.rotation(replacement_rotation)
```

Constructor/builder expressions remain distinct:

```mms
T.rotation(0.0, 1.57, 0.0) {}
```

`T` and `Transform` are true aliases. Both names expose the same constructors, builder methods,
and static namespaces, so authors can choose the desired verbosity in either scene declarations
or ordinary expressions:

```mms
let authored = T.position(1.0, 2.0, 3.0) {}
let verbose_authored = Transform.position(1.0, 2.0, 3.0) {}
```

The parser/evaluator should distinguish component construction from an ordinary static-method
expression by the expression form and whether a component body is present, not by which alias was
used. Documentation may use the long name where clarity helps and the short name where compactness
helps; neither spelling is more canonical.

## Local and world space

The unqualified methods should consistently return the component's authored/local TRS:

```mms
transform.translation()
transform.rotation()
transform.scale()
transform.trs()
```

Detached placement also needs the propagated world pose. Each live transform component exposes a
receiver-bound, non-callable `world` method table:

```mms
transform.world.translation()
transform.world.rotation()
transform.world.scale()
transform.world.trs()
```

This reads more naturally than growing four parallel `world_translation()`, `world_rotation()`,
`world_scale()`, and `world_trs()` methods on every transform component. The original transform is
already bound to the table, so every terminal accessor keeps its zero-argument getter and
one-argument setter contract.

`world` has no parentheses. Accessing it does not construct, clone, register, or copy a
`TransformComponent`:

```mms
let rotation = transform.world.rotation()
transform.world.rotation(replacement_rotation)
```

The first expression copies out a quaternion value. The second emits a world-space mutation for
the already-bound `transform`. The table itself is not a first-class transform value and should
not be storable independently of its receiver.

Callable spellings are deliberately excluded:

```mms
// Not this API: calling world suggests construction or a copied transform.
transform.world().rotation()
T.world(transform).rotation()
```

Local access remains concise on the component itself:

```mms
transform.trs()
```

This is more granular than a feature-specific snapshot operation and lets authors inspect or copy
only the channels they need.

Open question: should `transform.world.scale()` expose the decomposed scale of an
arbitrary matrix when ancestors contain non-uniform scale and rotation? If decomposition can be
ambiguous or sheared, `transform.world.trs()` must either return a typed error or document
its decomposition rules. World translation and rotation may still be independently useful.

## TRS value representation

The desired typed shape is a tuple:

```text
([f32; 3], [f32; 4], [f32; 3])
```

The current type-expression draft treats parentheses only as grouping and does not define tuple
types. Supporting the annotation above therefore requires both tuple type syntax and a tuple value
representation/destructuring story. This should be added deliberately rather than pretending a
heterogeneous TRS tuple is a homogeneous fixed array.

The first implementation does not need to expose those channels inside MMS. It can return an
opaque, first-class copied `TransformTrs` runtime value that passes directly between getter and
setter:

```mms
let pose = some_transform.world.trs()
target.world.trs(pose)
```

The engine still decomposes a world matrix to create this DTO; opaque means only that MMS cannot
yet split it into channels. Numeric indexing and named channel reads are deferred until detached
snapshot placement works. At that point, align the inspection API with the tuple/type-system work
rather than committing early to a nested array or hash table representation. The implementation
tracker records the deferred alternatives.

## Applying a copied TRS

The existing live method is:

```mms
target.update_transform(translation, rotation_euler, scale)
```

It accepts Euler rotation, so it cannot consume the quaternion returned by `rotation()` or
`trs()` without conversion. The new accessors use the same zero-or-one argument contract for
local reads and writes:

```mms
let rotation = target.rotation() // getter
target.rotation(rotation)        // setter

let pose = target.trs() // getter
target.trs(pose)        // setter
```

World-space access uses those same terminal method contracts through the bound `world` table:

```mms
let pose = source.world.trs() // zero arguments: getter
target.world.trs(pose)        // one argument: setter
```

The granular world setters follow the same rule:

```mms
let world_rotation: [f32; 4] = some_transform.world.rotation()
world_rotation = something_else
some_transform.world.rotation(world_rotation)

let translation = source.world.translation()
target.world.translation(translation)

let scale = source.world.scale()
target.world.scale(scale)
```

The quaternion stored in `world_rotation` is copied by value. Reassigning it does not affect
`some_transform`; the final one-argument call emits the mutation against the receiver already
bound to the `world` table.

`target.trs(pose)` writes local space. `target.world.trs(pose)` converts the world-space value into
the target's parent space and writes the resulting local TRS. Neither form retains a relationship
to `source`; it copies a value at evaluation time. That is the desired snapshot behavior for
detached slide placement.

## VTuber slide-deck target usage

Once world getters and a world setter exist, the deck can be a separate root:

```mms
let deck_pose = presentation_anchor.world.trs()
slide_root.world.trs(deck_pose)
```

An authored offset still needs to be composed with the sampled pose. Prefer general TRS
composition helpers over a deck-specific method. Candidate direction:

```mms
let deck_pose = presentation_anchor.world.trs()
let offset = trs(
    [-0.95, 0.15, -1.25],
    [0.0, 1.0, 0.0, 0.0],
    [0.055, 0.055, 1.0],
)
slide_root.world.trs(deck_pose * offset)
```

The exact constructor and multiplication syntax remain open. The essential contract is that the
result is a value, not a persistent parent/follower binding.

## Error contract

Accessors should return a useful runtime error when:

- the receiver is not a live `TransformComponent`;
- an accessor receives more than one argument;
- a propagated world transform is not available yet;
- world-matrix decomposition cannot produce the promised TRS representation;
- a setter receives arrays of the wrong size or non-finite numbers.

Do not silently return identity TRS for unavailable live state; that would place detached content
at the world origin and hide lifecycle bugs.

## Initial implementation slice

1. Add local `rotation()`, `scale()`, and `trs()` alongside existing `translation()`.
2. Add focused live-component method tests for values, arity errors, quaternion order, and copy
   semantics.
3. Decide and document the interim untyped `trs()` aggregate.
4. Add local one-argument setters for `translation(value)`, `rotation(value)`, `scale(value)`, and
   `trs(value)` alongside their zero-argument getters.
5. Add the receiver-bound, non-callable `world` method table with zero-or-one argument
   `translation` and `rotation` accessors using propagated transform state.
6. Specify decomposition before adding `scale` and `trs` to the world method table.
7. Add tests proving that accessing `world` neither creates nor registers a component, and that
   calling `world(...)` is rejected.
8. Add general TRS composition, then detach the VTuber slide deck and snapshot its presentation
   anchor on each successful slide change.

## Open questions

- Should `trs()` return a tuple immediately, requiring tuple values before tuple annotations?
- Is `{ translation, rotation, scale }` a better permanent value than positional tuple fields?
- Should getter quaternions always be normalized on read?
- Should `trs(value)` bypass `Transition` children for an instantaneous copy, or use ordinary
  transform mutation semantics?
- Should `transform.world.trs()` return `null`, a result-like value, or a runtime error before
  propagation?
- Is the XR presentation source the locomotion root, tracked head pose, or an explicitly authored
  presentation anchor?

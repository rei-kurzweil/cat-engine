# MMS Type Registry and Method Dispatch

> **Status: design draft.** None of the static type system described here is
> implemented yet. The evaluator currently stores every ordinary number as
> `Value::Number(f64)`, and its built-ins and receiver dispatch are partly
> hardcoded.
>
> This document is the current direction for MMS types, numeric defaults,
> receiver methods, and strict checking. It supersedes the conflicting syntax
> and numeric rules in [type-system.md](type-system.md),
> [numeric-types.md](numeric-types.md), and [coercion.md](coercion.md).

## Goals

MMS should have one description of callable types and methods that can answer
the same question in every execution mode:

> Given the type of this receiver and these arguments, which method is this,
> and what type does it return?

That description must serve:

1. the dynamic evaluator, using the receiver's runtime type;
2. the static checker, using inferred or declared types;
3. strict mode and transpilers, where every call must resolve before execution;
4. the host catalog, including component methods;
5. future methods authored on named MMS structs.

Simple scripts remain concise. `let` infers a type, while an explicit type
replaces `let` when the author needs to control representation.

## Source type surface

### Primitive types

MMS uses lowercase, fixed-width scalar names:

| Category | Types | Default literal type |
|---|---|---|
| Signed integers | `i8`, `i16`, `i32`, `i64` | `i64` |
| Unsigned integers | `u8`, `u16`, `u32`, `u64` | none |
| Floating point | `f32`, `f64` | `f64` |
| Other primitives | `bool`, `str`, `null`, `any` | literal-dependent |

There is no `usize`. Collection lengths have the stable source-level type
`u64`, independent of the CPU or transpilation target.

`null` is both the type and value used when a function has no meaningful
result. `any` is the gradual-typing escape hatch. A value of type `any` is
dynamically checked when it is used.

Collection and function types use:

```mms
[f32]                         // array of f32
[[u32]]                       // array of arrays
fn(f32, f32) -> f32           // function value type
```

Named struct and component types are nominal types registered alongside these
built-ins. A named struct may remain table-backed at runtime, but it must retain
its nominal type identity if it participates in method dispatch.

### Bindings

`let` means "infer the binding's type":

```mms
let distance = 1.0            // f64
let count = 1                 // i64
let enabled = true            // bool
let samples = [1.0, 2.0]      // [f64]
```

An explicit type is a postfix annotation on a `let` binding:

```mms
let shader_distance: f32 = 1.0
let precise_distance: f64 = 1.0
let count: u64 = 1
let samples: [f32] = [1.0, 2.0]
```

`let value: Type = expression` is the single typed-binding form. Named types
are resolved after parsing in the type namespace.

Reassignment keeps the existing form:

```mms
let opacity: f32 = 1.0
opacity = 0.5
```

The right-hand side of a reassignment must be assignable to the binding's
established type.

### Functions

Typed parameters and returns use the same postfix `name: Type` form:

```mms
let lerp = fn(a: f32, b: f32, t: f32): f32 {
    return a + (b - a) * t
}
```

Annotations may be omitted:

```mms
fn twice(x) {
    return x + x
}
```

The checker may infer omitted parameter and return types from constraints in
the function body and its call sites. In normal evaluation, anything that
remains unresolved becomes `any` and is checked dynamically. In strict mode,
every parameter and result must resolve to a concrete type; unresolved `any`
is an error.

This rule also applies to function literals. Strict mode does not require
redundant annotations when inference produces one unambiguous complete
signature.

## Numeric literals and conversions

### Literal typing

An unconstrained integer literal has type `i64`. An unconstrained literal with
a decimal point or exponent has type `f64`:

```mms
let frames = 120               // i64
let seconds = 2.0              // f64
let scale = 1e-3               // f64
```

A literal is initially an exact compile-time value. An explicit destination
may give it any numeric type in which it is valid:

```mms
let channel: i8 = 7
let count: u64 = 100
let opacity: f32 = 1.0
let distance: f64 = 1
```

This is contextual literal typing, not an implicit conversion from an already
created `i64` or `f64`. An integer literal must fit the destination integer
range. A negative literal cannot initialize an unsigned type. A floating
literal is rounded once to the destination IEEE-754 representation.

Invalid constant conversions are compile-time errors:

```mms
let too_large: u8 = 256             // error: outside u8
let negative: u64 = -1              // error: outside u64
```

### Established values

Once an expression has a concrete numeric type, changing that representation
is explicit—even when the conversion would be lossless:

```mms
let shader_value: f32 = 1.0
let same = shader_value        // f32

let precise: f64 = f64(shader_value)
let count: u64 = u64(1)

let cpu_value = 1.0            // f64
let gpu_value: f32 = f32(cpu_value)
```

This rule keeps APIs and generated shader/host boundaries visible. It also
avoids making overload selection depend on a widening graph.

Numeric type names act as conversion functions:

- integer to integer succeeds only when the value is representable;
- float to integer rejects NaN, infinity, and out-of-range values, then
  truncates toward zero;
- integer to float uses the target IEEE-754 rounding behavior;
- `f32` to `f64` is exact;
- `f64` to `f32` rounds to `f32`; overflow produces the corresponding infinity.

A failed nonconstant conversion is a runtime error in dynamic evaluation.
Where all inputs are constant, the checker reports it before evaluation.

Arithmetic requires equal established operand types. Authors convert one side
explicitly when types differ:

```mms
let x: f32 = 1.0
let y: f64 = 2.0
let z = f64(x) + y              // f64
```

Contextual typing may still give literals the other operand's type:

```mms
let x: f32 = 1.0
let y = x + 2.0                 // f32; 2.0 is context-typed as f32
```

Unit-bearing literals retain their existing unit identity. Their complete
generic type and conversion model remains with the unit-number design; absent
an explicit context, their numeric representation follows the same `i64` and
`f64` literal defaults described here.

## The shared type and method registry

### Canonical entries

Receiver methods are declared once:

```text
method [T].length() -> u64 = intrinsic(array_length)
method str.length() -> u64 = intrinsic(string_scalar_length)

method f32.sin() -> f32 = intrinsic(f32_sin)
method f64.sin() -> f64 = intrinsic(f64_sin)
```

`[T].length()` is a generic receiver signature. Matching `[f32]` binds `T` to
`f32`; matching `[Widget]` binds it to `Widget`. Its result does not depend on
`T`.

`str.length()` counts Unicode scalar values, preserving the current evaluator's
`chars().count()` behavior. Both string and array lengths return `u64`.

Numeric methods do not accept a differently typed numeric receiver through
implicit conversion:

```mms
let angle: f32 = 1.0
let a = angle.sin()             // f32

let precise = 1.0
let b = precise.sin()           // f64

let count = 1
let c = count.sin()             // error: i64 has no sin()
let d = f64(count).sin()        // f64
```

### Conceptual data model

The exact Rust layout may evolve during implementation, but the registry needs
the following information as one coherent model:

```rust
enum Type {
    Any,
    Null,
    Bool,
    Str,
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    F32, F64,
    Array(Box<Type>),
    Function {
        parameters: Vec<Type>,
        result: Box<Type>,
    },
    Named(TypeId),
    Component(TypeId),
}

enum TypePattern {
    Exact(Type),
    Array(Box<TypePattern>),
    Variable(TypeVariableId),
}

struct MethodSignature {
    type_parameters: Vec<TypeVariableId>,
    receiver: TypePattern,
    parameters: Vec<TypePattern>,
    result: TypePattern,
}

struct MethodSpec {
    name: String,
    signature: MethodSignature,
    target: DispatchTarget,
}

enum DispatchTarget {
    Intrinsic(IntrinsicId),
    Host(HostOperationId),
    Script(FunctionId),
}
```

Production types should use stable identifiers rather than dispatching on
display strings. Display names and aliases belong to type metadata.

This model evolves the existing `ValueType`, `ValueSignature`,
`ComponentSpec.methods`, and host API catalog. It must not become a second,
unrelated catalog that can disagree with host-call validation.

Language-provided primitive and collection entries are installed by the
standard runtime. Hosts register component types and their methods through the
same signature model. A future struct declaration registers its nominal type
and authored methods with `DispatchTarget::Script`; the source syntax for
declaring struct methods remains deferred to the struct-method design.

Anonymous table calls retain their current data behavior: a function-valued
field is invoked with the table as implicit `self`. Named structs use their
nominal registry type instead. This prevents a table field from accidentally
masquerading as a registered method on an unrelated nominal type.

### Registration invariants

Registry construction rejects:

- duplicate canonical type names or aliases;
- duplicate methods with the same receiver pattern, name, and parameter
  pattern;
- result or parameter types that are not registered;
- intrinsic or host targets whose implementation identifier is unknown;
- generic variables used outside their declaration.

Multiple methods may share a name when their receiver patterns differ, as with
`f32.sin()` and `f64.sin()`.

### Resolution order

For `receiver.method(arguments)`:

1. Determine the receiver type from static information or the runtime value.
2. Collect methods with the requested name.
3. Prefer an exact nominal or primitive receiver match.
4. Then match structural generic patterns such as `[T]`, binding their type
   variables.
5. Validate argument count and exact established argument types, allowing
   contextual typing for literals.
6. Substitute bound variables into the result type.
7. Reject no-match and ambiguous-match outcomes.

There is no arbitrary "first registered wins" behavior. Duplicate or ambiguous
registrations are catalog construction errors when detectable and call errors
otherwise.

Diagnostics include the receiver type, attempted argument types, and available
same-name candidates. Unknown names should also offer edit-distance
suggestions, consistent with current component and namespace diagnostics.

## Compatibility call forms

Existing global and namespace calls remain permanent aliases:

```mms
len(values)
values.length()

Math.sin(angle)
angle.sin()
```

Aliases point to the canonical registry entry or intrinsic identifier; they do
not duplicate implementations. Consequently, both spellings have identical:

- accepted types and literal-context behavior;
- result types;
- runtime errors;
- checker diagnostics;
- transpiler lowering.

`len` is overloaded for the same receiver types that provide `length()`.
`Math.sin` is overloaded for `f32` and `f64`.

## Consumers

### Dynamic evaluator

The runtime value representation must preserve numeric width and signedness so
that dispatch can distinguish `f32.sin()` from `f64.sin()` and transport values
to shaders or Vulkan without first erasing their type.

The evaluator:

1. obtains the receiver's runtime `Type`;
2. resolves the method through the registry;
3. validates runtime arguments against the resolved signature;
4. invokes its intrinsic, host, or script target;
5. verifies that the result agrees with the registered result type.

Component handles already carry a component type name for dispatch. The final
model should replace string identity with, or resolve it to, the registry's
stable component `TypeId`.

### Static checker

The checker uses the same registry and resolution algorithm with inferred
types. A call whose known receiver or arguments cannot match is an error before
evaluation. A call involving `any` is retained as a dynamic call in normal
mode.

Inference is constraint-based within the checked module. Literals, explicit
bindings, operators, calls, returns, and registry signatures contribute
constraints. If there is one complete solution, omitted function annotations
are unnecessary. Ambiguous or incomplete values become `any` in normal mode.

### Strict mode and transpilation

Strict mode accepts a program only when:

- every binding, function parameter, and result has a resolved non-`any` type;
- every method and function call resolves to one canonical signature;
- every conversion is explicit except contextual literal typing;
- every intrinsic has a lowering for the selected transpilation target.

This makes strictness a completeness requirement, not an annotation-count
requirement. A fully inferred function is valid.

Transpilers lower `IntrinsicId`, `HostOperationId`, or a resolved script
function rather than rediscovering semantics from a method's source spelling.
For example, `Math.sin(x)` and `x.sin()` reach the same `f32_sin` or `f64_sin`
intrinsic before target lowering.

## Implementation direction

The design can land incrementally without making runtime method syntax wait for
the entire static checker:

1. Replace hardcoded primitive built-in dispatch with canonical registry
   entries while retaining today's `Value::Number(f64)` as a temporary `f64`.
2. Route `len`, `Math` aliases, and receiver calls through those entries.
3. Expand runtime numeric values and type metadata to fixed-width types.
4. Add typed binding/function grammar and contextual literal checking.
5. Add module inference and normal-mode diagnostics.
6. Make strict checking a required precondition for transpilation.

At every stage, runtime validation remains the fallback for values that are
dynamic or originate across an unchecked host boundary.

## Deferred decisions

This draft deliberately does not choose:

- syntax for declaring methods inside user-authored structs;
- arbitrary union, trait, or interface types;
- generic user-authored functions;
- vector/matrix and shader resource types;
- the final type model for unit-bearing dimensions;
- target-specific ABI layouts.

The registry must be able to add these without changing the resolution contract
defined above.

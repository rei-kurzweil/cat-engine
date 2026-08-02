# Compound MMS types

Status: canonical draft.

## Sequences and fixed arrays

`[T]` is a variable-length, slice-like sequence. It does not promise owned
collection semantics or a static layout. `[T; N]` is a fixed-length array;
`N` must be statically evaluable and participates in type identity and layout.
`Vec<T>` and `List<T>` are reserved for a later owned-collection design and
must not alias `[T]`.

## Records and named structs

`{ field: Type }` is an explicit structural anonymous-record type. Anonymous
table and named allocation values both use equals-style fields:

```mms
let anonymous = {
    strength = 100
}

let cup = coffee {
    strength = 100
    additives = ["milk"]
}
```

Commas are optional in values; canonical formatting emits one field per line
without commas. Named structs are nominal. Same-field compatibility is
available only through an explicit anonymous record or intersection type, not
by accidental equivalence between declarations.

## Nullable, intersections, and unions

`T?` admits `null`. `A & B` requires both constraints; this is the primary way
to compose structural record requirements. `A | B` admits either member.
Precedence is postfix, then intersection, then union.

Dependency: table runtime values, nominal declaration IDs, and the type
expression parser. Smoke test: validate a fixed pair, a variable sequence, a
record intersection, a nullable, and a union at annotated boundaries. Exit
gate: validation is recursive, length-aware, cycle-safe, and reports the
failing path such as `cup.additives[1]`.


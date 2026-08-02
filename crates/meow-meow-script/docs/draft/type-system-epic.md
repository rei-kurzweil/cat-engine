# MMS type-system epic

Status: canonical draft; implementation has not started.

MMS remains gradually typed: annotations are optional, and unannotated code
keeps today's dynamic behavior. This epic owns the implementation order. The
focused drafts beside it own syntax and boundary details.

## Source contract

```mms
let opacity: f32 = 1.0

let drink = fn(c: coffee): bool {
    return true
}

export struct coffee {
    strength: f32
    additives: [str]
}

let cup = coffee {
    strength = 100
    additives = ["milk", "sugar"]
}
```

Bindings, parameters, returns, and struct fields consistently use
`name: Type`. Anonymous-table and named-struct values use `field = value`.
Commas are optional; canonical formatting emits one field per line without
commas.

## Vertical slices

1. **Type expressions.** Add a spanned `TypeExpression` AST, tokenizer and
   parser support, precedence, and parse/unparse round trips. Dependency:
   none. Smoke test: round-trip every form in
   [type-expressions.md](type-expressions.md). Exit gate: malformed types have
   source-local diagnostics and canonical output reparses to the same AST.
2. **Postfix annotations.** Add annotations to bindings, function parameters
   and returns, and struct fields. Dependency: slice 1. Smoke test: parse and
   unparse the source contract above. Exit gate: all four annotation sites
   retain spans and optional annotations without changing untyped programs.
3. **Nominal structs and modules.** Add declarations, imports, allocations,
   and declaration identity retained by instances. Dependency: slice 2 plus
   the existing module/table runtime. Smoke test: import `coffee`, allocate it,
   and observe that a same-shaped declaration is not interchangeable. Exit
   gate: exported type bindings resolve to one declaration identity across a
   session and reload behavior is specified.
4. **Runtime validation.** Validate annotated binding/call/return/field
   boundaries, including fixed arrays and compound types. Dependency: slice 3
   and a shared type registry. Smoke test: `[f32; 2]` accepts two finite numeric
   values and rejects the wrong length with the annotation span. Exit gate:
   every initial type form has deterministic boundary validation.
5. **Static analysis.** Add registry-backed resolution, diagnostics,
   inference, and strict mode. Dependency: slice 4 and the canonical runtime
   signature catalog. Smoke test: infer a local value, diagnose an invalid
   imported nominal value, and reject unresolved `Any` in strict mode. Exit
   gate: the checker and runtime validator resolve names through the same
   registry and the parity corpus runs in gradual and strict modes.

The syntax details are in [type-expressions.md](type-expressions.md),
[typed-declarations-and-functions.md](typed-declarations-and-functions.md),
[compound-types.md](compound-types.md), and
[checker-and-registry.md](checker-and-registry.md).


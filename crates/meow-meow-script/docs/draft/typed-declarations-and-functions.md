# Typed declarations, functions, and module-visible types

Status: canonical draft.

Annotations are optional and postfix:

```mms
let opacity: f32 = 1.0
let drink = fn(c: coffee): bool { return true }

export struct coffee {
    strength: f32
    additives: [str]
}
```

The same `name: Type` rule applies to a binding, parameter, function return
site, and struct field. A return annotation follows the parameter list because
it annotates the function result binding. Function values use the matching
type expression `fn(coffee): bool`.

An exported struct introduces both a value-space allocation constructor and a
module-visible type binding. Imports retain the identity of the exported
declaration, not merely its spelling. Instances retain that declaration ID.
Two declarations with equal fields remain different nominal types.

Dependency: `TypeExpression`, table values, module export/import identity, and
named struct declarations. Smoke test: export/import `coffee`, annotate a
parameter, allocate a value, and round-trip the module. Exit gate: all
annotation spans survive parsing and imported type names resolve to the exact
exported declaration in both runtime validation and static checking.


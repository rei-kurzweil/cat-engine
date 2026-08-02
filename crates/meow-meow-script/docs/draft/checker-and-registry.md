# Checker and registry integration

Status: canonical draft.

The runtime validator and static checker share one registry for primitive
types, exported nominal declarations, host signatures, aliases, and module
resolution. A separate checker-only name map would allow runtime and static
semantics to drift and is not acceptable.

Resolution produces stable type/declaration IDs plus source-facing names.
Imported nominal IDs remain stable for the session. Anonymous records,
intersections, unions, fixed arrays, and function signatures are interned or
otherwise compared canonically. Diagnostics retain definition, import, and
use spans and show a value path for compound failures.

Gradual mode infers what it can and permits dynamic/unknown values at
unannotated sites. Annotated boundaries always validate. Strict mode rejects
unresolved dynamic types at its configured gate. Static inference is a later
consumer of the already-working runtime registry and validation rules.

## Smoke-test gates

- Registry slice: export and import two same-shaped named structs. Exit when
  their IDs differ and each instance resolves back to its declaration.
- Runtime slice: pass and fail every type-expression variant at a binding,
  argument, return, and field boundary. Exit when diagnostics identify the
  boundary span and nested value path.
- Checker slice: infer locals and calls using the shared host signatures. Exit
  when no signature/type vocabulary is duplicated outside the registry.
- Strict slice: run the common parity corpus in gradual and strict modes. Exit
  when gradual behavior is unchanged and strict failures are deterministic.

Dependencies are the canonical runtime/signature catalog, module identity,
runtime validation, and then inference. These gates are required before a
strict-mode default can be considered.


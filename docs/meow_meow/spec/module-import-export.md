# ₍˄·͈༝·͈˄₎ MMS Module System — Spec

> **Status: implemented.** This describes the working v1 module system.
> See `docs/meow_meow/analysis/module-import-export.md` for the broader design exploration.

---

## Export

Use `export` to make a binding visible to other files. Bare `let`/`fn` are file-private.

```mms
// math.mms
export let pi = 3.14159265358979
export let tau = 6.28318530717959

export fn lerp(a, b, t) { return a + (b - a) * t }

let _scratch = 0   // file-private — not importable
```

Only `export let` and `export fn` at the root level are supported. Exports inside function bodies or if-branches are ignored (treated like `let`).

---

## Import

```mms
import { name }             from "file.mms"   // named — requires 'export'
import { name as alias }    from "file.mms"   // named with local alias
import ast { name }         from "file.mms"   // deferred component template
import ast { 0 as alias }   from "file.mms"   // positional template, requires 'as'
```

Multiple items in one statement:

```mms
import { pi, lerp, tau as τ } from "math.mms"
```

Paths are resolved relative to the importing file's directory.
In the target runner architecture, that directory comes from the importing
module's canonical `SourceId`, not ambient process state. Raw source without a
source identity cannot resolve a relative import and returns a typed
source-resolution error.

---

## Positional imports — root CE emits

Every free-standing component expression at the root of a file is indexed in emission order (0, 1, 2, ...), whether or not it uses `export`. This lets you import geometry files that have no bindings at all:

```mms
// parts.mms — no exports, just geometry
T.position(0, 0, 0) { R.cube() {} }   // index 0
T.position(1, 0, 0) { R.sphere() {} } // index 1
```

```mms
// scene.mms
import ast { 0 as cube, 1 as sphere } from "parts.mms"
cube
sphere
```

Positional imports always require an `as` alias — `import { 0 }` is a parse error.

---

## Module evaluation semantics

Modules are evaluated once per session/module identity. Ordinary `import`
binds evaluated values: scalar, table, and function exports are copied as
runtime values; direct component exports and positional roots are registered
as detached live components and preserve their identity on repeated imports.
Each imported factory call still evaluates its body and produces a fresh live
component tree.

`import ast` is the explicit deferred path. It accepts only component exports
or positional roots and binds `ComponentExpr(MaterializedCE)` values. It does
not register, attach, or initialize an ECS component.

```mms
import ast { 0 as my_cube } from "parts.mms"
my_cube
```

Ordinary named exports (functions, numbers, etc.) are available immediately
after import and work like any local binding. `import ast` of a non-component
export is an error.

---

## File resolution

```
"math.mms"          → relative to the importing file's directory
"lib/math.mms"      → relative path segments supported
```

Absolute paths are also accepted (pass through unchanged). There is no stdlib prefix (`@std/`) in v1 — stdlib files are loaded by relative path or prelude.

`run_file` and module-file entrypoints canonicalize the root identity.
Successful loads return canonical identities that drive diagnostics, nested
relative resolution, and per-session module-cache identity. Different path
spellings of the same canonical file must not produce duplicate module
instances.

---

## What is / isn't supported in v1

| Feature | Status |
|---------|--------|
| `export let` / `export fn` at root | ✅ |
| `import { name }` named import | ✅ |
| `import { name as alias }` | ✅ |
| `import { 0 as alias }` positional | ✅ |
| Mixed `{ name, 0 as alias }` list | ✅ |
| Relative path resolution | ✅ |
| Imports inside function bodies | target: use the defining/importing module `SourceId`; fail only when no identity exists |
| `import parts from "..."` namespace import | ❌ not yet |
| Circular import detection | ❌ not yet (will stack overflow) |
| Module caching (eval once) | target: cache by canonical resolved `SourceId` |
| `@std/` stdlib prefix | ❌ not yet |
| Re-export (`export { x } from "..."`) | ❌ not yet |
| `import "file.mms" as ns` namespace import | ❌ not yet — required for `ns.query(...)` |
| `module.query(selector)` / `.query_all()` | ❌ not yet — see [mms-query.md](../draft/mms-query.md) |

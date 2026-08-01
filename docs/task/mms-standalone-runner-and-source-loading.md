# MMS standalone runner and source loading

Date: 2026-07-30

Status: planned

Normative architecture:
[Mittens host and MMS runtime boundary](../meow_meow/spec/mittens-host-and-runtime-boundary.md).

This task implements the crate-owned standalone path. It does not introduce a
second runtime specification or change Mittens' engine behavior.

## Goal

Make `meow-meow-script` useful for scripts and component-producing modules
without Mittens or a configuration builder:

- `Runtime::standard()` creates the crate-owned open-name runtime.
- `Runner::standard()` creates a standard session and `StandardHost`.
- `Runner::new(SessionClient)` remains the core configuration-independent API.
- `Repl::standard()` and `Repl::new(Runner)` follow the same split.

## Runtime and host

- [ ] Add `Runtime::standard()` backed by one crate-owned `RuntimeSpec` using
      `ComponentNamePolicy::OpenUppercase`.
- [ ] Keep custom hosts usable with that runtime; do not require callers to
      clone or extend a builder merely to replace `StandardHost`.
- [ ] Add `StandardHost` with:
  - an ordered forest of collected emitted roots
  - opaque local handles for registered component nodes
  - local register/attach behavior preserving forest and child order
  - local component type/children/field inspection
  - filesystem source loading
- [ ] Expose collected roots and attachment results through crate-owned result
      DTOs.
- [ ] Return typed `UnsupportedHostOperation` for queries, engine component
      methods/APIs, audio, engine mutations, and other engine-only operations.
- [ ] Preserve typed invalid-request, conversion, source-loading, and protocol
      errors; do not encode failures as `null` or logged-only output.

## Source identity and module loading

- [ ] Add `Runner::run_file` and module-file entrypoints that canonicalize the
      root path before evaluation.
- [ ] Make source-load responses return canonical, stable `SourceId`s.
- [ ] Resolve each relative import against its importer identity, including
      nested imports.
- [ ] Key the per-session module cache by resolved canonical identity, not the
      textual import spelling.
- [ ] Allow raw source to supply an explicit `SourceId`.
- [ ] Reject a relative import from raw source without an identity with a
      deterministic typed source-resolution error.
- [ ] Never fall back to the process working directory for an identity-less
      relative import.

Filesystem support here is source loading, not a general ambient authority
surface. Networking, process control, and engine assets remain out of scope.

## Compatibility

- [ ] Keep existing Mittens runner entrypoints as wrappers over the generic
      runner.
- [ ] Keep `Runner::new(SessionClient)` and fake/custom-host tests independent
      of `RuntimeSpecBuilder`.
- [ ] Preserve existing Mittens runner compile fixtures and observable output,
      error, source-location, and import behavior as release gates.
- [ ] Do not add an engine-local component-name or source-resolution policy.

## Verification

- [x] `Runner::standard()` accepts and collects an arbitrary uppercase
      component tree, including nested unknown labels.
- [ ] Collected roots, registered local handles, and later attachments retain
      authored field and child order.
- [ ] A file entrypoint loads nested relative imports without linking another
      crate.
- [ ] Equivalent canonical paths share one module-cache entry.
- [ ] Raw-source relative imports fail deterministically when no `SourceId` is
      supplied.
- [x] Engine-only operations under `StandardHost` return their typed
      unsupported errors.
- [ ] A fake/custom host works with `Runtime::standard()` and
      `Runner::new(SessionClient)` without Mittens or a builder.
- [ ] Existing Mittens runner compile fixtures and observable-behavior tests
      pass unchanged or through an audited compatibility facade.

## Related

- [Component reflection and table dot access](mms-component-reflection-and-table-dot-access.md)
- [MMS REPL navigation and cat unification](mms-repl-navigation-and-cat-unification.md)
- [MMS evaluator deduplication checklist](mms-evaluator-deduplication.md)
- [Generic runner and REPL boundary](../meow_meow/analysis/generic-runner-and-repl-boundary.md)

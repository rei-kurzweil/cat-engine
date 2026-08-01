# Mittens MMS ownership cutover and 0.8 release

Date: 2026-07-31

Status: active planning task

## Purpose

Carry the Mittens-facing portion of the MMS runtime migration to the point
where the language implementation is opaque to `mittens-engine` and the
engine can make and ship an honest version decision.

This is the companion to
[MMS runtime configuration, generic frontends, types, and method dispatch](../meow_meow/task/epic/runtime-configuration-frontends-types-and-method-dispatch.md).
It covers that epic through Phase 5 only. Runtime numeric representation,
intrinsic receiver methods, inference, and static checking remain
`meow-meow-script` work unless they change a boundary DTO or the builder API.

The detailed implementation checklist remains
[MMS evaluator deduplication](mms-evaluator-deduplication.md). This task owns
the Mittens cutover sequence, compatibility decision, and release gate rather
than duplicating every evaluator checklist item.

## Recommendation

Plan the cutover as:

- `meow-meow-script` `0.6.0` -> `0.7.0`; and
- `mittens-engine` `0.7.0` -> `0.8.0`.

Commit to or reject the Mittens `0.8.0` target immediately after the Phase 0
public-API audit. Do not defer that decision until all migration work is
finished.

`mittens-engine 0.7.1` is appropriate only if the audit and compatibility
fixtures demonstrate that every supported public Rust surface and observable
behavior can be preserved through a boundary-safe facade. A patch version is
not appropriate merely because the common runner methods still compile.

The current known hazards make `0.8.0` the expected outcome. Public types such
as engine-local MMS `Value`, `MaterializedCE`, `RuntimeClosure`, worker
protocol types, and the public fields of `LoadedMmsModule` expose the
architecture being removed. Preserving their old semantics would retain a
second heap, evaluator, or closure model and violate the target boundary.

The speed of the previous `0.5` -> `0.6` -> `0.7` releases does not change the
compatibility meaning of the next release. Before `1.0`, a `0.7` -> `0.8`
change is the normal way to communicate an incompatible public API. Avoiding
that bump is less valuable than reaching one coherent scripting boundary.

## The point where Mittens stops caring

At completion, Mittens owns only:

- the one nested builder expression that produces its `RuntimeSpec` and
  opaque implementation bindings;
- `MittensHost`, which services crate-owned requests on the engine thread;
- conversion between crate boundary DTOs and ECS/engine values;
- component construction, queries, mutations, signals, assets, and other
  engine implementations;
- engine frame-loop, terminal, and live-world inspection adapters; and
- intentionally retained high-level compatibility conveniences.

Everything on the right of this boundary is opaque to Mittens:

```text
mittens-engine                         meow-meow-script
──────────────                         ────────────────
build RuntimeSpec ───────────────────► Runtime
start/drive operation ───────────────► SessionClient / Runner / Repl
service HostRequest ◄───────────────── evaluator pauses
return HostResponse ─────────────────► evaluator resumes
adapt final result ◄────────────────── typed result/events
```

Mittens must not know how MMS represents scopes, heap objects, closures,
modules, REPL state, inferred types, or intrinsic dispatch. Later MMS work
matters to Mittens only when it deliberately changes `RuntimeSpec`, a public
boundary DTO, or the host request/response protocol.

## Scope

This task includes:

1. the compatibility inventory and early release decision;
2. the Mittens `RuntimeSpec` builder expression and opaque host bindings;
3. the Mittens adapter over the crate-owned session and generic runner;
4. migration of modules, factories, callbacks, and keyframes;
5. migration of the REPL and worker integration;
6. deletion of the engine-local evaluator/runtime model; and
7. the coordinated crate version changes and migration notes.

It does not include:

- fixed-width MMS numeric values;
- `[T].length()`, `str.length()`, or numeric intrinsic dispatch;
- typed binding/function syntax;
- inference, static checking, strict mode, or transpilation; or
- a language server.

Component method bindings are in scope because they cross into Mittens.
Collection, string, and numeric methods implemented as crate intrinsics are
not.

## Phase 0: compatibility audit and version commitment

This phase happens before the builder migration grows compatibility shims.

- [ ] Inventory supported public and observable scripting surfaces:
  - `MeowMeowRunner` methods and result behavior;
  - `LoadedMmsModule`, its public fields, and export access;
  - engine-local `Value`, `MaterializedCE`, `RuntimeClosure`, heap, and object
    types;
  - public evaluator/worker request and response types;
  - `MittensHost` construction and handle-conversion APIs;
  - callback-bearing ECS components and constructors;
  - `IntentValue` variants carrying MMS runtime data; and
  - asset, panel, paint, pose, and preview types that expose module or
    materialized component data transitively.
- [ ] Add compile fixtures for the supported external Rust API.
- [ ] Record behavior fixtures for runner results/errors, module identity,
      template/live factories, handlers, and keyframes.
- [ ] Classify every surface as:
  - preserved exactly;
  - preserved through a boundary-safe compatibility facade;
  - explicitly unstable/internal; or
  - deliberately breaking.
- [ ] Reject any proposed facade that requires a second evaluator, heap,
      closure representation, or mutable module state outside the crate
      session.
- [ ] Record the release commitment:
  - choose `0.8.0` if any supported surface is deliberately breaking; or
  - choose `0.7.1` only if all supported surfaces and behaviors have passing
    compatibility fixtures.

Recommended gate outcome: commit to `mittens-engine 0.8.0` and use the audit
to write a focused migration guide rather than to preserve architecture that
is being removed.

Exit gate: the version target is decided and recorded before implementation
choices are distorted by an unresolved compatibility promise.

## Phase 1: one Mittens runtime configuration and host

- [ ] Replace independent component, constructor, property, method, signal,
      builtin, API, and parser-support lists with one nested crate-owned
      `RuntimeSpec` builder expression.
- [ ] Configure `ComponentNamePolicy::StrictRegistered` for Mittens.
- [ ] Attach concrete engine implementations while declaring their
      signatures.
- [ ] Make the build produce the one `RuntimeSpec` plus opaque implementation
      bindings containing no duplicate names or schemas.
- [ ] Route component methods and engine APIs by opaque operation ID.
- [ ] Remove `HostCapabilities` negotiation and string-based support matches.
- [ ] Complete `MittensHost` coverage for component lifecycle, queries,
      methods, handlers, audio, mutations, render assets, source loading, and
      typed unavailable-context errors.
- [ ] Validate session ownership and ECS generation on every component-handle
      operation.

Exit gate: Mittens describes MMS vocabulary once and services every declared
effectful operation exactly once.

## Phase 2: ordinary runner cutover

- [ ] Construct crate-owned persistent sessions from the configured runtime.
- [ ] Drive the generic `Runner` by servicing correlated `HostRequest`s with
      short-lived `MittensHost` instances.
- [ ] Preserve the high-level `MeowMeowRunner` convenience surface where it
      remains useful, but implement it only as an adapter.
- [ ] Migrate ordinary source, file, timeout, world, and render-asset runner
      entrypoints.
- [ ] Preserve canonical source identities and nested relative imports.
- [ ] Preserve observable compatibility only where promised by Phase 0; use
      the new API directly elsewhere.
- [ ] Move every executable MMS example onto the crate worker.

Exit gate: ordinary Mittens evaluation cannot reach the engine-local
evaluator.

## Phase 3: modules, factories, and delayed execution

- [ ] Migrate module loading and named/sequence export calls to the persistent
      crate session.
- [ ] Preserve shared heap identity across repeated export calls.
- [ ] Preserve explicit template and live factory modes.
- [ ] Migrate asset, panel, paint, pose, preview, and world-panel callers.
- [ ] Replace stored engine closures with opaque
      `(SessionHandle, CallbackHandle)` references.
- [ ] Migrate handlers, keyframes, animation callbacks, and audio lookahead.
- [ ] Define session lease, reset, and stale-callback behavior.
- [ ] Queue callbacks raised during host dispatch rather than synchronously
      re-entering the session.

Exit gate: Mittens stores no MMS closure body, module heap, or callable
runtime value.

## Phase 4: REPL and worker cutover

- [ ] Replace engine REPL evaluation with the crate-owned programmatic REPL.
- [ ] Keep only frame polling, terminal ownership, ECS inspection,
      GUID/short-ID resolution, and source snapshot adapters in Mittens.
- [ ] Service live inspection through crate-owned request/response DTOs.
- [ ] Remove copied host dispatch from the engine REPL backend.
- [ ] Replace the engine-local worker/ring-buffer protocol with the crate
      session protocol.
- [ ] Preserve or deliberately migrate the user-visible `ls`, `pwd`, `cd`,
      and `cat` behavior.

Exit gate: the engine REPL contains no parser, evaluator, heap, module, or
navigation semantics owned by MMS.

## Phase 5: deletion and release

- [ ] Delete `src/scripting/world_evaluator.rs`.
- [ ] Delete engine-local MMS `Value`, `ObjectWorld`, `MaterializedCE`, and
      closure state after callers have migrated.
- [ ] Delete the legacy evaluator request/response and ring-buffer protocol.
- [ ] Remove alternate expression evaluation from the component registry.
- [ ] Remove dead vocabulary, capability, signal, and method-support lists.
- [ ] Search outside `crates/meow-meow-script` for remaining MMS evaluator
      logic and remove it.
- [ ] Bump `meow-meow-script` to `0.7.0` and update the Mittens dependency.
- [ ] Apply the Phase 0 Mittens decision:
  - normally bump `mittens-engine` to `0.8.0`; or
  - use `0.7.1` only with the complete compatibility evidence required above.
- [ ] Publish migration notes covering runtime construction, runner/module
      changes, callback handles, errors, and removed public runtime types.
- [ ] Run crate, integration, example, compile-fixture, and full workspace
      test suites.

Exit gate: the manifests communicate the actual compatibility change,
Mittens depends on the new crate boundary, and no legacy language
implementation remains in the engine.

## Release policy after this cutover

Do not reserve another immediate Mittens breaking bump merely because MMS
later gains numeric types, receiver intrinsics, or static checking. Those are
language-crate changes and should be opaque to the engine.

Reconsider the Mittens version only if later work changes one of its actual
surfaces:

- the `RuntimeSpec` builder calls Mittens must make;
- host operation binding or `HostRequest`/`HostResponse` DTOs;
- engine-facing runner/module/callback APIs; or
- observable behavior that Mittens has promised to preserve.

This makes `0.8.0` a useful architectural boundary instead of merely the next
number in a rapid sequence.

## Completion criteria

- The Phase 0 version decision was made before the main implementation.
- Mittens constructs one strict `RuntimeSpec` and owns no parallel catalog.
- All engine effects are reached through opaque, specification-bound host
  operations.
- All runner, module, factory, callback, keyframe, REPL, and example paths use
  the crate session.
- Mittens owns no MMS evaluator, heap, closure, module state, or REPL
  semantics.
- Pure/runtime typing and intrinsic method implementation can evolve without
  Mittens changes.
- `meow-meow-script` is released as `0.7.0`.
- `mittens-engine` is released as `0.8.0`, unless full compatibility evidence
  justifies `0.7.1`.
- The release contains migration guidance appropriate to the selected
  version.

## Related documents

- [MMS runtime configuration, generic frontends, types, and method dispatch](../meow_meow/task/epic/runtime-configuration-frontends-types-and-method-dispatch.md)
- [Mittens host and MMS runtime boundary](../meow_meow/spec/mittens-host-and-runtime-boundary.md)
- [MMS evaluator deduplication](mms-evaluator-deduplication.md)
- [Standalone runner and source loading](mms-standalone-runner-and-source-loading.md)
- [Generic MMS REPL migration and navigation](mms-repl-navigation-and-cat-unification.md)


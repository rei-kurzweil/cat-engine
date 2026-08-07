# Mittens MMS ownership cutover for 0.9.0

Date: 2026-08-06

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

The release-level sequence is now defined by
[Mittens release roadmap: 0.8.0 and 0.9.0](release-roadmap-0.8.0-0.9.0.md).
This document covers the `0.9.0` MMS milestone. The catalog/configuration
foundation and the complete ownership cutover now ship together.

## Release and compatibility decision

The release boundary is:

- `mittens-engine 0.8.0` ships avatar mapping plus editor grid, paint, and
  accordion-restoration reliability and does not wait for this task; and
- `mittens-engine 0.9.0` ships the catalog foundation and is gated on the point
  where the duplicated
  engine-side evaluator, heap, closure, module, callback, and REPL runtime
  model has been removed.

For ordinary Mittens applications and MMS-authored scenes, this migration is
intended to preserve behavior. Custom script-interpreter configuration was not
a supported Mittens feature, and the new Mittens catalog describes the same
script-visible components and APIs, so adding it is not by itself a breaking
Mittens user experience.

Direct `meow-meow-script` embedding is a separate compatibility surface. The
crate is pre-1.0; its transitional flat `RuntimeBuilder`, `HostCapabilities`,
and host-owned `Session<H>` may change as the nested specification and
host-independent session become supported. Publish a direct-embedder migration
guide whenever that API changes.

Do not force `meow-meow-script` to jump to `0.9.0` merely to match the Mittens
plan. When its packaged API changes, bump it to the appropriate next pre-1.0
version: align with the current Mittens version when practical, otherwise
increment from its latest published version. Update the Mittens dependency
requirement and lockfile atomically.

`mittens-engine 0.9.0` remains appropriate even when ordinary application
behavior is preserved because public engine-local MMS values,
`LoadedMmsModule` state, evaluator/worker protocol types, and closure-bearing
ECS surfaces expose the architecture being removed. Compatibility façades are
acceptable only when they do not retain a second evaluator, heap, closure
representation, or mutable module state.

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

## Other release gates

The earlier pause is over, but these are independent `0.8.0` gates:

- [ ] [editor panel minimize and render suspension](editor-panel-minimize-and-render-suspension.md)
      is implemented but has open Asset/World restoration regressions;
- [ ] [editor grid and paint reliability](editor-grid-paint-0.8.0-release-gate.md)
      remains untriaged and unresolved; and
- [x] the implementation direction for
      [XR controller/hand pose basis and laser alignment](xr-controller-hand-pose-basis-and-laser-alignment.md)
      has landed, although headset validation remains open.

Humanoid automapping/AVC cleanup is implemented for `0.8.0`. None of these
editor/avatar gates is a prerequisite for beginning the `0.9.0` MMS work.

## Phase 0: compatibility audit and release-boundary commitment

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
- [x] Record the release commitment: the catalog/configuration foundation and
      legacy MMS deletion ship together in Mittens `0.9.0`.
- [ ] Record the next `meow-meow-script` version from its actual latest
      published version when the supported builder API is ready.
- [ ] Keep manifest/dependency/lockfile changes atomic for every MMS crate
      release consumed by Mittens.

Gate outcome: preserve the ordinary Mittens experience, permit direct-embedder
API evolution with migration guidance, and reject compatibility shims that
would preserve the duplicated runtime architecture.

Exit gate: the version target is decided and recorded before implementation
choices are distorted by an unresolved compatibility promise.

## Phase 1: one Mittens runtime configuration and host

This phase is the foundation of `mittens-engine 0.9.0`. It may land internally
while the legacy evaluator remains operational, but it is not published as a
separate Mittens milestone. There must be exactly one new catalog and ordinary
script behavior must remain unchanged.

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

This phase and the phases after it continue toward `mittens-engine 0.9.0`.

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

## Phase 5: deletion and release handoff

- [ ] Delete `src/scripting/world_evaluator.rs`.
- [ ] Delete engine-local MMS `Value`, `ObjectWorld`, `MaterializedCE`, and
      closure state after callers have migrated.
- [ ] Delete the legacy evaluator request/response and ring-buffer protocol.
- [ ] Remove alternate expression evaluation from the component registry.
- [ ] Remove dead vocabulary, capability, signal, and method-support lists.
- [ ] Search outside `crates/meow-meow-script` for remaining MMS evaluator
      logic and remove it.
- [ ] Confirm that the catalog/configuration surface has no remaining
      transitional duplicate that must survive deletion.
- [ ] When the deletion gate is reached, bump `mittens-engine` to `0.9.0`.
      Bump `meow-meow-script` according to its actual latest published version
      and cutover API changes; update the Mittens dependency requirement and
      lockfile in the same change.
- [ ] Publish migration notes covering runtime construction, runner/module
      changes, callback handles, errors, and removed public runtime types.
- [ ] Run crate, integration, example, compile-fixture, and full workspace
      test suites.

Exit gate: no legacy language implementation remains in the engine, Mittens
depends on the new crate boundary, and `mittens-engine 0.9.0` is ready to
publish with the corresponding released MMS dependency.

## Release policy after the cutover

MMS runtime-type, receiver-intrinsic, and static-checking work does not
automatically gate `mittens-engine 0.9.0`. It may land before or after the
ownership cutover when it remains opaque to the engine and does not destabilize
the supported `RuntimeSpec` or boundary DTOs.

Reconsider whether an MMS crate release must be coupled to a Mittens release
only if that work changes one of Mittens' actual surfaces:

- the `RuntimeSpec` builder calls Mittens must make;
- host operation binding or `HostRequest`/`HostResponse` DTOs;
- engine-facing runner/module/callback APIs; or
- observable behavior that Mittens has promised to preserve.

This makes `0.9.0` the combined catalog/configuration and precise
legacy-deletion boundary.

## Completion criteria

- The release sequence records `0.9.0` as both the configuration foundation
  and ownership/deletion boundary.
- Mittens constructs one strict `RuntimeSpec` and owns no parallel catalog.
- All engine effects are reached through opaque, specification-bound host
  operations.
- All runner, module, factory, callback, keyframe, REPL, and example paths use
  the crate session.
- Mittens owns no MMS evaluator, heap, closure, module state, or REPL
  semantics.
- Pure/runtime typing and intrinsic method implementation can evolve without
  Mittens changes.
- `meow-meow-script` is versioned from its actual published history and every
  direct-embedder API change has migration guidance.
- `mittens-engine 0.9.0` contains no duplicated engine-side MMS runtime and
  documents any deliberate Rust API changes while preserving the intended
  ordinary application and authored-scene behavior.

## Related documents

- [Release roadmap: 0.8.0 and 0.9.0](release-roadmap-0.8.0-0.9.0.md)
- [Mittens/MMS cutover resume checkpoint](mittens-mms-cutover-resume-checkpoint.md)
- [Editor panel minimize and render suspension](editor-panel-minimize-and-render-suspension.md)
- [XR controller/hand pose basis and laser alignment](xr-controller-hand-pose-basis-and-laser-alignment.md)
- [MMS runtime configuration, generic frontends, types, and method dispatch](../meow_meow/task/epic/runtime-configuration-frontends-types-and-method-dispatch.md)
- [Mittens host and MMS runtime boundary](../meow_meow/spec/mittens-host-and-runtime-boundary.md)
- [MMS evaluator deduplication](mms-evaluator-deduplication.md)
- [Standalone runner and source loading](mms-standalone-runner-and-source-loading.md)
- [Generic MMS REPL migration and navigation](mms-repl-navigation-and-cat-unification.md)

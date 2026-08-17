# MMS/Mittens 0.8 contract cutover

Date: 2026-08-16

Status: active cross-project task

## Purpose

Stabilize the public contract between `meow-meow-script` and `mittens-engine`
for their 0.8.0 releases. The target is for Mittens to describe its scripting
surface once through `RuntimeSpec`/`ConfiguredRuntime`, while MMS owns parsing,
evaluation, sessions, and the host protocol.

This is a coordinating roadmap. Detailed inventories and implementation notes
remain in:

- [Runtime cutover and legacy deletion](mms-mittens-runtime-cutover-and-legacy-deletion.md)
- [MMS RuntimeSpec and MittensHost bindings](mms-runtime-spec-and-mittens-host-bindings.md)
- [MMS evaluator deduplication](mms-evaluator-deduplication.md)
- [Mittens host and MMS runtime boundary](../meow_meow/spec/mittens-host-and-runtime-boundary.md)

The older trackers sometimes describe the destination as 0.9. For this task,
the release target is 0.8.0. Those references should be reconciled during the
public-contract audit rather than silently treated as current.

## Scope rule

Migrating non-demo components is parallel coverage work, not the structure of
this roadmap. It may proceed alongside Phase 1, but component coverage alone
does not establish a stable crate boundary.

## Phase 1 — stabilize the 0.8 boundary

### 1. Harden the persistent-session boundary

- [x] Replace unrestricted host rebinding with a scoped host lease or an
      equivalently constrained session API.
- [x] Make callback ownership by an originating session/runtime explicit and
      enforceable.
- [x] Serialize entry into a session and return a typed error when required
      live host context is unavailable.
- [x] Decide which current session/callback types are stable public API and
      keep temporary Mittens frame-loop scaffolding internal where possible.
- [x] Do not implement terminal/socket attachment as part of this phase; use
      the attachable-session draft only to avoid closing off that design.

The stable primitive is the MMS-owned `Session` plus its scoped `with_host`
lease. Callback handles are accepted only by their originating session. The
Mittens `RuntimeSpecSession` remains a private-state convenience driver around
that contract; it does not define a second session model.

### 2. Cut the ordinary runner over

Cutover probe (2026-08-16): routing the ordinary runner directly through the
configured MMS evaluator passed 194 of 237 scripting tests. The durable blocker
is lifecycle ownership: the ordinary `EvalOutput`-only facade currently has no
place to retain the MMS `Session` that owns callbacks and module state after the
initial evaluation returns. The ordinary path remains on the explicitly named
legacy evaluator for now; no compatibility fallback was added. Remaining probe
failures also identify component-signature and language-parity work for later
vertical slices.

- [ ] Make the ordinary `MeowMeowRunner` evaluation path use the crate-owned
      evaluator and its `ConfiguredRuntime` without requiring callers to opt
      into a special RuntimeSpec entry point.
- [ ] Preserve the intended public runner facade, source paths, results, and
      error behavior, or record deliberate 0.8 breaks explicitly.
- [ ] Run the shared example/smoke corpus through the ordinary runner path.
- [ ] Stop adding behavior to `src/scripting/world_evaluator.rs`; leave it as
      removable migration scaffolding.

### 3. Remove parallel vocabulary and configuration surfaces

- [x] Derive or remove `HostCapabilities` rather than advertising a second
      host schema beside `RuntimeSpec`.
- [ ] Remove string-based component-method support checks and derive or remove
      `SUPPORTED_COMPONENT_NAMES` and similar lists.
- [x] Audit the flat `RuntimeBuilder`, `ComponentSpec`, and `HostApiSpec`
      surfaces; remove, deprecate, or clearly subordinate them so 0.8 does not
      stabilize two competing configuration systems.
- [ ] Ensure implementation bindings contain behavior only, not copied names,
      signatures, aliases, or parser metadata.

### 4. Finish the operation-ID host contract

- [ ] Carry opaque operation IDs across every intended 0.8 host-effectful
      domain, including component constructors, initializers, methods, and
      signals.
- [ ] Give audio, engine mutations, source loading/imports, and miscellaneous
      host APIs either operation-ID dispatch or an explicit typed unsupported
      boundary.
- [ ] Settle public request, response, and binding enum shapes before 0.8 so
      later implementation work does not require changing the host protocol.
- [ ] Remove authoritative legacy string dispatch once its corresponding
      operation domain has migrated.

### 5. Validate and audit the contract

- [x] Reject duplicate or conflicting declarations, invalid nesting and body
      modes, unknown signature types, missing bindings, and orphan bindings.
- [ ] Add generated consistency tests proving each effectful declaration has
      exactly one compatible binding.
- [ ] Preserve typed distinctions between foreign/stale handles, unavailable
      context, unsupported operations, invalid input, conversion/source
      failures, and host failures.
- [ ] Audit the published Mittens and MMS Rust surfaces, including runner
      signatures, module/result DTOs, callback handles, request/response
      enums, and legacy re-export paths.
- [ ] Record the 0.8 smoke-test and workspace-test baseline.

## Phase 1 exit and reassessment gate

Completing items 1–5 does **not** automatically begin Phase 2.

At this gate:

- [ ] Reassess whether the public contract is honest and sufficiently complete
      for `mittens-engine 0.8.0` and `meow-meow-script 0.8.0`.
- [ ] Identify any remaining work that would still force a public breaking
      change after release.
- [ ] Review the remaining legacy implementation and component-coverage
      inventory.
- [ ] Stop and wait for the maintainer to explicitly initiate Phase 2.

## Phase 2 — internal ownership migration and deletion

**Do not start this phase merely because Phase 1 is complete. The maintainer
will initiate Phase 2 after the reassessment gate.**

Once initiated, migrate or remove the remaining internals behind the stable
0.8 contract:

- [ ] modules and factories;
- [ ] keyframes, delayed callbacks, and audio lookahead;
- [ ] REPL and live navigation;
- [ ] inspector/component serialization helpers;
- [ ] the legacy engine-local evaluators and object/value model;
- [ ] legacy registries, capability lists, and string dispatch; and
- [ ] the legacy evaluator thread protocol and compatibility conversions.

Phase 2 may reorganize either crate internally, but it must consume the Phase 1
contract rather than expand or replace it without a new explicit design
decision.

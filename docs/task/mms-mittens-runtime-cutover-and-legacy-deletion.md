# MMS/Mittens runtime cutover and legacy deletion

Date: 2026-08-16

Status: active focused checklist

## Goal

Make the ordinary Mittens scripting path use the evaluator, persistent
`Session`, `RuntimeSpec`, and host protocol owned by `meow-meow-script`.
After the 0.8 contract is reviewed and accepted, remove the duplicate engine
registries and evaluator without losing the engine behavior currently mixed
into those files.

This is the focused execution checklist beneath
[MMS/Mittens 0.8 contract cutover](mms-mittens-0.8-contract-cutover.md). It does
not authorize Phase 2 automatically.

## Important distinction

[MMS component migration checklist](mms-component-migration-checklist.md)
primarily tracks serialization of live ECS components back into MMS source via
`to_mms_ast`. That is related, but it is not the RuntimeSpec migration.

This checklist tracks the other direction and the runtime boundary:

```text
MMS source
  -> crate-owned parser/evaluator
  -> RuntimeSpec operation ID
  -> Mittens host binding
  -> ECS construction, mutation, query, or signal registration
```

Both directions must survive, but they should not share a second scripting
vocabulary registry.

## Current foundation

- [x] Define the Mittens vocabulary with `RuntimeSpec` and bind effectful
      declarations to opaque operation IDs.
- [x] Split component bindings into constructors, initializers, and live
      component methods.
- [x] Materialize the initial smoke-test components through the crate-owned
      evaluator and configured Mittens host.
- [x] Support persistent MMS callbacks and captured mutable table state in a
      session.
- [x] Constrain live-host access through a scoped `Session::with_host` lease.
- [x] Remove `HostCapabilities` and the public legacy flat runtime/catalog
      builder.
- [x] Add typed RuntimeSpec validation for malformed signatures and signal
      declarations, in addition to missing/orphan/duplicate bindings.
- [x] Add host-mediated source loading with `SourceId`, importer-relative
      resolution, nested imports, and per-session module caching.
- [x] Keep the RuntimeSpec cube/camera/light/bloom smoke example passing.

## 1. Settle ordinary runner and session ownership

The current ordinary runner returns `EvalOutput` and therefore has nowhere to
retain the MMS session that owns callbacks, captured tables, and module cache.
Do not hide this with a legacy fallback.

- [ ] Inventory public callers of every ordinary `MeowMeowRunner` evaluation
      entry point and what each caller needs after initial evaluation.
- [ ] Decide the stable 0.8 ownership shape:
  - a returned execution/session object;
  - a runner-owned session with an explicit identity; or
  - another design that makes ownership and serialized entry equally clear.
- [ ] Ensure the chosen result still exposes the immediate evaluation output
      needed by existing callers.
- [ ] Define how a root script supplies its `SourceId` so relative imports do
      not depend on the process working directory.
- [ ] Define explicit close/drop behavior for callbacks, module state, and any
      queued host invocations.
- [ ] Add tests for two independent sessions using the same configured runtime.
- [ ] Add tests proving callbacks cannot run through the wrong session or
      without required live Mittens context.
- [ ] Route the ordinary runner through `ConfiguredRuntime` and the crate-owned
      evaluator.
- [ ] Remove the special RuntimeSpec opt-in path once the ordinary path fully
      replaces it, or retain only a deliberately distinct low-level API.
- [ ] Confirm there is one execution path, with no automatic legacy fallback.

Exit condition: an ordinary script can be launched, retained across frames,
receive events, import modules, and be shut down without invoking the legacy
evaluator.

## 2. Complete RuntimeSpec component coverage

Create a generated or mechanically auditable coverage table before migrating
components ad hoc.

- [ ] Inventory every canonical component and alias currently recognized by:
  - `src/scripting/component_registry.rs`;
  - `src/scripting/component_method_registry.rs`;
  - parser/component-name lists;
  - component `to_mms_ast` implementations; and
  - existing examples and scripting tests.
- [ ] For every component, classify all exposed behavior as:
  - constructor (`T {}` and `T.named(...) {}`);
  - initializer inside a component-expression body;
  - named or positional property;
  - live component method;
  - signal and payload schema; or
  - ECS-to-MMS serialization only.
- [ ] Add every intended 0.8 constructor to `RuntimeSpec` with an exact typed
      signature and exactly one `ComponentConstructor` binding.
- [ ] Add every intended 0.8 body call/property with exactly one
      `ComponentInitializer` binding.
- [ ] Add every intended 0.8 live method with exactly one `ComponentMethod`
      binding.
- [ ] Add every intended 0.8 signal and payload schema with exactly one signal
      binding where host behavior is required.
- [ ] Add explicit typed unsupported boundaries for behavior intentionally
      excluded from 0.8 rather than silently accepting or string-dispatching it.
- [ ] Cover overloads and fixed-width numeric conversions, including range and
      integral-value failures.
- [ ] Migrate components in smoke-testable vertical slices and update the
      coverage table after every slice.

Suggested vertical-slice order:

- [ ] Transform, renderable/material, color, emissive, cameras, lights, bloom,
      and render-graph settings.
- [ ] Text, layout, style, opacity, scrolling, and interaction components.
- [ ] Input, raycast, selection, collision, and transform-control components.
- [ ] Animation, transition, clock, and keyframe-facing components.
- [ ] Mesh, texture, GLTF, skinning, avatar, IK, and transform-pipeline
      components.
- [ ] Audio components and audio host APIs, after confirming their intended 0.8
      boundary.
- [ ] Editor/runtime-only components, either supported or explicitly excluded.

Exit condition: RuntimeSpec is the sole authoritative description of every
component operation supported by the ordinary 0.8 runner.

## 3. Retire `component_method_registry.rs`

- [ ] Carry operation IDs end-to-end for every configured live component
      method.
- [ ] Remove `supports_component_method(component_type, method)`.
- [ ] Remove dispatch by component and method strings.
- [ ] Move reusable Rust method implementations into engine adapter modules
      that do not contain scripting names or signatures.
- [ ] Bind those behavior-only implementations from `runtime_config.rs`.
- [ ] Add a consistency test proving every declared effectful method has one
      compatible binding and there are no orphan bindings.
- [ ] Delete `src/scripting/component_method_registry.rs`.

Deletion condition: no parser, evaluator, host request, or engine adapter asks
whether a method-name string is supported.

## 4. Split and retire `component_registry.rs`

The file currently mixes scripting vocabulary with engine behavior. Preserve
the behavior, not the parallel registry.

- [ ] Replace `SUPPORTED_COMPONENT_NAMES` and name-resolution lists with data
      derived from the built RuntimeSpec.
- [ ] Replace the constructor match with operation-ID-bound constructor
      implementations.
- [ ] Replace builder/property/positional string dispatch with initializer
      bindings.
- [ ] Remove legacy AST/materialized-value conversion paths after the
      crate-owned evaluator produces the required materialized values directly.
- [ ] Move generic ECS tree spawning/attachment helpers to an engine adapter
      module if they remain necessary.
- [ ] Move ECS-to-MMS subtree serialization and inspector helpers to a
      serialization/inspection module. These are not host vocabulary
      registration.
- [ ] Verify save/load and clone round trips against
      [MMS component migration checklist](mms-component-migration-checklist.md).
- [ ] Delete `src/scripting/component_registry.rs` once no responsibility is
      left in it.

Deletion condition: no second file independently declares component names,
aliases, constructors, initializers, properties, methods, or signatures.

## 5. Make `world_evaluator.rs` unreachable from production

- [ ] Keep the file frozen while cutover work proceeds; add no new language
      behavior to it.
- [ ] Turn every failure found by the 194/237 cutover probe into a categorized
      parity or component-coverage test.
- [ ] Close crate-evaluator language gaps used by supported Mittens scripts.
- [ ] Verify modules and factories through the crate-owned evaluator.
- [ ] Verify persistent callbacks, delayed callbacks, and keyframe behavior
      through the crate-owned session model.
- [ ] Give audio/lookahead behavior an operation-ID contract or an explicit
      typed unsupported boundary.
- [ ] Verify REPL/live-navigation callers against the stable session boundary,
      without embedding REPL commands in the generic host protocol.
- [ ] Route every production runner, loader, example launcher, and relevant
      editor entry point away from `world_evaluator.rs`.
- [ ] Add a dependency/search assertion preventing new production references.

Phase 1 condition: `world_evaluator.rs` is unreachable migration scaffolding.
Physical deletion belongs to Phase 2 after the reassessment gate.

## 6. Contract and smoke-test gate

- [ ] Generate a RuntimeSpec consistency report covering declarations,
      operation IDs, bindings, aliases, and unsupported domains.
- [ ] Run MMS unit and documentation tests.
- [ ] Run Mittens scripting and component tests, recording accepted baseline
      failures separately from migration regressions.
- [ ] Run the headless RuntimeSpec smoke example through the ordinary runner.
- [ ] Run the graphical emissive-cubes example and verify camera, light, bloom,
      click callbacks, and retained table state.
- [ ] Run representative source import/module examples from paths outside the
      repository working directory.
- [ ] Audit public 0.8 Rust API surfaces for accidental legacy types and
      duplicated configuration entry points.
- [ ] Identify anything remaining that would require a breaking public change
      after 0.8.0.

## Phase 1 reassessment gate

- [ ] Review the completed checklist with the maintainer.
- [ ] Decide whether the MMS/Mittens contract is honest enough to release as
      0.8.0.
- [ ] Record any accepted exclusions or deliberately unstable APIs.
- [ ] Stop and wait for explicit maintainer authorization before Phase 2.

## Phase 2 deletion — do not start automatically

Only after explicit authorization:

- [ ] Delete the unreachable `world_evaluator.rs` and its private value/object
      model.
- [ ] Delete remaining legacy evaluator request/response and conversion types.
- [ ] Delete the retired component registries after their surviving engine and
      serialization helpers have moved.
- [ ] Remove legacy re-exports and migration-only entry points.
- [ ] Remove tests that exercise only deleted internals while preserving or
      relocating all observable-behavior regression coverage.
- [ ] Run the full workspace and release smoke-test baseline again.


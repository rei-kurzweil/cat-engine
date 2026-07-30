# Generic MMS REPL migration and navigation

Date: 2026-07-30

Status: planned

Normative architecture:
[Mittens host and MMS runtime boundary](../meow_meow/spec/mittens-host-and-runtime-boundary.md).

## Goal

Move the programmatic, navigation-aware REPL into `meow-meow-script` so it
works with `StandardHost`, fake/custom hosts, or Mittens. The core constructor
is `Repl::new(Runner)`; `Repl::standard()` is the builder-free convenience
path.

The REPL owns persistent input/evaluation and filesystem-like navigation.
Terminal ownership, engine frame integration, and live ECS inspection remain
adapters.

## Command model

The common navigation commands are:

- `ls` lists navigable children.
- `cd 0` or `cd name` moves through the current listing.
- `cd /path` resolves an absolute host-defined component path.
- `pwd` reports the current structured location.
- `cat` renders the current component or value.
- `cat 0` resolves listing index `0`, not the numeric literal.
- `cat name` first resolves a navigable child and then falls back to MMS
  expression evaluation.
- `cat query("...")` evaluates and renders the resulting value.

`ls`, `cd`, `pwd`, and `cat` are REPL commands, not hidden MMS builtins or
entries in `RuntimeSpec`. Call-shaped language forms remain ordinary MMS.
Decide explicitly whether `tree`, `dump`, `help`, `clear`, and `reset` are
REPL-only commands or standard crate builtins.

## Crate-owned REPL

- [ ] Add a programmatic `Repl` that accepts submitted input and emits
      structured `ReplEvent`s without stdin/stdout.
- [ ] Construct it with `Repl::new(Runner)`; never take a `RuntimeSpec` or
      builder.
- [ ] Add `Repl::standard()` over `Runner::standard()` and `StandardHost`.
- [ ] Keep bindings, modules, heap/table identity, current source identity,
      cursor, and breadcrumbs across snippets.
- [ ] Own input classification, multiline completion, navigation resolution,
      reset, error recovery, and shutdown in the crate.
- [ ] Keep pure formatting/rendering of tables, arrays, and component artifacts
      in the crate.
- [ ] Put terminal stdin/stdout/ANSI behavior behind an optional adapter.

## Pure and local navigation

- [ ] Navigate session-owned tables and arrays behind `ValueRef`s without
      copying identity-bearing values across the worker boundary.
- [ ] Navigate static component expressions through their type, authored
      fields, and ordered immediate component children.
- [ ] Navigate component values returned by functions without introducing a
      function-component type.
- [ ] Navigate the collected/local component forest supplied by
      `StandardHost`, including registered and attached local handles.
- [ ] Keep table dot reads and function-valued implicit-`self` methods
      consistent between normal evaluation and REPL snippets.

Pure component/table navigation must work even when the host rejects live
inspection.

## Optional live-host inspection

- [ ] Use crate-owned inspection request/response DTOs for live world and
      component navigation.
- [ ] Support validation, listing, child resolution, parent lookup,
      description/labels, and optional source rendering.
- [ ] Let custom hosts return typed unsupported errors without disabling pure
      navigation.
- [ ] Keep Mittens `World`, liveness checks, GUID/short-ID resolution, ECS
      labels, and subtree snapshots in `MittensHost` or its REPL adapter.
- [ ] Preserve distinct unsupported-inspection, foreign-handle, and
      stale-handle errors.

## Mittens migration

- [ ] Make the existing engine REPL a thin host, frame-loop, terminal, and
      compatibility adapter over the crate REPL.
- [ ] Remove copied evaluator/host dispatch from the engine backend.
- [ ] Keep existing user-visible navigation behavior where it agrees with this
      command model.
- [ ] Preserve supported Mittens REPL and runner compile fixtures as release
      gates.

## Verification

- [ ] `Repl::standard()` evaluates snippets and navigates arbitrary uppercase
      component trees without Mittens or a builder.
- [ ] `cat 0`, `cd 0`, named children, and expression fallbacks resolve in the
      documented order.
- [ ] Tables preserve mutation and closure aliasing across submissions and
      implicit-`self` calls.
- [ ] Functions return component trees whose type, fields, and ordered
      children can be listed and rendered.
- [ ] Static, collected, locally attached, and live component values navigate
      correctly.
- [ ] A fake inspection host supplies roots, children, parents, labels, and
      rendered source.
- [ ] Unsupported live inspection leaves pure table/component navigation
      usable; stale and foreign handles retain distinct errors.
- [ ] Programmatic input/output tests require no terminal.
- [ ] The Mittens adapter runs the same tests against `MittensHost`.

## Related

- [Standalone runner and source loading](mms-standalone-runner-and-source-loading.md)
- [Component reflection and table dot access](mms-component-reflection-and-table-dot-access.md)
- [Generic runner and REPL boundary](../meow_meow/analysis/generic-runner-and-repl-boundary.md)
- [MMS evaluator deduplication checklist](mms-evaluator-deduplication.md)

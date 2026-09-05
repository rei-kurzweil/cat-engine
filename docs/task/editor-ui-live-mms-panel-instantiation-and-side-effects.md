# Editor UI live MMS panel instantiation and side effects

Date: 2026-09-05

Status: planning; higher priority than panel-specific MMS state work

## Goal

Instantiate editor UI panels the same way an ordinary live MMS example instantiates its component
trees.

When an MMS panel factory is evaluated, all normal construction-time effects should take effect,
including:

- producing live component identities
- attaching the returned tree to the world
- registering authored event handlers
- retaining callback closures and their captured state
- running other supported side effects associated with building the component tree

Rust editor systems should receive a live panel shell and operate on it. They should not need to
materialize a panel early as a dead `MaterializedCE`, modify that intermediate tree, and only later
turn it into live world state.

## Motivation

Editor panels are authored in MMS, but the editor currently treats their factories differently
from ordinary live MMS execution. Panel exports are materialized through a template-oriented path,
which preserves their component-tree shape but does not preserve all effects of evaluating the
factory as live MMS.

This distinction becomes visible as soon as a panel owns reactive behavior. For example, the color
panel should be able to listen to its palette `SelectionChanged` event and update its active-color
well from MMS. The handler should capture the well or panel-local state just as it would in a normal
MMS example. A template-only panel path loses that ownership and pushes the behavior back into a
Rust editor system.

The color panel is one consumer of the fix, not the reason to build a color-specific bridge. The
larger issue is that MMS-authored editor panels should retain ordinary MMS runtime semantics.

## Direction

Bias the editor UI toward live evaluation and instantiation:

```text
evaluate MMS panel factory
  -> create live component subtree
  -> run construction side effects
  -> retain callback/session ownership
  -> attach live subtree to editor UI
  -> discover panel roots and slots
  -> populate dynamic slots with live rows as needed
```

The default editor-panel path should not be:

```text
evaluate only far enough to obtain MaterializedCE
  -> transform or store the dead tree in Rust
  -> spawn it later without the factory's live side effects
```

Template materialization can remain available for callers that explicitly need a reusable or
inspectable template. It should not be the default representation of a panel that is being created
for immediate runtime use.

## Live panel shell and dynamic content

Static panel structure and dynamic list content do not need to use the same creation mechanism.

The MMS factory should evaluate once to create the live shell, including:

- panel chrome and layout
- stable content, status, and detail slots
- authored selections and controls
- panel-local handlers and captured component handles

After that shell exists, Rust systems or data renderers may add, replace, or remove dynamic rows by
attaching live subtrees beneath the appropriate slots. List-heavy panels should not require the
entire panel factory to be rematerialized whenever their rows change.

This keeps dynamic projection efficient while allowing the shell to behave like normal authored
MMS.

## Callback and state ownership

A live panel instance must retain whatever runtime ownership is required for its callbacks to keep
working. In particular:

- a handler registered while evaluating a panel factory remains registered for that panel instance
- captured component handles continue to refer to the components attached to that instance
- captured tables or other closure state remain available across handler dispatches
- two instances of the same panel do not accidentally share instance-local closure state
- removing a panel releases or invalidates its handlers and retained runtime state
- recreating a removed panel or body installs fresh handlers bound to the new live components

A simple visual state update may only need to capture a component handle. More complex panels may
capture mutable tables. Both should follow the same lifetime as the live panel instance.

## Collapse, restoration, and rebuilding

Accordion collapse and panel restoration must respect live ownership. If collapsing removes a
body subtree that owns handlers or captured targets, restoration must evaluate or instantiate the
replacement body through a path that installs fresh live side effects. It must not restore a dead
template while leaving callbacks pointed at removed component identities.

The exact lifetime boundary—whole panel, body, or another authored scope—can be decided during
implementation. The invariant is that callbacks and their captured components share a coherent
lifetime.

## Responsibilities after the change

MMS panel modules should be able to own:

- local interaction and presentation behavior
- handlers that update their own controls and visual state
- instance-local closure state
- their stable shell and slot topology

Rust editor systems should continue to own:

- editor domain state and cross-panel orchestration
- dynamic data/model production
- attaching and reconciling live rows in authored slots
- observing bubbled panel events when editor state also depends on them
- runtime facilities that MMS invokes through normal component APIs

One event may therefore have both a panel-local MMS observer and an ancestor Rust observer. Neither
observer should consume or replace the event in a way that prevents the other from running.

## Acceptance criteria

- An editor panel factory is evaluated through a live MMS path when the panel is installed.
- Components captured by a panel callback are live handles to the attached panel instance.
- Event handlers authored by the panel factory are registered and continue working after factory
  evaluation returns.
- Captured table state persists across multiple event dispatches and remains instance-local.
- Dynamic rows can be attached, replaced, and removed beneath an already-live panel shell without
  rebuilding that shell.
- Removing a panel does not leave callable handlers targeting removed components.
- Collapse and restoration produce handlers bound to the currently live body components.
- Multiple instances of one panel do not share handlers, component captures, or mutable state by
  accident.
- The same panel-local event can still bubble to Rust editor owners.
- Panel installation no longer requires CE mutation as its ordinary integration path.

## Validation slices

Use small behavioral fixtures before migrating every editor panel:

1. Instantiate a test panel whose MMS click handler captures a text or style component and mutates
   it. Verify the handler runs in the editor-installed instance.
2. Capture a mutable table, dispatch multiple events, and verify the value persists without leaking
   into a second panel instance.
3. Attach and replace dynamic rows beneath a live content slot while the shell handler continues to
   work.
4. Collapse and restore the panel, then verify the new body and its handlers work and the removed
   body is no longer targeted.
5. Move the color panel's active-color-well update into its MMS selection handler while preserving
   the Rust paint-state observer of the same `SelectionChanged` event.

## Non-goals

- Removing every `MaterializedCE` use from tooling or explicit template workflows.
- Moving editor domain models or all list generation into MMS.
- Rebuilding complete list-heavy panels for every row change.
- Adding a color-panel-specific native binding as a substitute for live callbacks.
- Requiring a general mutable-state framework before captured component handles work.

## Likely touch points

- `src/engine/ecs/system/panel_system.rs`
- `src/engine/ecs/system/editor_inspector_system_stopgap_mms_adapter.rs`
- MMS runner/session APIs responsible for live module factory evaluation
- callback registration, dispatch, and cleanup ownership
- panel restoration and dynamic slot population paths
- panel modules under `assets/components/internal/`

## Related

- [Live Panel Factory Spawn And Stopgap Adapter Seams](./live-panel-factory-spawn-and-stopgap-adapter-seams.md)
- [Live MMS Module Preview Components Vs Panel Materialization](./live-mms-module-preview-components-vs-panel-materialization.md)
- [MMS module component materialization vs instantiation](./mms-module-component-materialization-vs-instantiation.md)
- [MMS: expose deferred component templates only through `import ast`](./mms-componentexpr-only-via-import-ast.md)
- [Color panel active color slot and selection visual policy](./color-panel-active-color-slot-and-selection-visual-policy.md)

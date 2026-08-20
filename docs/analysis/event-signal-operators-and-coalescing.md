# Analysis: event-signal operators and coalescing

## Prompt

Intent signals already have a component-recipient model: an intent names a
recipient component, routing operators can be installed against that recipient,
and systems use the recipient as the anchor for component-specific work.

Event signals have a different shape today.  They are dispatched to explicit
global or scope-rooted handlers.  Those handlers are useful, but there is not
yet an equivalent first-class, component-owned event-operator layer for
filtering, transforming, coalescing, or scheduling event handling.

This is an exploratory note, not an implementation task.

## Current state

- `RxWorld` dispatches an event to global handlers and then along the event
  scope's live ancestor chain.
- Systems and MMS/runtime integrations install closures for events such as
  `Click`, `SelectionChanged`, `ParentChanged`, and `DataEvent`.
- Intent routing has `SignalPipeline` / `SignalRouteUpwardComponent` machinery
  anchored to recipient components.  It is intentionally intent-oriented.
- The World-panel topology refresh is a local workaround, not a general event
  operator: `EditorWorkspaceRuntime` keeps a pending `HashSet<ComponentId>`;
  a global `ParentChanged` handler identifies the affected installed editor
  root and enqueues one internal World-panel refresh intent for that root.

This is appropriate for its narrow purpose, but it means coalescing policy is
currently reimplemented at each event consumer that needs it.

## Useful parallel

| Intent-side concept | Event-side analogue to investigate |
| --- | --- |
| Recipient component | Event handler owner/scope, or an explicitly declared event target |
| Recipient-attached route operator | Handler-owner-attached event operator |
| System acting for a recipient | User or system-installed event handler acting for its scope |
| Route/transform an intent | Filter, transform, aggregate, defer, or consume an event |
| Intent queue timing | Event delivery phase / coalescing boundary |

The event equivalent should not pretend every event has one natural recipient:
a global source, a pointer hit, and a topology mutation may each have different
meaningful anchors.  It should make the target/scope choice explicit.

## Candidate capabilities

- `filter`: deliver only events satisfying a component-owned predicate, such
  as a particular `DataEvent` name or a topology change inside a declared
  subtree.
- `map`: derive a normalized event payload or target before handlers see it.
- `coalesce`: retain one event per key until a declared boundary.  Keys might
  be scope root, event target, pointer, component type, or a custom reducer.
- `debounce` / `throttle`: schedule after a timing boundary without teaching
  every handler its own bookkeeping.
- `consume` / propagation policy: let a handler deliberately stop further
  scoped delivery when that is sound.
- `bridge_to_intent`: make the event-to-mutation transition explicit, so an
  event operator can aggregate first and emit one intent afterward.

## Design questions

1. Is this an extension of `SignalPipeline`, or a sibling `EventPipeline`?
   Keeping event semantics separate is likely clearer because events are
   broadcast/observed while intents are directed work requests.
2. Where do operator components live?  Candidate answers are the handler
   owner, an explicit scope component, or a system-owned registration node.
3. What is the coalescing boundary: one `process_signals` drain, one command
   flush, one engine frame, or a named transaction?  The answer affects both
   predictable UI updates and input responsiveness.
4. How are old topology ancestry and removed nodes represented?  `ParentChanged`
   needs old-parent data because the child may no longer be in the live chain.
5. How can user-authored MMS handlers participate without exposing engine
   implementation details or allowing unbounded retained event state?
6. Which event classes may be safely coalesced?  Pointer press/release and
   lifecycle events often cannot; layout dirtiness and subtree refresh requests
   often can.

## Small proving grounds, later

- Move the editor World-panel topology refresh from its workspace-local set to
  a generic `coalesce by editor root until command-flush end` operator.
- Use the same mechanism for repeated layout-dirty/topology notifications.
- Compare behavior against high-frequency pointer/move events, where a
  different timing policy will be required.

Do not generalize merely to remove a few lines from the editor adapter.  The
operator model needs clear delivery, ownership, cleanup, and observability
rules first.

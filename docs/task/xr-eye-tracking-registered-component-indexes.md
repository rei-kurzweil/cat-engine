# XR eye-tracking registered component indexes

Date: 2026-09-06

Status: interim performance/lifecycle task; implementation pending

Precedes: [One-shot XR eye-tracking source election](one-shot-xr-eye-tracking-source-election.md)

Related: [Generic XR eye-tracking source selection](generic-xr-eye-tracking-source-selection.md)

## Outcome

Make `XREyeTrackingSystem` retain indexes of the eye-tracking components it owns, following the
registration pattern already used by `AvatarControlSystem`, `OpenXRSystem`, `IKSystem`,
`InputXRGamepadSystem`, and other engine systems.

Eye-tracking work must scale with the number of generic selectors and eye-tracking source
components, not with the total number of components in the world/forest.

This is a prerequisite cleanup before implementing bounded startup detection and one-shot source
election. It removes the current full-world discovery scans without changing the intended election
policy.

## Current cost and coupling

The current `XREyeTrackingSystem::tick` performs four calls to `world.all_components()` every
frame:

1. `ensure_generic_sources` scans the entire world for `XREyeTrackingComponent`.
2. `tick_standard` scans the entire world for `VRChatOSCEyeTrackingComponent`.
3. `tick_htc` scans the entire world for `HTCEyeTrackingComponent`.
4. `resolve_generic_trackers` scans the entire world for `XREyeTrackingComponent` again.

For `N` total world components, the baseline discovery cost is approximately four full-world type
checks per frame before socket polling or packet decoding. Adding unrelated models, UI, transforms,
renderables, or editor components therefore increases eye-tracking tick cost.

The transport functions also build temporary ID vectors and prune socket/cache maps by testing
membership in those vectors. At small source counts this is minor, but it is unnecessary work and
obscures ownership.

The problem is not primarily the few priority comparisons for one avatar. It is that the system has
no retained knowledge of its own components and repeatedly rediscovers them from the entire forest.

## Retained indexes

Add explicit sets to `XREyeTrackingSystem`:

```rust
pub struct XREyeTrackingSystem {
    selectors: HashSet<ComponentId>,
    vrchat_osc_sources: HashSet<ComponentId>,
    htc_sources: HashSet<ComponentId>,
    mediapipe_sources: HashSet<ComponentId>,

    // Existing socket, decode-cache, failure, and future election state.
}
```

A single `HashMap<ComponentId, EyeTrackingComponentKind>` is also acceptable if it makes removal and
diagnostics simpler, but hot transport loops should not repeatedly downcast unrelated component
types to rediscover the kind.

Tick behavior becomes:

```text
registered generic selectors -> ensure/configure their small direct-child candidate sets
registered OSC sources       -> poll OSC receivers
registered HTC sources       -> poll HTC receivers
registered MediaPipe sources -> future provider polling/subscription
registered selectors         -> discovery/election or selected-source forwarding
```

Iterating a copied/snapshotted list of registered IDs is acceptable where mutable world access makes
direct set iteration awkward. The snapshot size must be proportional to registered tracking
components, not total world size.

Expected steady-state complexity is `O(selectors + sources + received packets)`, independent of
unrelated world components.

## Registration lifecycle

Follow the established component lifecycle pattern:

1. Each eye-tracking component emits a registration intent from `Component::init`.
2. The mutation/system dispatcher routes that intent to `XREyeTrackingSystem::register(...)`.
3. Registration determines the component kind once and inserts its ID into the corresponding set.
4. Duplicate registration is idempotent through `HashSet::insert`.
5. Removal unregisters the ID and clears every socket, decode cache, failure marker, discovery
   record, selected-source reference, and provider subscription associated with it.

Possible intent surface:

```rust
IntentValue::RegisterEyeTracking { component_id }
IntentValue::RemoveEyeTracking { component_id }
```

The system can inspect the registered component once to classify it as selector, OSC, HTC, or
MediaPipe. Separate typed intents are unnecessary unless they materially simplify diagnostics.

Add matching `SystemWorld::register_eye_tracking(...)` and `remove_eye_tracking(...)` entry points,
or route directly to clearly named `XREyeTrackingSystem` methods in the mutation executor, following
the conventions already used for `InputXR` and `ControllerXR`.

## Runtime-created default sources

The first generic-selection slice currently creates missing default source children from inside
`XREyeTrackingSystem` with low-level `world.add_component` / `world.add_child`. Those APIs do not call
`Component::init`, so an init-intent-only index would miss these components.

Choose and enforce one lifecycle-safe path:

- preferably create/attach defaults through the normal initialized mutation path; or
- when the eye-tracking system internally creates a source, register it in the corresponding index
  in the same operation and initialize it if component initialization is required.

Do not add a fallback full-world scan to compensate for missed registration. Tests must cover both
authored source children and selector-generated default children.

The same rule applies to future MediaPipe components and any source component created by a runtime
selection/retry UI.

## Removal and stale-ID safety

`SystemWorld::remove_subtree_immediate` already performs type-specific cleanup for several retained
indexes before deleting component records. Add eye tracking to that authoritative removal path.

Provide one idempotent method:

```rust
XREyeTrackingSystem::component_removed(component_id)
```

It should remove the ID from every eye-tracking index and every resource/cache map. It should also
clear or invalidate generic selector state that refers to a removed source.

Component `cleanup` may emit the removal intent as the normal path, but subtree removal must remain a
backstop because the current authoritative removal implementation explicitly cleans registered
systems before deleting nodes.

Hot loops should defensively discard an indexed ID if its world record no longer exists or has the
wrong type. This cleanup is proportional to the tracking index and protects against lifecycle bugs;
it is not permission to rescan the world.

## Topology changes

Registration answers “which eye-tracking components exist.” Parent/child topology answers “which
sources belong to which selector or AVC.” Moving an already registered component must not require
re-registering it.

When a source or selector is attached, detached, or reparented:

- mark only the affected selector/AVC topology dirty;
- recompute ownership from its direct children;
- do not scan unrelated roots;
- preserve the one-source-per-avatar rules defined by the election task.

The initial implementation may recompute the direct children of every registered selector because
that still scales with tracking components. A later dirty-selector set is appropriate if topology
work itself becomes measurable.

## Interaction with one-shot election

Land this task first. The election implementation can then operate entirely over retained indexes:

- initialize a selector and its candidate IDs once;
- poll only candidates during the bounded discovery window;
- select one source at the deadline;
- unregister/disable receiver activity for losing candidates;
- poll only the selected source afterward.

The selected source may remain registered as an existing component even when its receiver is
inactive. Distinguish component registration from active polling/subscription so restarting
discovery does not require a world scan.

## Implementation plan

1. Add selector and per-source ID indexes to `XREyeTrackingSystem`.
2. Add idempotent register/remove methods and unit tests for each component kind.
3. Add eye-tracking register/remove intents and `Component::init`/`cleanup` implementations.
4. Wire the intents through both runtime mutation dispatch paths used by the engine.
5. Register selector-generated default children through a lifecycle-safe path.
6. Add `component_removed` to authoritative subtree cleanup and clear socket/cache state
   immediately.
7. Replace all four `world.all_components()` scans in `XREyeTrackingSystem` with retained-index
   iteration.
8. Replace vector membership pruning with direct registered-set/resource-key cleanup.
9. Verify direct source components, nested source candidates, runtime attach/remove, and
   OSC -> HTC -> OSC manual selection remain correct.
10. Only after this lands, implement the two-second one-shot election state machine.

## Tests and measurements

- Initializing each component kind registers exactly one ID in the correct index.
- Repeated registration is idempotent.
- Authored and selector-generated source children are both indexed.
- Runtime attach registers before the next eye-tracking tick.
- Direct removal and subtree removal unregister immediately and release UDP sockets/caches.
- Removing a selected or candidate source invalidates only its owning selector state.
- Reparenting changes ownership without duplicating registration.
- Stale/missing IDs are removed defensively without a panic.
- `XREyeTrackingSystem` contains no steady-state `world.all_components()` call.
- Add a focused benchmark or instrumentation test with a fixed number of trackers and increasing
  counts of unrelated components. Eye-tracking tick time/work count should remain approximately
  constant.
- Add a scaling case that increases tracking sources while holding unrelated components constant;
  work should grow linearly with tracking sources.

## Acceptance criteria

- Eye-tracking tick cost does not scale with unrelated world component count.
- No eye-tracking selector or transport is discovered through a per-frame full-world scan.
- All eye-tracking types participate in explicit registration and removal lifecycle.
- Selector-created defaults cannot bypass registration.
- Removing a component releases its socket/provider/cache state immediately.
- Runtime attachment, removal, and reparenting keep indexes and selector ownership correct.
- The resulting indexes are the only candidate source of truth used by the subsequent one-shot
  election implementation.

## Expected files

- `src/engine/ecs/component/xr_eye_tracking.rs`
- `src/engine/ecs/system/xr_eye_tracking_system.rs`
- `src/engine/ecs/system/system_world.rs`
- `src/engine/ecs/rx/signal.rs`
- `src/engine/ecs/rx/mutation_executor.rs`
- eye-tracking lifecycle and performance tests

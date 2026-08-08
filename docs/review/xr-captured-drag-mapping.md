# XR captured drag-mapping implementation review

Date: 2026-08-08

Status: focused refactor is structurally sound; one mapping-stability issue and several coverage/documentation follow-ups remain

## Scope

This review covers the uncommitted implementation of Stages 1–3 from
[`xr-captured-drag-mapping.md`](../draft/xr-captured-drag-mapping.md), principally:

- hit-independent pointer-ray snapshots in
  [`raycast_system.rs`](../../src/engine/ecs/system/raycast_system.rs);
- captured continuation and runtime drag mappings in
  [`gesture_system.rs`](../../src/engine/ecs/system/gesture_system.rs);
- mapping-surface diagnostics in
  [`gizmo_system.rs`](../../src/engine/ecs/system/gizmo_system.rs) and
  [`gizmo.rs`](../../src/engine/ecs/component/gizmo.rs);
- the frame-order integration in
  [`system_world.rs`](../../src/engine/ecs/system/system_world.rs).

The review treats direct handle-aware mappings from Stage 4 as intentionally out of scope.

## Outcome

The refactor has the right separation of responsibilities and fixes the original zero-hit seam:

```text
PointerSystem activations
        |
        v
RayCastSystem current ray + zero-or-more hits
        |
        v
GestureSystem captured target + continuation + generic mapping
        |
        v
TransformGizmoSystem handle constraint + snapping + mutation
```

`RayCastSystem` clears and republishes its snapshot map every frame, records the ray before querying
the BVH, and keeps the sample even when the query returns no hits. `SystemWorld` processes raycast
signals and then invokes `GestureSystem` while the matching snapshot is still available.

At press time, `GestureSystem` now resolves two independent pieces of active state:

- `DragContinuation` decides whether original-target contact remains required;
- `DragMapping` decides how motion becomes a world-space point.

Built-in gizmo descendants get `Captured + StartRayPlane` for desktop and spatial pointers.
Ordinary spatial interactions remain `RequireTargetContact + ContactHit`. Controller-backed
`Draggable` targets retain their captured controller-translation mapping. Later hits do not replace
the renderable captured at `DragStart`.

The gizmo diagnostic rename is also conceptually correct. The magenta surface is now oriented by
the drag-start ray, matching the gesture mapping plane rather than the later axis/XY/YZ/XZ
constraint. The legacy environment variable remains as a compatibility fallback.

## Findings

### High: near-parallel rays can still produce extreme or behind-origin drag points

[`GestureSystem::ray_plane_intersect`](../../src/engine/ecs/system/gesture_system.rs) rejects
non-finite denominators and `abs(denom) < 1e-4`, but every finite intersection outside that narrow
band is accepted. It does not reject `t < 0`, and the fixed denominator threshold does not bound the
distance to the mapped point.

For a plane at `z = -2`, a normalized ray with direction approximately
`[0.999999994, 0, -0.00011]` passes the guard and intersects about 18,182 world units away. Reversing
the small Z component produces a similarly distant intersection behind the ray origin. Either
sample can turn one controller-rotation frame into a very large `DragMove`, after which the gizmo
constraint and snapping faithfully apply the jump.

This matters particularly for the new XR path because rotating a controller toward 90 degrees from
the captured start-ray normal naturally approaches this condition. The current unit test proves
only that `0.00001` is rejected; it does not prove the draft's stronger requirement that degeneracy
cannot create extreme deltas.

Recommended resolution:

- reject intersections behind the ray origin;
- add a scale-aware stability rule, such as a maximum mapping distance or maximum step relative to
  the captured start distance;
- on an unstable frame, retain the last valid mapped point and resume without emitting the invalid
  step;
- add tests immediately below and above the parallel threshold, for both positive and negative
  `t`.

### Medium: focused tests stop at the gesture/gizmo boundary

The new gesture test proves zero-hit continuation, original-target retention, and `DragEnd` against
a renderable placed beneath a `TransformGizmoComponent`. The existing gizmo tests separately prove
constraint helpers and snapping behavior. None of the focused tests drives the combined
`RayCastSystem -> GestureSystem -> TransformGizmoSystem` path for an XR axis or planar handle.

Consequently, the reported 7 gesture tests and 11 gizmo tests do not yet directly establish these
draft acceptance cases:

- XR axis translation continues after leaving the arm;
- XR planar translation continues after leaving the square;
- continued captured updates reach grid snapping and preserve grid binding;
- two simultaneous gizmo drags retain independent mappings;
- desktop and XR rays produce the same start-plane mapping for the same geometry;
- controller-translation `Draggable` behavior is preserved by a regression test.

The implementation looks consistent with those behaviors, but end-to-end tests would protect the
important dispatch, ancestry resolution, active-raycaster, and constraint seams that unit-level
tests currently bypass.

### Low: the updated draft still describes the pre-refactor runtime as current

The draft status and final section say Stages 1–3 are implemented, while its “Current pieces” and
problem discussion still state that there is no independent current-ray snapshot and that spatial
pointers are unconditionally forced to `RequireTargetContact`. Those statements now describe the
old baseline, not the current runtime.

Rename that section to “Pre-refactor baseline” or update the prose to distinguish historical
behavior from the implemented path. The same cleanup should clarify that `PointerRaySnapshot` is a
publicly re-exported Rust type even though continuation and mapping remain private runtime types.

## Behavior matrix

| Press target / pointer | Continuation | Mapping | Continued hit required |
| --- | --- | --- | --- |
| Gizmo descendant, desktop | Captured | Start-ray plane | No |
| Gizmo descendant, spatial/XR | Captured | Start-ray plane | No |
| Controller `Draggable` | Captured | Controller translation | No |
| Ordinary desktop with default policy | Captured | Start-ray plane | No |
| Ordinary desktop with `RequireTargetContact` | Require target contact | Contact hit | Yes |
| Ordinary spatial target | Require target contact | Contact hit | Yes |

The precedence is sensible for the focused slice: controller `Draggable` behavior wins first,
then gizmo ancestry, then the desktop global policy, with contact-hit behavior as the fallback.

## Additional observations

- Snapshot lifetime is frame-scoped rather than “last known ray,” which avoids silently using a
  stale sample when a raycaster did not cast.
- Snapshot directions produced by both cursor and parent-forward sources are normalized before
  insertion.
- A captured mapping pauses cleanly when no current snapshot exists; it does not fall back to an
  unrelated hit.
- `DragEnd` retains the last valid mapped point, and click qualification remains based on the
  current release hit rather than captured drag continuation.
- The diagnostic still reconstructs the mapping surface from `DragStart` rather than reading the
  private `DragMapping`. For valid production rays the reconstructed point and normal are
  equivalent, but a future mapping variant should expose diagnostic state explicitly rather than
  extending this reconstruction pattern.
- Rotation handles still receive no effective XR rotation because their configured
  `ScreenSpace1DSlider` mapping requires screen deltas. This is consistent with Stage 4 remaining
  future work, but “XR gizmo dragging” should currently be read as the translation-focused slice.

## Validation performed for this review

The following commands pass on the reviewed worktree:

```text
cargo check
cargo test gesture_system::tests --lib
cargo test active_spatial_pointer_retains_current_ray_without_hits --lib
cargo test gizmo_system::tests --lib
```

Results:

- gesture tests: 7 passed;
- hit-independent ray snapshot test: 1 passed;
- gizmo tests: 11 passed;
- `cargo check`: passed.

The commands emit existing repository warnings. This review did not rerun the full library suite or
strict Clippy because the supplied validation already records unrelated failures in both scopes.

## Recommendation

Keep the continuation/mapping split and the frame-scoped ray snapshot design. Before treating the
XR translation fix as fully hardened, close the extreme-intersection case and add at least one
end-to-end XR axis test plus one planar/snap test. The rest of Stage 4 can remain independent.

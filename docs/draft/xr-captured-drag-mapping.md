# XR captured drags and gizmo mapping

Status: Stages 1-3 implemented internally. Direct gizmo mappings and any serialized configuration
remain design work; the internal types described here are not an API contract.

## Problem

An XR controller can start a transform-gizmo drag, but movement stops as soon as its ray no longer
intersects the exact renderable that received `DragStart`. Thin translate arms and plane handles
therefore feel uncaptured.

Desktop dragging does not have this problem. `GestureSystem` captures a plane through the initial
hit point, intersects the current cursor ray with it, and continues emitting `DragMove` without
requiring another handle hit.

The XR ray itself is still computed while the trigger is held. The missing seam is that the current
ray is exposed to `GestureSystem` only as part of `RayIntersected`, and `RayIntersected` exists only
when something was hit. `GestureSystem` also explicitly changes every spatial pointer to
`RequireTargetContact`, even when the configured global policy is `StartPlaneProjection`.

This is not a grid-snapping problem. Grid snapping consumes successful gizmo translations; it
cannot observe translations after the gesture stops producing `DragMove`.

## Goals

- Gizmo drags capture after a successful press for desktop and XR pointers.
- A captured drag receives the current pointer ray even when that ray hits nothing.
- Contact-dependent interactions remain possible.
- Continuation policy, pointer-to-drag mapping, and gizmo constraints have distinct names and
  ownership.
- The first fix can reuse start-plane projection; improved axis, plane, and rotation mappings can
  follow independently.
- Debug visualization shows the surface actually used by the mapping.

## Non-goals

- Redesigning XR grab (`Grabbable`) or controller-translation drag (`Draggable`) in the first pass.
- Choosing final public MMS syntax for drag behavior.
- Adding rotation or scale snapping.
- Requiring a full pointer-event protocol rewrite before XR gizmos can capture.

## Current pieces

### Pointer activation

`PointerSystem` maps desktop mouse and XR controller state into per-pointer
`PointerActivations { pressed, down, released, ... }`. Gesture lifecycle is therefore already
mostly device-independent.

### Pointer ray production

`RayCastSystem` computes a ray each active frame:

- desktop pointers use cursor-through-camera;
- XR/controller pointers use the pointer transform's world-space forward direction.

It then performs the spatial query and emits one `RayIntersected` per hit. There is no independent
"current pointer ray" snapshot. A frame with zero hits consequently provides no ray sample to
gesture code even though the ray was computed.

### Gesture capture and mapping

`GestureSystem` owns per-pointer `GestureState`, captures the raycaster and renderable at press,
and emits `DragStart`, `DragMove`, and `DragEnd`.

The current `DragUpdatePolicy` combines two separate decisions:

- whether continued contact is required;
- how pointer motion becomes a world-space drag point.

Its variants reveal the combination:

- `RequireTargetContact` is a continuation rule and uses consecutive surface hits as its mapping;
- `StartPlaneProjection` is a mapping and implicitly means capture/free-after-start.

Desktop pointers may use `StartPlaneProjection`. Spatial pointers are unconditionally changed to
`RequireTargetContact`. Controller targets carrying `Draggable` take a separate path that follows
controller translation rather than controller-ray rotation. Gizmo handles carry only
`Raycastable.drag_only()`, so they do not use that path.

### Gizmo constraints

`TransformGizmoSystem` consumes the world-space `hit_point` and `delta_world` supplied by
`DragMove`. It then applies operation-specific constraints:

- an axis handle projects displacement onto its world-space axis;
- a plane handle projects displacement onto a captured world-space two-axis basis;
- a rotation handle currently prefers `ScreenSpace1DSlider`, which has no XR screen delta;
- snapping is applied after translation mapping and constraint resolution.

The planar handle basis is not the same thing as GestureSystem's start projection plane:

```text
current pointer ray
        |
        v
gesture mapping surface       produces a continuous world-space point
        |
        v
gizmo operation constraint    keeps an axis or XY/YZ/XZ components
        |
        v
snap + transform mutation
```

The two stages are not redundant in the current event contract. The generic gesture layer first
creates a point; the gizmo layer then interprets that point according to the selected handle.
However, direct handle-aware mappings could replace this two-stage approximation later.

### Debug plane mismatch

The gizmo debug visualization currently chooses the handle's XY/YZ/XZ normal for planar handles,
but GestureSystem actually uses a plane whose normal is the drag-start ray direction. It can
therefore display a constraint plane while calling it the projection surface. Both the ownership
and the naming are misleading.

## Separate the concepts

The runtime should treat the following as independent dimensions.

### 1. Activation lifecycle

Answers: is this pointer pressed, held, or released?

Owner: `PointerSystem` and `PointerActivations`.

This part already works for desktop and XR.

### 2. Continuation policy

Answers: after a valid `DragStart`, must the pointer remain in contact with the original target?

A possible internal type is:

```rust
enum DragContinuation {
    RequireTargetContact,
    Captured,
}
```

`Captured` retains the original event target until release. It does not by itself say how movement
is calculated.

The policy is primarily a property of the interaction target, not the hardware:

- gizmo handles and panel drag surfaces generally want `Captured` for every pointer type;
- painting, poking, and surface-dependent tools may want `RequireTargetContact`;
- a pointer-level default can remain as a fallback, but "XR" should not imply contact-required.

If this becomes configurable, suggested precedence is:

1. explicit drag behavior on the captured target or its raycastable ancestry;
2. pointer default;
3. system default.

The first implementation can keep this internal and mark built-in gizmo handles as captured
without committing to serialized authoring syntax.

### 3. Current pointer sample

Answers: where is the active pointer ray now, regardless of hits?

```rust
#[derive(Clone, Copy)]
struct PointerRaySnapshot {
    origin_world: [f32; 3],
    direction_world: [f32; 3],
}
```

This must be updated before the scene query and retained even when the hit list is empty. Plausible
homes are `PointerSystem` or `RayCastSystem`; the important contract is lookup by captured pointer
or raycaster during the same frame.

Do not infer the current ray from `RayIntersected`: that recreates the contact dependency we are
trying to remove.

### 4. Drag mapping

Answers: given the current pointer sample and drag-start state, what coordinate or scalar drives
the interaction?

An internal enum would make the current implicit choices visible:

```rust
enum DragMapping {
    ContactHit,
    StartRayPlane {
        point_world: [f32; 3],
        normal_world: [f32; 3],
    },
    ControllerTranslation {
        last_origin_world: [f32; 3],
    },
}
```

This is a useful `GestureState` implementation type, but it should not immediately become a public
or serialized component. It describes generic mappings that do not require knowledge of a gizmo
operation.

`StartRayPlane` is a better name than "screen plane": the same mapping works with a desktop cursor
ray, an XR controller ray, or an autonomous pointer ray. Its start normal is normally the initial
ray direction.

### 5. Tool-specific constraint or mapping

Answers: what does this particular handle mean?

The gizmo knows information that generic gesture recognition does not: operation, pivot, axis,
coordinate space, and planar basis. That information should remain in `TransformGizmoSystem`.

Initially, the gizmo can continue constraining generic `StartRayPlane` output. Later it can consume
the raw pointer sample and use a more direct `GizmoDragMapping`, for example:

```rust
enum GizmoDragMapping {
    AxisClosestPoint,
    HandlePlaneIntersection,
    RingPlaneAngle,
    ScreenSlider,
}
```

These are not continuation policies. Any of them can operate under captured continuation.

The existing `GestureCoordType` is already a partial, gizmo-consumed mapping selector
(`WorldPlane` versus `ScreenSpace1DSlider`), but its name and variants do not cover XR or distinguish
generic gesture mapping from handle constraints. It should be evaluated for replacement or rename
only when tool-aware mappings are implemented.

## Proposed event and state flow

```text
raw mouse/XR input
        |
        v
PointerSystem
  activation edges + pointer identity
        |
        +--------------------------+
        v                          v
current PointerRaySnapshot      RayCastSystem
                                   |
                                   v
                              zero or more hits
        |                          |
        +-------------+------------+
                      v
                 GestureSystem
          capture target at DragStart
          resolve continuation + generic mapping
                      |
                      v
             captured drag update
        pointer ray + mapped point/delta
                      |
                      v
             TransformGizmoSystem
        operation constraint/mapping + snap
                      |
                      v
               transform mutation
```

The captured target remains the renderable selected at `DragStart`; later hits must not retarget
the drag. Current hits are relevant only to `RequireTargetContact` mappings and release/click
qualification.

## Do drag events need raw rays?

Not for the first XR translation fix. `GestureSystem` can use `PointerRaySnapshot` internally and
continue emitting the existing mapped `hit_point` and `delta_world`.

Raw ray data becomes valuable when implementing direct gizmo mappings. At that point, extend the
event without removing compatibility fields:

```rust
DragMove {
    pointer: ComponentId,
    ray_origin_world: [f32; 3],
    ray_dir_world: [f32; 3],
    hit_point: [f32; 3],       // compatibility/generic mapped point
    delta_world: [f32; 3],     // compatibility/generic mapped delta
    screen_pos_px: Option<(f32, f32)>,
    screen_delta_px: Option<(f32, f32)>,
    // existing target fields...
}
```

Adding `pointer` also removes the need for consumers to reverse-map `raycaster` to pointer identity.
This event expansion should be deferred until a consumer needs it; a current-ray lookup is enough
for the first stage.

## Incremental implementation path

### Stage 1: expose hit-independent current rays

1. Retain a normalized `PointerRaySnapshot` for every raycaster that casts this frame.
2. Expose read-only lookup by raycaster (or by pointer through `PointerSystem`).
3. Pass that lookup to `GestureSystem` during its tick.
4. Add tests proving an active XR pointer has a current ray on a zero-hit frame.

This is the essential seam. It does not change interaction behavior by itself.

### Stage 2: captured start-plane mapping for gizmos

1. Resolve built-in gizmo handles to `DragContinuation::Captured`.
2. At `DragStart`, create `DragMapping::StartRayPlane` for desktop or spatial pointers from the
   same initial hit and ray.
3. While held, intersect the current pointer ray with the captured mapping plane even when the hit
   list is empty or contains other objects.
4. Keep emitting events against the initially captured renderable.
5. Preserve current target-contact behavior for interactions that request it.

This stage fixes XR translation arms and planar translation handles without first redesigning the
gizmo math.

### Stage 3: correct visualization and terminology

1. Move projection-surface debug state to the owner of `DragMapping`, or expose the resolved
   mapping state to the visualizer.
2. Name the two possible displays explicitly:
   - `mapping_surface`: the surface used to turn a pointer ray into a drag point;
   - `gizmo_constraint`: the axis or XY/YZ/XZ basis used by the tool.
3. If both are displayed, use distinct colors/labels. Do not draw the constraint plane while
   describing it as the mapping surface.

### Stage 4: direct gizmo mappings

This is an improvement, not a prerequisite for XR capture:

- axis translation: closest points between the pointer ray and gizmo axis;
- planar translation: intersect the pointer ray with the actual handle plane;
- rotation: ring-plane angle or a controller-specific angular mapping;
- scale: axis or uniform scalar mapping.

Every direct mapping needs a defined degeneracy fallback. For example, handle-plane intersection
becomes unstable when the pointer ray is nearly parallel to the plane; the captured start-ray plane
or a screen/controller-space mapping can serve as fallback.

Once direct mappings are stable, generic `StartRayPlane` followed by gizmo projection may remain as
a fallback rather than the primary path.

### Stage 5: public configuration, only if needed

After built-in interactions establish useful defaults, decide whether continuation and mapping need
scene-authored components. Avoid exposing a single enum that recombines both concepts.

Potential authored concepts, if use cases demand them:

- `DragBehavior.capture()` / `DragBehavior.require_contact()`;
- a pointer default continuation policy;
- tool-owned mapping configuration.

## Tests

### Gesture and pointer tests

- A held XR pointer publishes ray snapshots while hitting nothing.
- A captured drag emits `DragMove` after leaving its original renderable.
- A captured drag remains targeted at the original renderable when the ray crosses another object.
- A contact-required drag pauses when the original target is missed.
- Desktop and XR start-plane projection use the same mapping math given the same rays.
- Releasing a captured drag emits one `DragEnd` with the last mapped point.

### Gizmo tests

- XR axis translation continues after leaving the thin arm.
- XR planar translation continues after leaving the square.
- Axis and planar constraints still remove disallowed movement after generic plane mapping.
- Bound-grid snapping receives continued translations and retains the binding.
- Two simultaneous pointers retain independent captured target and mapping state.

### Degeneracy and diagnostics

- A ray nearly parallel to a mapping or handle plane does not produce non-finite or extreme deltas.
- Debug visualization matches the exact plane stored in active mapping state.
- Constraint visualization, if enabled separately, matches the gizmo's captured world-space basis.

## Implemented slice and next step

The first behavioral slice now implements Stages 1-3:

- add a hit-independent current-ray snapshot;
- model captured continuation separately inside active gesture state;
- use the existing start-ray-plane math for desktop and XR gizmos;
- label and render the gizmo diagnostic as the gesture mapping surface.

This is a focused refactor around ray availability and active-drag state, not a prerequisite rewrite
of the entire pointer/gesture/gizmo stack. It fixes the immediate XR interaction problem while
leaving Stage 4's direct handle-aware mappings as an independently testable follow-up.

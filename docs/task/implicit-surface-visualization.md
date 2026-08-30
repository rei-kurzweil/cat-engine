# Task: Implicit surface authoring visualization

Date: 2026-08-30

Status: focused tracker; not implemented

## Outcome and stop condition

Add an editor-settings toggle that reveals the authored controls behind every
`ImplicitSurface` in the active editor scope. The visualization must make the
sampling bounds and individual implicit cube/sphere sources understandable and
make each source selectable through its authored transform.

Stop when an author can enable the overlay, click a wireframe source control,
move/rotate/scale it with the existing transform gizmo, release the gizmo to
trigger exactly one implicit-mesh regeneration, and disable the overlay without
leaving runtime nodes behind. Do not build a general SDF graph editor,
scalar-field volume renderer, or new transform gizmo in this slice.

## What should be visible

For each `ImplicitSurface` under an effective editor root, show:

1. one wireframe box for the authored sampling bounds;
2. one wireframe control for each authored field source, at its resolved
   transform and dimensions; and
3. a visually distinct color for authoring controls so they remain legible
   through the generated surface.

The normal generated implicit mesh remains visible. This overlay explains the
inputs that created it; it does not replace the output with another copy of
the extracted triangles.

The first two source-control shapes are exact mappings to renderables already
owned by the engine:

- `ImplicitCube` -> `RenderableComponent::wireframe_box(...)` (the existing
  wireframe cube/box mesh); and
- `ImplicitSphere` -> `RenderableComponent::wireframe_icosahedron(...)`, with
  sphericalness `1.0` and a small fixed tessellation level.

Do not use a filled translucent sphere/cube for these handles. Use overlay
rendering and modest emissive/color treatment so the wireframes remain readable
when inside the generated surface. The sampling box is informational and
non-selectable. Source wireframes are raycastable/selectable because they are
the editing handles.

`ImplicitCube` field evaluation/public MMS authoring may land in its own
implicit-primitive slice. This visualization system should use a shape enum and
support the cube marker from the outset (or become active as soon as that
component exists), without duplicating or prematurely defining the cube SDF in
the visualization layer.

## Existing seams

- `ImplicitSurfaceSystem` already resolves authored sphere transforms into
  surface-root-local centers/radii and rebakes when its authored fingerprint
  changes.
- `ImplicitSurfaceSystem` already registers the generated mesh AABB in
  `MeshBoundsSystem` as `MeshOutputKind::ImplicitSurface`.
- `BoundsVisualizationSystem` currently filters generated mesh outputs to
  `MeshOutputKind::CombineMesh`, so the ordinary **Show bounds** toggle does
  not yet show an implicit output even though the descriptor exists.
- `CameraVisualizationSystem` provides the closest interaction precedent: it
  unions editor-owned scoped requests, attaches runtime-only markers beneath
  authored transforms, and maps marker hits back to those transforms.
- `CollisionVisualizationSystem` provides the scoped request and cleanup
  precedent for multiple editor owners, but its markers are intentionally
  non-selectable.
- Editor Settings is authored in
  `assets/components/internal/panels.mms`; row presence comes from
  `SettingsPanelConfig`, while live toggle state belongs to
  `EditorContextState` and the corresponding runtime request.
- `GestureSystem` already emits `DragStart`, `DragMove`, and `DragEnd`, and
  `TransformGizmoSystem` already subscribes to those events. `DragEnd` is the
  existing release/completion boundary for this task; no new pointer-release
  event is needed.

## Keep output bounds and authoring controls separate

The existing **Show bounds** control should include
`MeshOutputKind::ImplicitSurface`. That box is the tight AABB of the extracted
mesh and follows the standard generated-mesh bounds path.

The new **Show implicit controls** toggle owns a different box: the explicit
`ImplicitSurface.bounds(...)` sampling domain. Both boxes may be visible at
once because they answer different questions. Do not overload the generic
bounds registry with sampling-domain semantics.

## Runtime system

Add `ImplicitSurfaceVisualizationSystem`, following the request ownership
model used by camera and collision visualization:

```rust,ignore
pub struct ImplicitSurfaceVisualizationRequest {
    pub scope_roots: Vec<ComponentId>,
}

pub struct ImplicitSurfaceVisualizationSystem {
    requests: HashMap<ComponentId, ImplicitSurfaceVisualizationRequest>,
    surfaces: HashMap<ComponentId, ImplicitSurfaceMarkers>,
}
```

The request owner is the `EditorUIComponent`. Enabling the row installs or
updates that owner's effective editor roots; disabling it removes the request.
The union of live requests determines which authored surfaces have markers.
Removing an editor, surface, field source, or request must remove its corresponding
runtime marker state.

Add a focused intent analogous to the existing visualization intents:

```rust,ignore
IntentValue::ImplicitSurfaceVisualizationSet {
    component_id: owner,
    scope_roots,
    visible,
}
```

Route it through the normal signal pipeline and mutation executor. Do not let
the settings click call the visualization system directly.

## Share field resolution with the mesher

The visualization system must not independently reimplement transform
composition, uniform-scale validation, nested-surface boundaries, or sphere
radius semantics.

Extract or publish a read-only resolved authoring snapshot from the implicit
surface ownership layer, equivalent to:

```rust,ignore
pub struct ResolvedImplicitSurfaceControls {
    pub root: ComponentId,
    pub root_model: TransformMatrix,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub fields: Vec<ResolvedImplicitFieldControl>,
}

pub struct ResolvedImplicitFieldControl {
    pub field: ComponentId,
    pub authored_transform: ComponentId,
    pub transform_root_local: TransformMatrix,
    pub shape: ResolvedImplicitControlShape,
}

pub enum ResolvedImplicitControlShape {
    Cube { authored_half_extents: [f32; 3] },
    Sphere { authored_radius: f32 },
}
```

`ImplicitSurfaceSystem` and `ImplicitSurfaceVisualizationSystem` should consume
the same resolver contract. Visualization reads must not sample the field,
run marching cubes, upload a mesh, or cause a rebake by themselves.

Keep the sampling box visible whenever the root bounds and root transform are
valid, even if a field is invalid or extraction fails. Show every individually
valid field control and retain the existing actionable diagnostic for invalid
ones. This lets the overlay help repair a broken surface instead of vanishing
at the first authoring error.

## Marker topology and selection

Every marker subtree must be runtime-only (`Serialize.off()`) and excluded from
authored fingerprints and MMS output.

Sampling bounds marker:

- attach beneath the surface root's effective authored transform;
- render a wireframe cube transformed to `bounds_min..bounds_max`;
- set `Selectable.off()` and avoid raycast participation; and
- update in place when bounds or the outer transform changes.

Field-source marker:

- attach beneath the authored transform that owns the corresponding
  `ImplicitCube` or `ImplicitSphere` whenever possible;
- render cubes with the existing wireframe box mesh, scaled to their authored
  local full extents and then inherited through the authored transform;
- render spheres with the existing wireframe icosahedron mesh, scaled to the
  authored local diameter and then inherited through the authored transform;
- enable raycasting and normal editor selection;
- use a stable marker name such as
  `implicit_surface_visualization_marker`; and
- extend `preferred_scene_selection_transform(...)` so a marker hit resolves
  to that authored transform, as camera markers already do.

Selecting a marker therefore targets the existing authored `T`, not a helper
node and not the leaf implicit-field component. Translation, rotation, and
uniform scale use the existing gizmo operations. Radius/extent editing through
the inspector is deferred, but selecting and manipulating the owning transform
must work in this slice.

Marker creation/removal must not perturb the surface fingerprint or cause a
rebake loop. Add a regression test that enables the overlay, runs multiple
ticks, and observes no additional mesh upload until authored input changes.

## Gizmo edit transaction and regeneration boundary

Do not regenerate the implicit mesh on every `DragMove`. The wireframe control
is the live manipulation preview; the previously extracted mesh remains in
place until the gesture completes.

Use the existing gesture lifecycle as an explicit edit transaction:

1. On `DragStart` for a transform gizmo, resolve the gizmo's current target
   transform. If it owns an implicit field, record its owning
   `ImplicitSurface` as interactively editing.
2. On each `DragMove`, let `TransformGizmoSystem` apply the normal authored TRS
   update. The marker follows that transform immediately, but
   `ImplicitSurfaceSystem` suppresses extraction/upload for the editing surface.
3. On `DragEnd` (pointer/button release), clear the editing flag and commit the
   surface dirty state only after the final transform update is visible.
4. On the following reconciliation, regenerate and replace the implicit mesh
   exactly once from the final authored values.

This can be a narrow implicit-surface edit gate driven from gizmo events; it
does not require inventing a general undo/transaction framework in this slice.
However, keep the boundary explicit enough to be promoted later to a generic
`TransformEditStarted` / `TransformEditCommitted` contract if other expensive
derived systems need the same behavior.

The gate must be keyed by owning surface and support more than one pointer or
gizmo safely (use active gesture identities/counts rather than one process-wide
boolean). A `DragEnd` with no movement should not regenerate. Translation,
rotation, or scale that returns to the original authored value should not
regenerate because the final fingerprint is unchanged.

If the target, field, surface, editor, or request disappears before `DragEnd`,
discard the active edit without touching dead IDs. If a gesture is cancelled by
an existing cancellation/lost-capture path, treat that boundary consistently:
commit once when the authored transform was retained, or restore/cancel without
a rebuild when the transform was reverted. Do not leave a surface permanently
suppressed after an interrupted drag.

## Editor Settings row

Add a settings panel row labeled **show implicit controls**.

Plumbing required:

- `SettingsPanelConfig.show_implicit_surfaces` controls whether the row is
  authored into a particular settings panel;
- `EditorContextState.implicit_surfaces_visible` stores live UI state;
- the authored row carries a distinct payload kind such as
  `ImplicitSurfaceVisibility`;
- clicking it emits `ImplicitSurfaceVisualizationSet` with the effective
  editor roots; and
- the toggle renderer follows the existing camera/collider boolean styling.

The default live state is off. Row presence may default on alongside the other
standard visualization rows.

## Update and cleanup behavior

On each visualization reconciliation:

- discard requests whose owner no longer exists;
- discard dead scope roots;
- discover only `ImplicitSurface` roots inside requested scopes;
- add/remove cube and sphere markers as authored source topology changes;
- update marker transforms and dimensions in place when authored controls
  change;
- remove all markers immediately after the last matching request disappears;
  and
- never serialize, select, raycast, or bound the sampling-box helper itself.

During a gizmo drag the generated mesh intentionally remains at its last
committed state while controls reflect the live authored transform. After
`DragEnd`, the replacement mesh may lag while it rebakes; controls should remain
available during that work.

## Focused implementation order

1. Generalize the implicit input resolver into a shared read-only control
   snapshot without changing extraction output.
2. Add cube/box and sphere/icosahedron wireframe marker creation, request, and
   cleanup tests to `ImplicitSurfaceVisualizationSystem`.
3. Add marker-to-authored-transform scene-hit resolution and selection tests.
4. Add the DragStart/DragMove/DragEnd edit gate and prove one committed rebuild.
5. Add the settings intent, context state, authored row, and click plumbing.
6. Allow `MeshOutputKind::ImplicitSurface` through ordinary generated-mesh
   bounds visualization.
7. Exercise the existing `examples/implicit-surface.mms` scene inside an
   editor root and verify moving a source previews through its wireframe and
   causes exactly one rebake after release.

## Acceptance checks

- The toggle is off initially and appears in a normally configured Editor
  Settings panel.
- Enabling it shows the sampling box and all 144 terrain sphere controls plus
  all five canopy sphere controls in the current example as wireframe
  icosahedra.
- An `ImplicitCube` fixture appears as the existing wireframe box/cube mesh.
- Clicking a source marker selects its authored transform and places the
  existing gizmo there.
- `DragMove` translation/rotation/scale updates the selected wireframe control
  live without extracting or uploading an implicit mesh.
- The matching `DragEnd` regenerates and replaces the implicit mesh exactly
  once using the final authored transform.
- A drag that produces many move events still causes zero uploads before
  release and one upload after release.
- A click/no-op drag or a drag ending at the original fingerprint causes no
  upload.
- Editing a sphere radius updates its marker and rebakes once.
- Enabling/disabling visualization without authored edits causes no mesh
  upload or fingerprint change.
- Nested outer transforms place both the sampling box and sources correctly.
- Invalid fields retain every valid control and do not leave stale helpers.
- Generic **Show bounds** displays the implicit output's tight generated-mesh
  AABB independently of **Show implicit controls**.
- Markers never appear in serialized MMS or authored World-panel rows.
- Removing the surface/editor/request leaves no markers, requests, renderable
  instances, raycast entries, or stale component IDs.

## Deferred

- direct radius handles and numeric property editing;
- visualization of the sampled voxel grid or scalar values;
- smooth-min influence volumes and blend-region shading;
- capsules, arbitrary functions, CSG operators, or a node graph;
- live drag-time partial remeshing or GPU field extraction; and
- per-surface color/style customization for authoring controls.

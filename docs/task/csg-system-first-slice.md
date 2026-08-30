# Task: CSG system first slice

Date: 2026-08-29

Status: active; implementation-ready first slice of
[Mesh CSG operations](mesh-csg-operations.md)

## Outcome and stop condition

Deliver one end-to-end authored `CSG` expression whose single `Subtraction`
operation subtracts one or more ordered static mesh cutters from a static base
mesh. The system retains the authored operands for editing and serialization,
suppresses their normal runtime representations after a successful bake, and
registers one system-owned derived visual and aggregate bound at the CSG root.

Stop this slice when the box-minus-offset-box example renders correctly and
the focused adapter, system, and lifecycle tests below pass. Do not extend the
slice to implicit surfaces or additional boolean operations.

## Public MMS model

The first public form is:

```mms
CSG {
    Subtraction {
        R.cube() // base

        T.position(0.5, 0.0, 0.0).scale(0.6) {
            R.cube() // cutter
        }
    }
}
```

The authored structure has these rules:

- `CSG` is the derived-mesh owner and root.
- Every `CSG` has exactly one operation child. In this slice that child must
  be `Subtraction`.
- A `Subtraction` body contains a base operand subtree followed by one or more
  cutter operand subtrees. Child order is semantic: evaluation is a left fold,
  `(((base - cutter_1) - cutter_2) ...)`.
- Each operand subtree must resolve to exactly one eligible
  `RenderableComponent`. An empty operand or a subtree containing multiple
  renderables is an authoring error rather than an implicit merge.
- Transform containers within an operand subtree are allowed. The effective
  world transform of its one renderable is converted into CSG-root-local space
  before evaluation.
- A nested `CSG`, nested operation, or operation outside a `CSG` is invalid.

Reserve the names `Union` and `Intersection` in the design, but do not add
them to the component registry or runtime configuration in this slice.

## Operand eligibility

An operand is eligible only when its resolved renderable has CPU mesh data and
the mesh is:

- finite;
- static;
- an indexed triangle list;
- closed and consistently oriented; and
- free of skinning and morph-target data.

Validate index bounds and triangle grouping before entering `csgrs`. Validate
closedness by requiring each undirected triangle edge to occur exactly twice
with opposing directed uses. Reject degenerate or non-finite triangles with an
actionable error.

The following are explicitly outside the first slice:

- imported multi-primitive scenes;
- `CombineMesh` outputs;
- skinned or morphed meshes;
- dynamic/per-frame meshes;
- nested `CSG` expressions;
- implicit-surface output; and
- union, intersection, xor, or other boolean operations.

An imported static primitive is eligible only when it exposes one unambiguous
CPU triangle mesh and satisfies the same adapter contract. Traversing or
combining a multi-primitive imported scene is not part of this task.

## Authored and derived ownership

`CSGComponent`, `SubtractionComponent`, all operand transforms, and all source
renderables are authored ECS state. Their normal `Component::to_mms_ast(...)`
implementations must round-trip the expression in authored child order.

The evaluated mesh, GPU instance handle, cached fingerprint, local bounds, and
suppression bookkeeping are system state owned by `CsgSystem`. They must never
appear in MMS serialization. The derived visual is registered against the CSG
root so selection/raycast results identify the authored owner rather than a
hidden runtime component.

The output uses the base operand's `MaterialHandle`. Material changes on a
cutter do not affect output selection, while a base material change triggers a
rebake/update. Boolean-generated vertices receive placeholder UVs of
`[0.0, 0.0]`; this slice makes no UV preservation promise.

## Backend verification gate

Pin the backend in the root `Cargo.toml`:

```toml
csgrs = { version = "=0.20.1", default-features = false, features = ["f64", "delaunay"] }
```

Keep all `csgrs` types behind an internal adapter. Do not enable default,
import/export, SDF, metaball, physics, or CAD-oriented features directly.
`f64` is the calculation precision at the backend boundary; validate conversion
back to the renderer's `f32` representation.

Before accepting the dependency:

1. Run `cargo tree -i csgrs` and record the resolved inverse dependency tree in
   the implementation record appended to this document.
2. Run `cargo tree -e features -i csgrs` and record the enabled feature paths.
3. Confirm that the lockfile contains no unexpected dependency expansion. The
   known optional physics-conversion impact from the `f64` selection is a
   deliberate accept/reject decision, not something to overlook.
4. If the actual dependency impact is unacceptable, stop before registering
   public MMS components and either obtain a narrower upstream feature split or
   reassess the backend.

## Internal adapter contract

Add a small engine-private adapter, for example
`src/engine/graphics/csg_mesh.rs`, with a boundary equivalent to:

```rust
fn mittens_to_csgrs_mesh(
    mesh: &CpuMesh,
    operand_to_root: TransformMatrix,
) -> Result<csgrs::mesh::Mesh<()>, CsgMeshError>;

fn csgrs_to_mittens_mesh(
    mesh: &csgrs::mesh::Mesh<()>,
) -> Result<CpuMesh, CsgMeshError>;
```

Input conversion must validate the operand contract first, apply
`inverse(root_world) * operand_world` to positions, transform normals with the
correct inverse-transpose rule, preserve handedness/winding, and convert
positions to `f64`. Non-invertible transforms fail validation. Do not route
through STL, DXF, Bevy, Parry, Rapier, or any backend import/export helper.

Output conversion must ask the selected `delaunay` path for triangles, produce
a stable triangle/vertex ordering for identical inputs, reject non-finite or
out-of-`f32` values, and recompute outward renderer normals from final triangle
geometry. Build a plain `CpuMesh` with no skin/morph data and placeholder UVs.
An empty, valid result is a successful cached bake with no derived visual and
no aggregate bound; its source operands remain suppressed because they are
correctly represented by empty geometry. It is not a backend error.

Use typed adapter errors carrying the operand role/index and validation cause
so `CsgSystem` can log a message such as “CSG root X cutter 2 has index 41 but
only 24 vertices,” rather than a generic bake failure.

## Reversible source suppression

The current `CombineMesh.keep_transforms()` behavior establishes that authored
sources may remain in `World` while a derived visual represents them. The CSG
slice always retains sources and therefore needs a reversible form of that
precedent, not `RenderableSystem::suppress_renderable()` as a one-way removal.

Generalize suppression around an explicit derived owner. A suitable internal
contract is:

```text
suppress(owner, source)
release(owner, source)
release_all(owner)
owner_of(source) -> Option<ComponentId>
```

The exact type may live in `RenderableSystem` or in a small shared derived-mesh
ownership registry, but one transition must consistently cover:

- `VisualWorld` registration/instance removal;
- `RenderableSystem` discovery and pending upload state;
- source `BoundsComponent` participation and aggregate mesh bounds;
- `BvhSystem` membership; and
- `RayCastSystem` eligibility/index state.

Suppressed sources remain fully present and serializable in `World`. While an
owner is active, later renderable registration or pending-upload completion
must not make a source visible again. Releasing the owner must rediscover and
register the retained source, restore its bounds and raycast/BVH participation,
and apply its current transform/material rather than stale cached state.

Removing a source, reparenting it outside the owned expression, reparenting it
to another owner, or removing the `CSG` root must release the old ownership
relationship. A retained source that leaves the expression becomes active
again. Root cleanup removes the derived `VisualWorld` instance, aggregate
bounds, raycast/BVH entry, fingerprints, and all suppression claims.

Do not suppress any source until the first complete bake has successfully
converted, uploaded, and registered its replacement.

## `CsgSystem` evaluation and cache

Add `CsgSystem` beside `CombineMeshSystem` and wire it through
`SystemWorld::prepare_render(...)` after imported CPU meshes are available and
before ordinary pending renderables are flushed. Add a distinct
`MeshOutputKind::Csg` for aggregate bounds.

For every root, reconciliation performs one transaction:

1. Discover the one operation and ordered operand subtrees from live authored
   child order. Reject malformed topology.
2. Resolve exactly one eligible source renderable per operand.
3. Compute each source's `operand_to_root` matrix.
4. Build a fingerprint from operation kind, ordered source IDs, CPU mesh
   content/revision, operand-to-root transforms, topology/order, and the base
   material. A mesh handle alone is insufficient if its CPU contents can be
   replaced in place.
5. If only the CSG root's world transform changed, update the existing output
   model and aggregate world bounds without running the boolean again.
6. Otherwise convert operands, evaluate ordered `difference`, convert the
   result, upload it, and register one derived visual and local bound.
7. Atomically replace an older output only after all prior steps succeed, then
   reconcile the source suppression set.

Cache at least the successful fingerprint, root model, derived instance
handle, derived CPU mesh handle, local bound, ordered source set, and base
material. Child attachment/removal/reparenting and order changes must become
visible to reconciliation even if no transform signal fires.

### Failure policy

- On the first failed bake, leave all sources active and emit one actionable
  diagnostic for the current failing fingerprint.
- On a failed rebake, keep the last known-good derived output and keep the
  sources represented by that output suppressed. Never remove good geometry
  before its replacement is ready.
- Avoid logging the same unchanged failure every frame; log again after the
  expression fingerprint changes.
- If ownership changes make the last known-good output no longer a valid
  representation of a removed/reparented source, release affected sources and
  remove the stale output rather than claiming ownership of unrelated content.

## Component and registry work

Add serializable marker components for `CSG` and `Subtraction` under
`src/engine/ecs/component/`, export them from `component/mod.rs`, and register
only these public names through:

- `src/scripting/component_registry.rs`;
- `src/scripting/runtime_config.rs`; and
- the existing component-expression serialization path.

Add the system module/export, `SystemWorld` field, registration hooks, render
preparation reconciliation, transform/update notification, and cleanup hooks
using the existing `CombineMeshSystem` seams as the starting inventory. The
new suppression lifecycle must also replace or generalize CombineMesh's direct
call to the one-way suppression helper without changing CombineMesh's public
collapse/`keep_transforms()` behavior.

## Rebuild matrix

| Change | Boolean recompute | Output model/bounds update | Source ownership reconcile |
| --- | --- | --- | --- |
| Operand CPU mesh data/revision | yes | yes | if source set changed |
| Operand transform | yes | yes | no |
| Cutter/base order | yes | yes | yes |
| Operation/operand topology | yes | yes | yes |
| Base material | yes | visual updated | no |
| Cutter material | no | no | no |
| CSG-root transform only | no | yes | no |
| Source removed/reparented | yes or stale output removed | yes | yes |
| CSG root removed | no | output removed | release all |

## Verification plan

### Adapter tests

- Convert the built-in cube and prove valid triangle count, closed edges,
  outward winding, and outward normals.
- Subtract a translated/scaled cube from another cube in root-local space and
  verify representative inside/outside points and output bounds.
- Run identical conversion/evaluation repeatedly and compare canonical output
  vertices/indices for determinism.
- Reject an index outside the vertex array, an index count not divisible by
  three, non-triangle topology, degenerate/open geometry, and non-finite input.
- Reject meshes with either skin data or any morph target.
- Reject non-invertible/non-finite operand transforms and output values that
  cannot be represented as finite `f32`.

Record observed results for disjoint, fully consuming, and near-coplanar box
cutters. These fixtures characterize `csgrs 0.20.1`; they do not promise
general numerical robustness. A fully consuming cutter must exercise the
intentional empty-result policy.

### System tests

- Discover base and multiple cutters in authored order and prove left-fold
  evaluation uses that order.
- Apply nested operand transforms relative to a transformed CSG root and prove
  the output remains root-local.
- Register exactly one derived visual and one `MeshOutputKind::Csg` aggregate
  bound for a successful root.
- Select the base material even when cutters have different materials.
- Change only the root transform and prove the adapter/boolean call count does
  not increase while the model and world bounds do update.
- Reject zero/multiple operations, zero cutters, ambiguous operand subtrees,
  nested CSG, and unsupported source kinds with actionable diagnostics.

### Lifecycle tests

- Serialize a successful expression and prove the authored CSG, operation,
  transforms, and source renderables remain while no derived mesh is emitted.
- Keep sources visible before the first successful bake, then suppress their
  visual, pending, bounds, BVH, and raycast state after success.
- Reparent a source away from its owner and prove that source is restored with
  its current state.
- Remove a CSG root and prove output cleanup plus restoration of all retained
  sources.
- Mutate source mesh data, operand transform, child order, topology, and base
  material and prove the appropriate rebake occurs.
- Force a failed first bake and prove sources remain visible.
- Force a failed rebake and prove the last known-good output and suppression
  remain intact without duplicate per-frame diagnostics.

## Example

Add a small live MMS example, with the normal `.mms`/`.rs` pairing if required
by the example harness. It should show a large box with a smaller, visibly
offset box removed so the new interior faces and asymmetric cut are obvious.
Keep the scene free of implicit surfaces, imported models, `CombineMesh`, and
editor-heavy content.

## Implementation order

1. Run and record the dependency/feature-tree gate.
2. Implement and pass the adapter validation/conversion fixtures.
3. Add the two authored components and their MMS round-trip coverage.
4. Generalize reversible derived ownership and cover it with focused unit
   tests before using it from CSG.
5. Add `CsgSystem`, transactional last-known-good behavior, aggregate bounds,
   and the rebuild matrix tests.
6. Add and visually validate the offset box subtraction example.
7. Run the focused adapter/system/lifecycle tests and stop the slice.

## Follow-up work

- Public `Union` and `Intersection` operations.
- `CombineMesh` and imported multi-primitive operands.
- Baked `ImplicitSurface` operands and terrain authoring.
- UV/material propagation across split polygons.
- Skinned, morphed, dynamic, open, or non-manifold inputs.
- Broader tolerance controls or numerical-robustness guarantees.

## Implementation record

Fill this section during implementation with:

- the resolved `cargo tree -i csgrs` output/summary;
- the enabled feature-tree summary and dependency-impact decision;
- observed disjoint, fully consuming, and near-coplanar behavior;
- the final example path; and
- the focused test commands used for acceptance.

### 2026-08-29 backend gate

- Authored the example first at
  `examples/constructive-solid-geometry.mms`, as requested.
- Attempting to resolve the required exact dependency with
  `cargo tree -i csgrs` failed before an inverse tree could be produced:
  `csgrs v0.20.1` requires `core2 = "^0.4"`, but the only matching release,
  `core2 v0.4.0`, is yanked.
- Because a fresh lockfile cannot resolve the required backend, neither the
  enabled feature tree nor an acceptable lockfile-impact comparison can be
  produced. Per the backend verification gate, the dependency addition was
  removed again and public MMS components were not registered. The repository
  therefore remains buildable while the backend pin or upstream packaging is
  reassessed.

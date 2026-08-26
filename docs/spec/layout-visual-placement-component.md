# `LayoutVisualPlacementComponent`

`LayoutVisualPlacementComponent` is runtime metadata that lets layout place a
bounded renderable subtree without changing its authored transform or inserting
another transform into the component forest.

It is a direct child of the authored `TransformComponent` that roots the visual
content:

```text
styled layout-item Transform
├── Style / Text
└── visual-root Transform
    ├── LayoutVisualPlacementComponent
    └── renderable subtree
```

The component is produced and owned by `LayoutSystem`. It is not an MMS
component and is not authored or serialized.

## Data contract

The intended shape is:

```rust
pub struct LayoutVisualPlacementComponent {
    /// Aggregate visual AABB before layout placement, expressed in the
    /// visual root's parent coordinate space.
    pub source_bounds_parent_local: Aabb,

    /// Layout-owned correction in that same parent coordinate space.
    pub translation_parent_local: [f32; 3],
}
```

The exact Rust field names may follow local conventions, but their coordinate
space and ownership are normative.

`source_bounds_parent_local` includes the visual root's authored translation,
rotation, and scale plus all eligible descendant transforms. It excludes any
previous `translation_parent_local` supplied by this component.

The component stores no authored transform state and does not copy or replace
the `TransformComponent`'s translation, rotation, or scale.

## Composition contract

`TransformSystem` reads at most one direct
`LayoutVisualPlacementComponent` child while resolving a transform. The
effective local matrix is:

```text
effective_local = translation(translation_parent_local) * authored_local
matrix_world = parent_world * effective_local
```

The layout translation is applied outside the authored local matrix. It is
therefore expressed in parent-local axes and is not scaled or rotated by the
authored transform.

All ordinary spatial consumers continue to use the resolved world matrix:

- rendering;
- descendant transform propagation;
- bounds/BVH updates;
- raycasting and collision; and
- editor visualization.

Those consumers do not interpret the placement component independently.

## Measurement contract

Intrinsic visual measurement must ignore the placement translation. It walks
the authored transform chain, measures cached renderable AABBs, and returns a
pre-placement aggregate in the visual root's parent coordinate space.

Conceptually, the layout pass carries:

```rust
struct VisualContentMeasurement {
    bounds_parent_local: Aabb,
    content_root: ComponentId,
}
```

Layout uses the complete AABB—not only its width and height—to compute the
placement. For centered horizontal and bottom vertical alignment:

```text
offset_x = center_x(target_content_region) - center_x(source_bounds)
offset_y = bottom_y(target_content_region) - min_y(source_bounds)
```

Mittens layout extends downward along local negative Y. The resolved target
region and edge calculation must follow that convention.

Ignoring the existing placement during measurement is required. Including it
would feed the previous pass's correction into the next measurement and cause
drift.

## Ownership and lifecycle

- `LayoutSystem` is the only writer and lifecycle owner.
- `TransformSystem` is the only system that interprets it as transform input.
- A visual-root transform may have at most one placement component.
- Layout creates or updates it after a successful bounded-content measurement.
- Layout removes it when the transform is no longer the selected visual root,
  its owning styled item leaves layout, or its bounds become unavailable.
- Creation, update, and removal invalidate the transform's resolved world
  matrix and all descendant spatial state.
- An unchanged layout pass must reuse the existing component rather than add a
  duplicate.

The component must be applied early enough that rendering and spatial systems
observe the corrected matrix in the same frame as the completed layout pass.

## Component-tree behavior

This is a real ECS metadata component, analogous to `BoundsComponent`, but it
is not a transform and does not change transform ancestry. No authored content
is reparented.

The runtime label should use the internal `__layout_visual_placement` name.
Authored serialization, layout-child discovery, and presentational-content
selection must ignore it. Low-level runtime inspection may still reveal the
component; code must not rely on authored `children()` containing only authored
nodes.

Nested styled transforms are layout boundaries. An ancestor layout item must
not attach placement metadata to, or measure through, a nested styled item.

## Authored-transform interaction

Authored systems and animation continue to update the normal
`TransformComponent`. They neither read nor compensate for layout placement.
On the next transform resolution, their new authored matrix is composed with
the current layout translation.

Layout placement never rescales or rotates content. Features that fit content
into a target size remain the responsibility of `FitBounds` or explicit
authored transforms.

## Failure behavior

- No measurable bounds: remove stale placement metadata and use ordinary
  authored transform behavior.
- No unambiguous visual content root: do not mutate any authored transform;
  report the item as unsupported for automatic placement in diagnostics.
- Duplicate placement components: treat this as an engine invariant violation;
  do not compose multiple layout translations.
- Non-finite bounds or offsets: reject the placement update and retain ordinary
  authored transform behavior.

## Required tests

- A scaled origin-centered cube is centered and bottom-aligned without authored
  translation.
- An off-center AABB is placed from its actual `min`, `max`, and center.
- Repeated layout passes produce the same placement without drift.
- Animated authored TRS remains authored and composes with stable layout
  placement.
- Removing layout eligibility removes the component and its spatial effect.
- Serialization omits the component and round-trips only authored TRS.
- Nested styled transforms receive independent placement ownership.

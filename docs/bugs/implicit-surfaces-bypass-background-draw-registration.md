# Implicit surfaces bypass background draw registration

Date: 2026-09-03

Status: open; root cause confirmed, fix not yet implemented

## Summary

An `ImplicitSurface` authored beneath `BG.occlusion_and_lighting()` is not
registered as a background instance. It is drawn through the normal foreground
draw lists instead of the dedicated background occluded-and-lit pass.

This makes implicit cloud dressing behave differently from ordinary renderables
authored with `R.cube()`, `R.sphere()`, and similar `R.*` forms under the same
background subtree. In particular, it can participate in foreground depth and
scene-color ordering when it should be isolated to the background stage.

## Reproduction

1. Load `examples/rough-transmission.mms`.
2. The cloud instances are descendants of `BG.occlusion_and_lighting()` and are
   authored through `ImplicitSurface` in
   `assets/components/backgrounds/cloud.mms`.
3. Compare their draw behavior with an ordinary `R.cube()` placed under the
   same `BG` block.
4. The ordinary renderable enters the background-occluded-lit list. The baked
   implicit surface instead enters the normal foreground list.

## Expected behavior

Every render-producing component should inherit the nearest ancestor
`BackgroundComponent` consistently:

- a descendant of `BG {}` belongs in the plain background draw list;
- a descendant of `BG.occlusion_and_lighting() {}` belongs in the background
  occluded-and-lit draw list; and
- background instances remain excluded from the foreground opaque,
  transmission, and ordinary transparency lists.

The registration path must retain the existing material, opacity, lighting, and
transform behavior of implicit surfaces.

## Actual behavior and root cause

Normal `RenderableComponent` instances are registered by `RenderableSystem`.
Before calling `VisualWorld::register`,
`resolve_effective_renderable_style` walks ancestors and reads
`BackgroundComponent`, passing the resulting `background` and
`background_occluded_lit` flags to the `VisualInstance`.

`ImplicitSurfaceComponent` bypasses `RenderableSystem`. During
`SystemWorld::prepare_render`, `ImplicitSurfaceSystem::reconcile_and_build`
extracts and uploads its mesh, then directly calls `VisualWorld::register`.
That call currently hard-codes both background flags to `false`:

```rust
visuals.register(
    root,
    /* mesh, transform, color, opacity, ... */
    false, // background
    false, // background_occluded_lit
    false, // overlay
    /* ... */
);
```

Consequently, the visual-world draw-cache classifier cannot place the instance
in `background_order` or `background_occluded_lit_order`; it falls through to
one of the foreground lists.

## Likely fix seam

Give `ImplicitSurfaceSystem` the same effective style resolution required for
its supported visual attributes, at minimum a shared helper that resolves the
nearest ancestor `BackgroundComponent`. Pass those resolved flags to
`VisualWorld::register` when the baked output is first created.

The update path also needs review: if an implicit surface's background ancestry
changes after it has been registered, `VisualWorld` needs an API to update the
two background flags and invalidate the draw cache, or the implicit output must
be re-registered. Do not duplicate `RenderableSystem`'s full style walk without
first deciding which visual style rules are intentionally shared.

## Acceptance

- [ ] A plain implicit surface beneath `BG {}` appears in
      `VisualWorld::background_order()` and not a foreground draw list.
- [ ] A plain implicit surface beneath `BG.occlusion_and_lighting() {}` appears
      in `background_occluded_lit_order()` and not a foreground draw list.
- [ ] The existing implicit-surface refraction behavior remains correct outside
      a background subtree.
- [ ] Background implicit surfaces do not occlude foreground geometry after the
      renderer clears depth between the background and foreground stages.
- [ ] Focused automated coverage exercises both ordinary and
      occluded-and-lit background ancestry.

## Relevant code

- `src/engine/ecs/system/implicit_surface_system.rs`
- `src/engine/ecs/system/renderable_system.rs`
- `src/engine/graphics/visual_world.rs`
- `src/engine/graphics/vulkano_renderer.rs`

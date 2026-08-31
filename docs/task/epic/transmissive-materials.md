# Transmissive materials: refraction and rough transmission

Date: 2026-08-30

Status: proposed planning epic

## Purpose

Add a shared transmissive-material architecture with two intentionally distinct material models:

- sharp screen-space refraction; and
- rough transmission, which refracts and filters the scene behind the surface.

Both models consume the same per-view scene-color snapshot, but they have separate fragment shaders,
pipeline identities, options, and validation. They must not become conditionals inside the toon
fragment shader.

This epic ties together the material-model change, renderer pass boundary, scene-color resource,
authoring inputs, desktop/XR behavior, and focused visual/performance proof. It is a planning epic,
not one implementation patch.

Related current work:

- [ECS and MMS authoring contract](../transmissive-materials-ecs-mms-authoring-contract.md)
- [Implemented render-to-texture bridge](../../spec/render-to-texture.md)
- [Animated shader-material inputs](../animated-shader-material-inputs-mms-animation-system.md)
- [Single-layer transparency contract](../single-layer-transparency-depth-write-contract.md)
- [Dedicated mirror shader precedent](../mirror-dedicated-shader-refactor.md)
- [Base render-pipeline diagram](../../spec/render-graph-pipeline.svg)

## Decision summary

1. `MaterialHandle` remains resource identity. It is not the semantic material-model enum.
2. The material definition becomes a composition of a vertex-stage family, a fragment-stage model,
   and constrained render state rather than a name for one monolithic pipeline.
3. The fragment stage includes `Transmissive(TransmissiveModel)`, whose two variants are
   `Refraction` and `RoughTransmission`.
4. Refraction and rough transmission use separate fragment shaders. They may share compatible
   vertex outputs, scene-color descriptors, and common transmission inputs.
5. Static versus cached-deformed/skinned geometry is an orthogonal renderer pipeline variant. Do
   not expose `SKINNED_REFRACTION` and `SKINNED_ROUGH_TRANSMISSION` as authored material models.
6. Every active render view captures its full viewport after background, opaque, and cutout draws,
   before any transmissive draw.
7. The existing runtime-texture bridge is reused where its resource machinery fits, but its
   published, stable, one-frame-delayed `TextureHandle` path is not the source sampled by
   transmission. Transmission needs a same-frame renderer-internal snapshot.
8. Screen-edge behavior is defined. Invalid UVs never produce transparent or uninitialized holes;
   the first slice clamps/fades to a valid scene-color sample. Accurate off-screen content is a
   later guard-band or ray-traced feature.

## What “sample the entire screen” means

Yes: each fragment of a transmissive object samples from a texture covering the entire active
render viewport, not a texture cropped to that object's screen-space bounds.

For a centered object, a refracted coordinate can move outside the object's silhouette and still
land on a valid scene-color pixel. That background color is then shown inside the object's
silhouette. Crossing the object's own screen bounds therefore does not create a gap.

The hard boundary is the captured viewport:

```text
captured viewport
+------------------------------------------------+
| source pixel                                   |
|      *                                         |
|       \ refracted lookup                       |
|        \                                       |
|         v        transmissive object           |
|                 +----------------+             |
|                 | shaded fragment|             |
|                 +----------------+             |
+------------------------------------------------+

valid: source pixel is outside the object's silhouette but inside the viewport
invalid/unavailable: the refracted lookup points beyond the viewport edge
```

Every covered fragment still writes a result. “Missing” data only occurs when the desired lookup
leaves the captured viewport or asks for information screen-space rendering never captured. The
first implementation must use a sampler with clamp-to-edge and an explicit edge-validity/fade
policy, so these cases degrade to a valid but approximate color rather than black, transparency,
or undefined memory.

This has unavoidable screen-space limitations:

- content outside the camera viewport does not exist in the snapshot;
- a very large refraction offset near a screen edge stretches or fades the last available pixels;
- geometry hidden from the original camera cannot be recovered by bending a ray toward it;
- the opaque snapshot does not contain later transparent or transmissive surfaces;
- two overlapping transmissive objects do not recursively refract one another in the first slice;
- desktop, each XR eye, and each mirror/capture view need their own correctly oriented snapshot.

An oversized guard-band render could provide real pixels beyond the displayed viewport, but its
fill-rate and memory cost—especially for two XR eyes—keeps it out of the first slice.

## Composable material program

In Vulkan terms, a material does not normally own two pipelines. It contributes two programmable
shader stages—vertex and fragment—to one resolved graphics pipeline. The renderer combines those
stages with vertex layout, depth/blend/raster state, attachment formats, MSAA, and clipping state,
then caches the resulting graphics pipeline.

The target semantic shape is approximately:

```rust
pub struct Material {
    pub vertex: VertexStageFamily,
    pub fragment: FragmentStage,
    pub state: MaterialRenderState,
}

pub enum VertexStageFamily {
    MeshSurface,
    GridSurface,
}

pub enum FragmentStage {
    Toon(ToonOptions),
    Unlit(UnlitOptions),
    EmissiveToon(EmissiveToonOptions),
    Mirror(MirrorOptions),
    Transmissive(TransmissiveModel),
}

pub enum TransmissiveModel {
    Refraction(RefractionOptions),
    RoughTransmission(RoughTransmissionOptions),
}
```

`VertexStageFamily::MeshSurface` is a logical, interoperable vertex component. The renderer resolves
it to the concrete static or cached-deformed shader module for the instance being drawn. Both
concrete modules must emit the same versioned mesh-surface varying interface, allowing compatible
fragment stages to be exchanged without creating authored `SKINNED_*` materials.

The stage boundary is typed rather than “any vertex path string can be paired with any fragment
path string.” Each stage declares:

- a stable input/output interface identity;
- required descriptor/binding capabilities;
- required vertex attributes or resolved deformation source; and
- compatible render-state/phase constraints.

Material registration or pipeline resolution rejects mismatched varyings, missing scene-color
bindings, unsupported vertex layouts, and invalid phase/state combinations before drawing. The
first slice only needs built-in stage families and interfaces; arbitrary user shader reflection,
dynamic shader loading, and a general plug-in ABI remain out of scope.

Exact storage names may adapt to the material-instance work, but these ownership rules are fixed:

- the vertex-stage family owns geometry-to-varying behavior;
- the fragment-stage enum chooses surface shading behavior;
- the resolved Vulkan graphics pipeline is a cache product, not material identity;
- `MaterialHandle` identifies a registered definition or instance;
- the renderable/import path explicitly records the concrete static versus cached-deformed geometry
  variant, and the renderer selects that vertex module independently of the material;
- a material instance owns its transmission parameters;
- a per-view descriptor supplies scene color and viewport data;
- an ordinary authored base texture remains distinct from the renderer-owned scene-color input.

The current built-in handle constants remain compatibility identities while material definitions
are introduced. Avoid growing the public handle list for every combination of vertex stage,
fragment stage, skinning, clipping, emissive extraction, and transparency phase.

### Pipeline resolution

The renderer resolves and caches a pipeline from a key approximately like:

```text
vertex-stage family
+ concrete geometry variant (static or cached-deformed)
+ fragment stage
+ material render state / render phase
+ clip variant
+ target formats and MSAA
```

Changing only `Toon` to `Refraction` exchanges the fragment component while retaining the
compatible mesh-surface vertex interface. Changing an instance's explicitly recorded geometry
variant from static to cached-deformed exchanges the concrete vertex module while retaining the
same fragment stage. Pipeline resolution validates that a cached-deformed variant owns a valid
deformation-cache range. Neither operation requires a new authored material-model name for the
cross-product.

### Common transmission inputs

Start with the smallest shared set needed by both shaders:

- index of refraction (`ior`, finite and greater than or equal to `1.0`);
- effective thickness or distortion distance;
- tint/color attenuation;
- normal influence or refraction strength; and
- edge fade width.

`RoughTransmissionOptions` additionally owns roughness. Roughness `0` should visually converge on
the sharp refraction result closely enough for a comparison fixture, without requiring both models
to compile to the same shader.

Defer absorption distance, dispersion, normal maps, back-face thickness reconstruction, and
artist-selectable sample counts until the base paths are correct and measured.

## Shader separation

Provide dedicated fragment shader assets, for example:

- `assets/shaders/refraction-mesh.frag`
- `assets/shaders/rough-transmission-mesh.frag`

They may share a small include/generated interface later if the shader build supports one, but one
shader must not branch between the two models per fragment.

Both paths:

1. derive screen UV from the current fragment and active viewport;
2. compute a refracted lookup from view direction, transformed normal, IOR, and effective thickness;
3. apply the same coordinate orientation and edge-validity convention; and
4. tint/attenuate the sampled result.

Sharp refraction samples the scene-color base level at the refracted coordinate. Rough transmission
samples a filtered neighborhood around that coordinate. The first rough path should use a
renderer-built mip/blur pyramid and select LOD from roughness and effective thickness; it should not
take an unbounded number of per-fragment samples.

Separate shader paths are still compatible with shared descriptor layouts and shared static or
cached-deformed vertex shaders. “Separate refraction and rough-transmission shaders” means separate
material shading behavior, not duplicated skinning/deformation work.

## Render-graph and image lifecycle

The current renderer records background, opaque, cutout, single-layer transparency, multi-layer
transparency, and overlay in one dynamic-rendering scope. Transmission introduces a deliberate
boundary:

```text
background -> opaque -> cutout
                       |
                       v
              resolve/copy scene color
                       |
                       +--> mip/blur pyramid (when rough transmission is visible)
                       |
                       v
       refraction / rough transmission
                       |
                       v
 ordinary transparent -> overlay -> post-process/present
```

Implementation rules:

- End or otherwise establish a legal Vulkan synchronization boundary after cutout.
- Resolve MSAA before sampling; the transmissive shaders sample a single-sample image.
- Keep source color in the correct color space for lighting/filtering. Do not blur encoded sRGB
  values as though they were linear.
- Transition the snapshot to sampled-read usage, then reopen the destination color attachment with
  `LOAD` for later phases.
- Never sample the same subresource while it remains bound for color writes.
- Allocate/reuse targets per view extent, format, MSAA mode, and frames-in-flight requirements.
- Build the rough-transmission pyramid only when a visible rough-transmission draw needs it.
- Keep the snapshot renderer-internal. Do not route the critical same-frame sample through
  `TextureComponent.render_image` or its frame-boundary publication swap.

The existing `RenderToTextureSystem` is still useful precedent for stable producer/consumer
ownership. It does not currently orchestrate this new pass boundary, and extending its selector
registry is not a prerequisite for the first transmissive material.

## Render ordering and depth contract

Create a dedicated transmissive phase after opaque/cutout and before ordinary transparent phases.
For the first slice:

- transmissive surfaces depth-test against opaque/cutout depth;
- they do not write depth;
- all transmissive surfaces sample the same immutable opaque scene snapshot;
- their own tint/alpha blends into the live destination color;
- the first fixture keeps transmissive panels non-overlapping; view-dependent compositing order for
  overlapping transmissive surfaces remains follow-up work, and recursive transmission is not
  claimed;
- ordinary transparent content is not visible through transmission unless a later design adds a
  pre-transmission transparent subset.

This phase must integrate with existing clip/render-stream behavior deliberately rather than being
silently treated as `transparent_single`.

## Execution plan

### Phase 0: material and render-stream contract

- [ ] Inventory every place that matches built-in `MaterialHandle` values, including pipeline
      routing, emissive classification, glTF material selection, batching, clipping, and tests.
- [ ] Replace the current path-pair placeholder with a material definition composed from a typed
      vertex-stage family, fragment stage, and constrained render state.
- [ ] Introduce `FragmentStage::Transmissive(TransmissiveModel)` and its common validated inputs.
- [ ] Define the shared mesh-surface varying interface and the internal concrete static versus
      cached-deformed vertex variants so authored transmission does not encode skinning.
- [ ] Add registration/pipeline-resolution validation for stage interfaces, descriptor
      capabilities, vertex inputs, and render-state/phase compatibility.
- [ ] Define material-instance identity, dirtying, batching, and MMS serialization in coordination
      with the animated shader-material task.
- [x] Define the dedicated transmissive render-stream phase and its depth/sort policy.

Exit gate: compatible vertex and fragment components can be exchanged through one typed interface;
a registered material can express either transmission model without adding a `SKINNED_*` public
material model; and renderer routing has one authoritative pipeline-resolution path.

### Phase 1: same-frame per-view scene color

- [ ] Split the current rendering scope after opaque/cutout at a legal Vulkan boundary.
- [ ] Allocate a full-viewport, sampleable, single-sample scene-color snapshot.
- [ ] Resolve/copy the opaque scene into it and reopen destination color with preserved contents.
- [ ] Bind scene color as a per-view input, not as each object's authored base texture.
- [ ] Cover resize, MSAA on/off, post-processing on/off, frames in flight, and image transitions.
- [ ] Prove a diagnostic shader can sample pixels outside its own silhouette anywhere within the
      viewport.

Exit gate: same-frame opaque scene color can be sampled while drawing later geometry, with no
feedback validation errors and no one-frame ghosting.

### Phase 2: sharp refraction

- [x] Add the dedicated refraction fragment shader and pipeline routing.
- [x] Add IOR, effective thickness/strength, tint, and edge-fade inputs.
- [x] Support both static and cached-deformed geometry through renderer-selected vertex variants.
- [x] Define front-face/back-face behavior for closed meshes; start with a documented single-surface
      approximation if true entry/exit thickness is not yet available.
- [x] Clamp/fade invalid UVs so the material never reveals black or transparent gaps.
- [x] Add centered panel fixtures whose lookups can visibly cross their silhouettes.

Exit gate: the fixture shows a sharp displaced background across the whole lens, including samples
originating outside its silhouette, and camera/object motion has no previous-frame trail.

Current first-visual-slice status: the desktop window path captures the Bloom/post-processing
`main_color` target after background, opaque, and cutout draws, then reopens the live attachment and
draws the dedicated refraction phase. Static and cached-deformed vertex pipelines share
`refraction-mesh.frag`. Closed meshes currently use a two-sided, single-interface approximation:
the fragment normal is oriented for the rasterized face, but no entry/exit ray pair or physical
interior distance is reconstructed. The full Phase 1 gate remains open because non-post-processed
window targets, XR eyes, mirror captures, resize validation, and live Vulkan validation still need
focused proof. Overlapping transmissive-surface ordering also remains open; this first fixture uses
non-overlapping panels so its result does not depend on that policy.

The current snapshot intentionally contains sharp emissive source color but not its later Bloom
blur. Do not fix that by sampling only the blurred Bloom target or by adding it in the refraction
shader: the final compositor would then add an unrefracted halo a second time. The next capture
slice must make one sampleable **opaque-plus-Bloom composite** before refraction, then use that
composite as both the refraction source and the preserved live destination. See
[Refraction sampling of post-process composite](../refraction-postprocess-composite-capture.md).
Sphere visual diagnosis and a later bevel modifier are tracked separately in
[Refraction visual diagnostics and bevel](../refraction-visual-diagnostics-and-bevel.md).

### Phase 3: rough transmission

- [ ] Add the separate rough-transmission fragment shader and pipeline routing.
- [ ] Build/reuse a linear-color mip or blur pyramid from the same scene snapshot.
- [ ] Map roughness and effective thickness to a bounded LOD/filter footprint.
- [ ] Skip pyramid generation when no visible rough-transmission surface requires it.
- [ ] Compare roughness `0`, intermediate roughness, and maximum supported roughness against the
      sharp refraction fixture.
- [ ] Verify edge filtering never samples uninitialized padding or wraps to the opposite edge.

Exit gate: rough transmission preserves the refracted displacement while progressively blurring
the background, and its GPU cost is bounded and measured.

### Phase 4: view families, integration, and proof

- [ ] Produce independent snapshots for desktop and both XR eyes with correct orientation.
- [ ] Validate stereo stability: each eye samples its own view rather than sharing the other eye's
      scene color.
- [ ] Decide and implement mirror/capture-view support, or explicitly reject transmissive draws in
      those views until their per-view snapshot is available.
- [ ] Validate clipping, transparency ordering, bloom/post-processing, resize, and render-target
      publication interactions.
- [ ] Record GPU time, image memory, pyramid-generation cost, draw/batch counts, and descriptor
      churn for sharp-only, rough-only, and mixed scenes.
- [ ] Add an MMS example that places text/high-contrast geometry behind centered sharp and rough
      transmissive objects.

Exit gate: desktop and XR behavior are visually stable, all unsupported capture combinations fail
explicitly, and the costs of the added pass boundary and rough pyramid are recorded.

## Acceptance criteria

- Refraction and rough transmission are distinct material enum variants, fragment stages, and
  resolved pipeline identities.
- Transmission is not implemented as a toon-shader flag.
- A material composes typed vertex and fragment stage components plus constrained render state;
  the resolved graphics pipeline is cached separately from material identity.
- Incompatible stage interfaces or descriptor requirements fail validation before drawing.
- Static and cached-deformed/skinned meshes use the same authored transmission models.
- Each active supported view samples a same-frame, full-viewport opaque scene snapshot.
- A refracted lookup may cross the object's silhouette without a gap.
- A lookup outside the captured viewport follows the documented clamp/fade fallback and never reads
  undefined data.
- Roughness changes filtering footprint rather than merely reducing opacity.
- Rough-transmission work is omitted when no rough surface is visible.
- MSAA, post-processing, resize, desktop, and XR paths have focused validation.
- Performance and memory deltas are recorded before the epic is marked complete.

## Explicit non-goals for the first complete slice

- path-traced or ray-traced transmission;
- physically exact two-interface solid refraction;
- recovering off-screen or camera-occluded geometry;
- recursive refraction through multiple transmissive layers;
- refraction of ordinary transparent objects;
- dispersion/chromatic aberration;
- depth-aware rough-transmission denoising;
- oversized guard-band rendering;
- arbitrary user-authored shaders or a general material node graph; and
- a public shader-stage plug-in ABI or unrestricted pairing of shader asset paths; and
- replacing the existing mirror capture model.

## Stop condition

Stop expanding the feature when the two dedicated material models render the same-frame opaque
scene correctly for desktop and XR, screen-edge fallback is deterministic, static and
cached-deformed geometry are covered without authored variant explosion, and the focused cost
measurements are recorded. Everything in the non-goal list requires a new task justified by a
specific scene or measured failure.

# Transmission quality, depth, and MSAA configuration seams

Status: review complete; proposed APIs are not implemented.

## Outcome

The three settings do not belong at the same ownership level:

| Setting | Recommended owner | Why |
| --- | --- | --- |
| Transmission snapshot resolution | a `TransmissionPass` under `RenderGraph` | One color/depth snapshot is shared by every transmissive object in a view. Per-object resolution would conflict. |
| Foreground depth comparison | `Refraction` and, later, `RoughTransmission` | This is a material correctness/art-direction choice and can vary between objects sharing a snapshot. |
| Mirror MSAA | `Mirror`, with the renderer setting as its inherited default | Each mirror already owns its capture resolution and capture targets. Its sample count can also vary once pipelines are keyed by sample count. |

All renderer images do **not** need the same sample count. Images attached to the same rendering
scope must have compatible sample counts, and the graphics pipeline used in that scope must use
that count. Independent scopes may use different counts. Sampled post-process images and resolve
destinations are normally single-sample.

## Current coupling

`RendererSettings` currently exposes one `msaa4x` boolean and the renderer turns it into one
renderer-wide `msaa_samples` value. That value is used when scene pipelines are created and is
reused for window, XR, mirror, post-process extraction, and depth attachments.

That global coupling is an implementation choice rather than an image-format requirement. The
renderer already contains counterexamples:

```text
main scene color (1x or 4x) + matching live depth (1x or 4x)
                |
                +--> main color resolve (1x)
                +--> refraction color snapshot (1x)
                +--> refraction depth snapshot (1x)

emissive extraction (1x or 4x) --> Bloom source (1x)
                                  --> Bloom blur A/B (1x, optionally half resolution)

mirror color attachment (currently inherits 1x or 4x)
  + matching mirror depth (same count)
  --> mirror resolve/sample texture (1x)
```

Relevant current seams:

- `src/engine/ecs/component/renderer_settings.rs` owns the single global toggle.
- `src/engine/graphics/vulkano_renderer.rs` creates scene pipelines with that sample count and
  stores only one `msaa_samples` value.
- `src/engine/graphics/post_processing.rs` creates optional multisampled scene/emissive
  attachments but keeps their sampled resolves and Bloom blur images single-sample.
- `src/engine/graphics/vulkano_renderer.rs::ensure_window_refraction_targets` creates the
  refraction color and depth snapshots as full-resolution, single-sample images.
- `src/engine/graphics/vulkano_renderer.rs::ensure_mirror_offscreen_targets` creates mirror color
  and depth attachments using the global count and a single-sample color resolve.

## 1. Transmission snapshot quality and resolution

### Ownership

Resolution must be configured per transmission pass/view family, not per `Refraction` object.
Every visible sharp-refraction object samples the same immutable scene-color snapshot, and every
depth-aware object samples its matching depth snapshot. Two objects cannot safely request two
different resolutions without allocating and producing two separate snapshot pairs.

The scene-facing seam should therefore be a render-graph child. Proposed authoring:

```mms
RenderGraph {
    TransmissionPass.resolution_scale(0.5) {}
}
```

`TransmissionPass` does not exist yet. If it is omitted, visible transmission should continue to
activate an implicit full-resolution pass so existing scenes preserve their appearance. A renderer
setting could eventually provide device/profile defaults, but it should not be the only authored
seam because this work and its images are part of the render graph.

Prefer a validated scale such as `1.0`, `0.5`, or `0.25` over a vaguely named `quality` value.
This makes the resource consequence inspectable:

```text
snapshot pixels = view_width * view_height * resolution_scale^2
```

At half width and height, each snapshot contains one quarter of the pixels. The snapshot memory,
copy/downsample traffic, and typical texture-cache demand decrease correspondingly. The geometry
draw itself does not become one quarter as expensive: transmissive surfaces are still rasterized at
the output view resolution.

### Color and depth must remain a pair

If the transmission color snapshot is reduced, its depth snapshot must use the same extent and UV
mapping. Color may be filtered while reducing. Depth needs an explicit conservative reduction
policy because ordinary averaging can reintroduce foreground leakage.

This renderer uses `0` for near and `1` for far. A conservative reduced-depth texel should therefore
retain the **minimum** (nearest) contributing depth when the purpose is rejecting foreground
samples. That can reject some otherwise valid refraction around thin silhouettes, but it does not
pull foreground color through the surface. A later quality mode could offer a less conservative
policy explicitly.

The current MSAA depth resolve uses `SampleZero`. Reducing spatial resolution cannot be folded into
that attachment resolve because a Vulkan resolve keeps the attachment extent. It needs a subsequent
depth reduction pass (or another explicitly supported copy/filter path). This should be included in
the cost of the lower-resolution option.

### Rough transmission is a separate axis

Base snapshot resolution and rough-transmission filtering quality should not share one setting.
`RoughTransmission` may eventually need a mip chain, blur pyramid, or sample-count setting. Its
filtering quality determines how the snapshot is blurred; `TransmissionPass.resolution_scale`
determines how much source information exists before that filtering.

## 2. Optional foreground-depth comparison

Implementation and A/B acceptance are tracked in
[Optional refraction foreground-depth comparison](../task/refraction-depth-compare-configuration.md).

The proposed material API is:

```mms
Refraction.depth_compare(false) {}
```

The actual component name is `Refraction`, not `Refractive`. The default should remain `true`.
`false` deliberately restores the cheaper, less correct screen-space behavior: a displaced lookup
may sample an opaque/cutout object that is in front of the transmissive fragment.

This option belongs in the common transmission options so `RoughTransmission` can expose the same
contract when its renderer path is implemented. It must be a real boolean. The current
`TransmissionOptions` payload contains four fully assigned floats—IOR, thickness, strength, and
edge fade—and the current scripting dispatcher converts all transmission builder arguments to
`f32`. Do not overload a sign bit or magic float value for this flag.

### GPU and resource behavior

The renderer should aggregate the visible requirements for each view:

| Visible transmissive objects | Depth snapshot | Fragment depth read |
| --- | --- | --- |
| none | absent | none |
| all use `depth_compare(false)` | absent | none |
| at least one uses `depth_compare(true)` | produced once for the view | only depth-aware fragments need it |

The narrow first slice may keep producing/binding depth and add a flat per-instance flag that skips
the texture read in the shader. A follow-up should aggregate the flag before target preparation so
an all-disabled view also skips the depth resolve/copy and depth snapshot allocation.

There are two reasonable GPU implementations:

1. A flat per-instance flag and one shader pipeline. This is the smallest code change and permits
   mixed objects, but mixed flags within a fragment wave can diverge and the shared descriptor
   layout still contains depth.
2. Depth-aware and depth-unaware batch/pipeline variants. This removes the branch and depth binding
   from the unaware variant, but adds batch identity and pipeline variants.

Start with the flag unless measurement demonstrates meaningful divergence. Keep it as a distinct
integer/flags field in instance data so the four authored transmission floats retain their current
meaning.

## 3. Independent MSAA for mirrors and other view families

### What must match

Within one geometry rendering scope:

- the color attachment sample count must match the pipeline rasterization sample count;
- the depth/stencil attachment used with it must use that same count; and
- an optional resolve destination must be single-sample.

Attachments in another scope do not have to match. A mirror capture can be rendered at 1x while
the desktop window renders at 4x, and its resolved mirror texture can still be sampled normally by
the 4x window pass. Likewise, each XR eye's internal rendering count can differ from mirror capture
MSAA. The XR swapchain/copy destination does not become multisampled merely because the internal
eye render target is multisampled.

Mirrors do not inherently require MSAA. Turning it off reduces mirror color/depth attachment
storage and multisample raster work, while making reflected geometry edges more aliased. Mirror
resolution and mirror MSAA are independent: a high-resolution 1x mirror and a low-resolution 4x
mirror have different quality and cost tradeoffs.

### Proposed authoring seam

`Mirror` already owns a local `quality` value from 64 through 2048. Add an optional local sample
override, with absence meaning inherit the renderer default:

```mms
Mirror.quality(1024).samples(1) {}
```

Initially accept only the renderer-supported values (`1` and `4`) and fall back predictably when
the device does not support the request. `RendererSettings` can later grow separate window, XR,
and mirror defaults if device profiles need them, but a second global boolean would not solve the
per-mirror use case.

### Required renderer seam

This is not only an image-allocation change. Current scene/material graphics pipelines are created
once with the renderer-wide sample count and reused for mirror rendering. To support a different
mirror count:

- make sample count part of the graphics-pipeline cache/identity;
- select the matching pipeline family for each render view;
- store sample count in `MirrorOffscreenTargets` and include it in target recreation checks;
- allocate mirror color and mirror depth with the requested matching count;
- allocate/use the single-sample resolve only when the requested count is greater than one; and
- test window 4x + mirror 1x, window 1x + mirror 4x, and stereo mirror captures.

The same pipeline-key change is the prerequisite for independent XR MSAA. It is better to model
sample count as a property of `RenderView`/render-target policy than to add mirror-specific branches
through every pipeline selector.

## Recommended implementation slices

- [ ] Add `depth_compare: bool = true` to common transmission authoring, serialization, resolution,
      instance data, and shader input; preserve current depth allocation initially.
- [ ] Aggregate per-view depth requirements and omit the refraction depth snapshot/resolve/copy
      when no visible transmission needs it.
- [ ] Add an implicit/configurable `TransmissionPass` with `resolution_scale`, paired color/depth
      extents, conservative depth reduction, and resize/XR tests.
- [ ] Key scene pipeline families and offscreen targets by sample count.
- [ ] Add `Mirror.samples(1|4)` as an inherited local override and benchmark representative mirror
      counts and resolutions.
- [ ] Decide whether XR sample count needs a separate renderer/profile default after the sharp and
      rough transmission XR path is implemented.
- [ ] Feed these activation predicates, image extents/sample counts, and pipeline variants into
      [the material/renderer resource graph task](../task/material-renderer-resource-graph.md).

## Decision summary

- Put transmission resolution on the shared render pass, not the material.
- Put `depth_compare` on the material, default it on, and optimize the all-disabled view later.
- Decouple MSAA by render view. Start by giving each mirror an inherited local override.
- Keep resolve, Bloom, and transmission sampling images single-sample; only matching attachments in
  a multisampled geometry scope need the higher sample count.

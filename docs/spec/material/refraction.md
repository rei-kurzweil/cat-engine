# Refraction material

Status: desktop sharp-refraction path implemented; XR and rough transmission remain incomplete.

## Purpose

`Refraction` is a dedicated transmissive shading model. It bends a same-frame, renderer-owned
screen-space snapshot inside the renderable's silhouette. It is not a toon-shader option and it
does not use the renderable's authored mesh UVs for scene lookup.

Typical MMS authoring places the component under the same renderable as its color/tint:

```mms
T.position(0.0, 0.0, 0.0) {
    Grabbable {}
    R.sphere() {
        C.rgba(0.88, 0.96, 1.0, 1.0)
        Refraction.ior(1.52).thickness(0.22).strength(1.0).edge_fade(0.025)
    }
}
```

## Authored inputs

| Input | Meaning | Constraint |
| --- | --- | --- |
| `ior` | Index of refraction used by the screen-space direction approximation | finite, at least `1.0` |
| `thickness` | Effective travel distance controlling displacement | non-negative |
| `strength` | Artistic multiplier on displacement | non-negative |
| `edge_fade` | Viewport-edge width over which displacement fades | positive |
| `depth_compare` | Reject displaced samples owned by nearer opaque/cutout geometry | boolean; defaults to `true` |
| color/alpha | Mild transmission tint and surface alpha | inherited from normal color authoring |

The current closed-mesh behavior is a two-sided, single-interface approximation. It orients the
normal for the rasterized face but does not reconstruct entry and exit points or physical interior
distance.

## Desktop render order

The currently supported desktop path requires active post-processing:

```text
background + opaque + cutout
          |             |
          |             +--> single-sample scene depth
          +--> emissive extraction --> Bloom blur
                                    |
main color + Bloom -----------------+--> immutable scene color
                                             |
                                             v
                                      sharp refraction
                                             |
                           ordinary transparency + overlay
                                             |
                                           present
```

Bloom is composed before refraction so the glow bends with the emissive surface. The final output
does not add Bloom a second time.

## Renderer-owned inputs and attachments

For every supported frame/view containing sharp refraction, the renderer may own:

| Resource | Samples | Created when | Use |
| --- | ---: | --- | --- |
| main color | view MSAA count plus single-sample resolve | post-processing is active | opaque/cutout destination and Bloom base |
| live depth/stencil | view MSAA count | normal 3D rendering | opaque/cutout writes; later depth testing |
| refraction scene color | 1 | refraction is visible | immutable opaque-plus-Bloom sample source |
| refraction scene depth | 1 | refraction is visible | reject displaced foreground samples |
| Bloom source and blur images | 1, with an optional MSAA source | Bloom is active | emissive extraction and filtering |

The scene-color and scene-depth snapshots have matching view, frame slot, extent, and projection.
They are renderer-internal and are not published as authored `Texture` resources.

With MSAA enabled, color and depth are resolved before transmission samples them. Depth currently
uses `SampleZero` resolve into a single-sample `D32_SFLOAT_S8_UINT` image. Without MSAA, depth is
copied at the opaque/cutout boundary. A depth-only view is bound to the shader even though the
allocation also contains stencil.

## Shader lookup

The fragment shader derives `base_uv` from `gl_FragCoord / viewport`, calculates a refracted offset
from view direction, surface normal, IOR, thickness, and strength, then clamps the candidate UV to
the captured viewport. Displacement fades near viewport edges.

### Foreground-depth rejection

The scene-color snapshot stores only the nearest opaque/cutout surface at each UV. A displaced
lookup can therefore land on geometry that is closer to the camera than the transmissive fragment.
The shader samples matching scene depth and accepts the candidate only when:

```text
candidate_depth + foreground_bias >= transmissive_fragment_depth
```

The renderer uses conventional `0`-near, `1`-far depth, clears to `1.0`, and tests with
`LessOrEqual`. The initial foreground bias is `1e-4` in device-depth space.

If the candidate is foreground, refraction falls back to `base_uv` before its single color lookup.
This prevents the foreground object from being pulled sideways into the surface without adding a
second color sample.

`Refraction.depth_compare(false)` skips the scene-depth sample and uses the displaced, clamped UV
directly. This is a per-material comparison control, so enabled and disabled objects can share one
view. The renderer still creates and binds the depth snapshot in either mode; resource elision for
an all-disabled view is deferred.

Depth rejection cannot reveal the background hidden behind that foreground object because the
one-layer snapshot never captured it. Layered captures, depth peeling, or ray tracing would be
required for that information.

## Current support and limitations

- Desktop window with active post-processing: sharp refraction, Bloom inclusion, depth rejection.
- XR eyes: the material is authored but per-eye scene snapshots are not wired yet.
- Mirror/capture views: no supported transmissive snapshot contract yet.
- Rough transmission: authored component exists, but its dedicated filtered renderer path is not
  implemented.
- Ordinary transparent content and other transmissive surfaces are not included in the immutable
  source.
- Viewport-external and camera-hidden geometry cannot be recovered.

## Cost

The current refraction path adds one single-sample depth image per supported view/frame slot and a
depth resolve or copy at the opaque boundary, regardless of authored `depth_compare` values.
Enabled fragments add one nearest-filtered depth read; disabled fragments skip it. Both modes still
perform one scene-color read. MSAA and high-resolution stereo views make depth preparation the
dominant added cost; record GPU timestamps and allocated image bytes before making cross-view
performance claims.

## Related work

- [Transmissive materials epic](../../task/epic/transmissive-materials.md)
- [Foreground-depth leakage tracker](../../task/refraction-foreground-depth-leakage.md)
- [Bloom-before-refraction capture](../../task/refraction-postprocess-composite-capture.md)

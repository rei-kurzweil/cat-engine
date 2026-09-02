# LOD and mipmaps

Date: 2026-09-02

## Purpose

Keep two related but distinct ideas aligned:

- **Mip levels** are prefiltered resolutions within one image. They solve texture
  minification and give a rough-transmission shader a bounded way to blur its
  same-frame scene snapshot.
- **LOD selection** chooses a stable, viewer-family-specific quality tier for a
  consumer. The consumer decides what that tier changes: mesh complexity,
  mirror capture extent, procedural detail, update rate, or a transmission
  budget.

The terms should not be conflated. A shader selecting `textureLod` from
authored roughness is selecting a mip level for filtering; it is not running
the shared adaptive LOD policy.

## Connected work

| Work | Owns | Current direction |
| --- | --- | --- |
| [Texture mipmaps and filtering](../../draft/texture-mipmaps.md) | Mip-chain creation and sampler behavior for authored textures | Generate full chains on upload where possible, retain pre-baked DDS chains, and allow opt-out by clamping sampling to level zero. This is a draft, not yet an implementation tracker. |
| [Generalized LOD policy and selection](../../task/generalized-lod-policy-and-selection.md) | Shared policy: projected coverage, discrete tiers, hysteresis, cooldown, and per-viewer-family runtime state | The selector returns a tier/detail factor; it does not perform consumer-specific allocation or rendering work. |
| [Adaptive mirror detail](../adaptive-mirrors.md) | Mirror-specific use of a selected tier | Map tiers to bounded capture-resolution bands while preserving the authored quality ceiling. |
| [Transmissive materials](../../task/epic/transmissive-materials.md#phase-3-rough-transmission) | Rough-transmission filtering and render resources | Build a renderer-owned, same-frame linear-color mip or blur pyramid only when rough transmission is visible, then map roughness and thickness to a bounded filter footprint. |

## Boundary and sequencing

Rough transmission must not wait for general adaptive LOD: its fixed image
pyramid is the base filtering mechanism required to make frosted glass.
General texture mipmaps also need not be complete first, although both systems
should share image-pyramid and synchronization utilities where that produces a
clean renderer boundary.

Once generalized LOD exists, rough transmission can become a consumer:

```text
authored roughness + thickness -> shader mip/blur footprint
shared LOD tier              -> maximum pyramid resolution / maximum filter level /
                                optional effect budget or disablement
```

The shared LOD tier reduces cost but must not alter the meaning of authored
roughness within the quality budget it selects. Its result is viewer-family
specific, so stereo eyes receive a coordinated choice.

## Rough-transmission constraints

The transmission input is not an authored texture. It is a same-frame,
renderer-owned scene-color snapshot, so its pyramid is rebuilt or refreshed
for each active supported view and must not use the one-frame-delayed texture
publication path.

Foreground-depth rejection also remains a material correctness rule. A simple
color mip can contain foreground colour averaged from nearby pixels. Before
accepting a broad filtered footprint, the rough path needs a conservative
matching depth policy (or a deterministic fallback to a finer valid level) so
filtering does not reintroduce the foreground leakage rejected by sharp
refraction.

## Next documentation additions

Add focused notes here as decisions are made about:

- a reusable renderer image-pyramid utility and its Vulkan transitions;
- the rough-transmission depth-pyramid/rejection policy;
- how LOD tiers map to transmission budgets; and
- measurements for image memory, pyramid generation, and visual error.

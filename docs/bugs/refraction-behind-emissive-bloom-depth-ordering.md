# Bug: Refraction draws over Bloom from nearer emissive geometry

Status: open / desktop reproduction described; renderer-path diagnosis documented; fix not selected

## Observed behavior

With Bloom enabled, place a `Renderable` using `Refraction` behind a bright emissive rectangle.
The refractive object is correctly hidden behind the rectangle's solid red geometry, but it can
draw on top of the rectangle's blurred red glow. The ordering is therefore inconsistent across
the same source object:

| Area of the nearer emissive rectangle | Observed result |
| --- | --- |
| Solid rectangle / emissive core | Refraction is depth-occluded |
| Bloom halo outside the rectangle silhouette | Refraction appears over the glow |

The same effect is visible when the glow is the only part of the rectangle at the displaced lookup
coordinate. This can make the refractive object appear in front of a nearer object even though its
opaque geometry is correctly behind it.

## Reproduction

1. Enable the desktop Bloom path.
2. Add a bright emissive red rectangle and a refractive renderable behind it.
3. Give the refractive material enough displacement to cross the rectangle boundary or its halo.
4. Compare the refractive surface over the rectangle core and over the surrounding blurred glow.
5. Repeat with Bloom disabled; the inconsistent halo ordering should disappear because the extra
   color-only layer is absent.

## Expected behavior

The nearer emissive source should occlude the refractive object consistently, or the renderer should
apply an explicit documented policy for whether Bloom is allowed to appear through/over refraction.
The solid source and its derived glow should not accidentally use different visibility rules.

## Current render-path explanation

The current desktop path has separate ownership for scene color and scene depth:

```text
emissive rectangle geometry ──> opaque color + geometry depth
             |
             +──> emissive extraction ──> blur ──> Bloom color

opaque color + Bloom color ──> scene-color snapshot ──> refraction color lookup
geometry depth ──────────────> live depth / refraction depth comparison
```

The emissive rectangle's geometry writes a nearer depth value during the opaque/emissive scene
render. The refractive pass still uses ordinary depth testing, so fragments behind the rectangle
fail over the solid silhouette.

Bloom is a blurred additive color result. Its blur spreads beyond the source silhouette, but it does
not write matching source ownership or source depth for those pixels. Outside the rectangle's
silhouette, the live depth value can belong to background geometry (or clear depth), so a refractive
fragment behind the rectangle passes the normal depth test there. Its scene-color lookup then reads
the precomposited snapshot containing the red halo and writes the refracted result over that halo.

This is not evidence that Bloom is writing a nearer depth or that the refraction depth comparison is
not running. It is the expected consequence of comparing a geometry-depth attachment against a
blurred color layer whose pixels no longer have one-to-one geometry ownership.

## Root-cause boundary

The bug is at the boundary between geometry visibility and post-process color ownership:

- depth testing can occlude against the rectangle's solid geometry;
- the Bloom blur has no equivalent per-pixel source depth/ownership metadata;
- refraction samples the Bloom-composited color, while its visibility test still uses geometry depth.

The existing foreground-depth fallback only rejects a displaced candidate when the sampled scene
depth is nearer than the refractive fragment. A halo pixel whose depth belongs to the background is
therefore eligible even when its color originated from foreground emissive geometry.

## Non-goals for this report

- Do not treat this as a simple depth-write toggle on Bloom.
- Do not change refraction ordering, snapshot allocation, or Bloom composition until the desired
  visibility policy is chosen.
- Do not claim that the per-material `depth_compare` A/B switch fixes this halo case; disabling it
  changes candidate rejection but cannot restore Bloom source ownership.

## Candidate follow-ups

Any fix needs a policy or representation that connects blurred Bloom contribution to visibility,
for example:

- source-depth/ownership metadata carried through the Bloom pipeline;
- depth-aware Bloom compositing that does not spread foreground glow across background ownership;
- separate foreground/background Bloom layers with an explicit refraction compositing rule.

Each option changes post-processing resources or pass boundaries and should be evaluated separately
from the current refraction depth-comparison A/B work.

## Related work

- [Optional refraction foreground-depth comparison](../task/refraction-depth-compare-configuration.md)
- [Foreground-depth leakage](../task/refraction-foreground-depth-leakage.md)
- [Bloom-before-refraction capture](../task/refraction-postprocess-composite-capture.md)
- [Bloom stencil clipping](bloom-stencil-clipping.md)

# Task: Prevent Bloom from bypassing refraction

Status: next renderer slice.

## Problem

Sharp refraction currently snapshots `main_color` after background, opaque, and cutout rendering.
That contains the normally shaded, sharp emissive object, so that base object can be refracted.

Bloom is different: after the refraction pass, the renderer redraws emissive geometry into a
separate Bloom source, blurs it, and the final fullscreen composite adds that blurred image over
`main_color`. This late additive layer is not sampled by refraction and is not displaced by the
refractive surface. Consequently, a lens can show the underlying emissive object refracted while
its glow remains at the original, unrefracted screen position.

Sampling only the blurred Bloom target is not correct: refraction needs the full scene. Leaving
the normal final Bloom composite in place is also not correct, because it continues to add an
unrefracted halo over the refractive silhouette.

## Required result

For a supported desktop view, refraction samples the same-frame scene after opaque/cutout and
Bloom composition. The refractive surface replaces that composited image inside its covered pixels,
so a bright halo bends with the underlying emissive object and is not also visible unrefracted
through the surface.

## Proposed pass ownership

```text
background + opaque + cutout -> main_color
emissive extraction -> bloom source -> blur
main_color + blurred Bloom -> composited_scene (sampleable)
composited_scene -> live refraction destination
refraction -> ordinary transparent -> overlay -> present/blit
```

`composited_scene` is a renderer-internal, per-view, single-sample target. It needs color-attachment,
sampled, and transfer usage appropriate to the chosen copy/blit path. It cannot be sampled while it
is bound as the draw destination.

## Work

- [ ] Extract the existing Bloom extraction/blur sequence into a callable stage that can run before
      the refraction phase while preserving depth-based emissive filtering.
- [ ] Allocate/reuse `composited_scene` by view extent, format, MSAA mode, and frames in flight.
- [ ] Composite `main_color` and the blurred Bloom result into that target before refraction.
- [ ] Preserve the composited result as the destination beneath refraction, then run ordinary
      transparency and overlay exactly once.
- [ ] Make the final pass a blit when the Bloom composition has already been consumed; do not add
      Bloom a second time.
- [ ] Cover Bloom enabled/disabled, no emissive content, MSAA on/off, resize, and target ownership
      transitions.
- [ ] Add a visual fixture with a bright emissive line crossing a refractive sphere and pane; prove
      the halo bends without an unrefracted duplicate inside either silhouette.

## Non-goals

Do not fold ordinary transparent content into this source, add recursive transmission, or change
rough-transmission filtering in this slice.

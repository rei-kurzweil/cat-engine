# Adaptive mirror detail

Date: 2026-09-02

Status: proposed. Establish the correct viewer-family mirror result first, then
optimize from measurements.

[Back to the desktop workbench](README.md)

## Outcome

Spend mirror pixels where they are visible:

- distant or small on-screen mirrors render at lower resolution;
- nearby mirrors increase useful pixel density without unstable resolution
  flicker; and
- when only a close-up subregion of a mirror is visible, render more detail for
  that visible region instead of paying for the entire authored mirror surface.

Do not move or resize authored mirror geometry as an optimization side effect.
Represent close-up concentration as a capture crop/tile, viewport/scissor, or
equivalent projection mapping whose UV transform is explicit and stable.

## Existing foundation

- Mirrors already carry an authored `quality`/resolution scale and use
  per-mirror offscreen targets.
- [Mirror viewer-family captures](../task/mirror-viewer-family-captures.md)
  records the need for independent mono and stereo capture families.
- [Mirror camera orientation and tracking](../bugs/mirror-camera-orientation-and-tracking.md)
  remains a correctness issue. Adaptive quality must not conceal it.
- [Mirror render-pass status](../task/mirror-render-pass-status.md) records
  remaining self-exclusion, material, recursion, and observability work.

## Design questions

1. Is the quality signal projected mirror area, distance, grazing angle, visible
   clipped area, or a weighted combination? Projected pixel coverage should be
   the baseline because distance alone misprices large and oblique mirrors.
2. Is adaptation per logical mirror, viewer family, or concrete stereo view?
   It must not make left/right eyes choose visibly incompatible detail.
3. Which discrete extent bands are allowed, and what hysteresis/cooldown avoids
   reallocating targets while the viewer moves near a threshold?
4. For close-up partial visibility, should the renderer crop to the visible
   mirror polygon's screen-space bounds, use fixed tiles, or retain a full
   target with a smaller high-detail inset?
5. How are crop/projection and UV remapping published so the surface samples
   the right portion of the capture?
6. What budget caps total mirror pixels and capture count across many mirrors?

## Milestones

- [ ] Add mirror counters/timings: logical mirrors, capture views, target
      extents, allocated pixels, visible projected pixels, and GPU time.
- [ ] Add a repeatable scene with one mirror viewed far, medium, near, at a
      grazing angle, and as a partially visible close-up.
- [ ] Fix/validate viewer-family orientation and parallax before judging image
      quality changes.
- [ ] Implement discrete resolution bands from projected coverage with
      hysteresis and a minimum/maximum authored quality clamp.
- [ ] Reuse targets within bands; prove normal camera motion does not allocate
      every frame.
- [ ] Design and prototype close-up visible-region cropping with explicit UV
      remapping and guard pixels for filtering.
- [ ] Add a total per-frame mirror pixel budget and a deterministic degradation
      order for multiple mirrors.
- [ ] Verify no left/right stereo mismatch, edge shimmer, stale crop, or visible
      resolution pumping.
- [ ] Compare GPU time and image error against a fixed high-resolution baseline.

## First safe slice

Land resolution bands only. Keep the full mirror capture region and select a
target extent from projected on-screen coverage. Close-up cropping changes the
capture projection and sampling contract, so it should be a separate second
slice after banding is stable.

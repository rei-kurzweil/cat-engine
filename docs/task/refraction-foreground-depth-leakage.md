# Task: Prevent foreground color from leaking into screen-space refraction

Status: desktop implementation complete; visual, XR, rough-transmission, and cost proof pending.

## Problem

A transmissive renderable samples a full-viewport color snapshot captured before the transmissive
phase. That snapshot contains the nearest opaque/cutout surface at every screen coordinate, without
recording whether that surface is in front of or behind the transmissive fragment doing the lookup.

When refraction displaces a lookup onto a neighboring pixel occupied by an object closer to the
camera, the foreground object's color is pulled sideways into the transmissive surface. Moving the
transmissive object behind other scene geometry makes the error especially obvious. If foreground
geometry covers the transmissive fragment's own pixel, ordinary depth testing already hides that
fragment correctly; the bug concerns a **displaced** lookup landing on foreground geometry elsewhere
in the snapshot.

Previously, `refraction-mesh.frag` sampled only `scene_color` and could not reject that lookup. The
desktop post-processing path now prepares and binds a matching single-sample scene-depth snapshot.
XR and other view families do not yet provide transmissive snapshots.

## Required behavior

For a supported view, a displaced refraction lookup must not contribute color from a scene surface
that is closer to the camera than the transmissive interface being shaded, within a documented
depth bias. Opaque/cutout surfaces behind that interface remain eligible samples.

Rejected samples must degrade deterministically to a valid same-frame color. They must not produce
black, transparency, viewport wrapping, or one-frame-old data.

## Screen-space constraint

Depth rejection can identify that the visible texel belongs to foreground geometry, but it cannot
recover the hidden color behind that geometry: the single-layer color snapshot never captured it.
The practical screen-space fallback is therefore to reduce the displacement toward `base_uv`, whose
scene sample should be behind the transmissive fragment whenever that fragment passed its normal
depth test.

The first implementation should choose one bounded policy:

1. reject the candidate and sample `base_uv`; or
2. take a small, fixed number of steps from the candidate toward `base_uv` and use the furthest
   depth-valid coordinate.

The first policy is the smallest correctness slice. The second can preserve more distortion near a
foreground boundary but adds bounded texture reads and should follow only if the hard fallback is
visually objectionable.

Perfect recovery would require additional scene layers, depth peeling, a separately rendered
background, or ray tracing. Those approaches materially increase memory, bandwidth, rendering, or
scene-query cost and are not required to stop the foreground leak.

## Proposed first slice: depth-aware rejection

- [x] Add a renderer-owned, per-window-frame, single-sample scene-depth input captured/resolved at
      the same opaque/cutout boundary as scene color.
- [x] Use a depth-only image view when sampling the existing `D32_SFLOAT_S8_UINT` resource, or a
      separate resolved depth target where MSAA/depth-resolve support requires it. Do not sample a
      multisampled attachment as though it were the existing `sampler2D` path.
- [x] Extend the refraction descriptor layout with scene depth and bind color/depth from the same
      view, eye, frame slot, extent, and projection.
- [x] Compare candidate scene depth with the current transmissive fragment depth using the engine's
      `LessOrEqual`, clear-to-`1.0` depth convention. Prefer a centralized reconstruction/comparison
      helper if projection variants make raw depth comparison unsafe.
- [x] Reject candidates that are closer than the transmissive surface, with a small documented
      bias to avoid equality noise at intersections.
- [x] Initially fall back to the undisplaced scene-color coordinate. Keep viewport edge fading and
      clamping as separate validity rules.
- [ ] Apply the same rule to sharp refraction and, when implemented, rough transmission. A filtered
      rough lookup must not blend foreground samples back across the rejected boundary.

The current rough-transmission slice samples its undisplaced screen coordinate, so it does not need
the sharp path's displaced-candidate rejection. Its 1/2 through 1/32 color levels still lack
matching depth-aware filter footprints, however. It is therefore intentionally **not** accepted as
foreground-safe rough transmission: a foreground edge may still bleed into a frosted sample. Keep
this checkbox open until the filtered footprint is made conservative or per-tap depth-valid.

## Reproduction fixture

- [x] Add a high-contrast opaque foreground card, a grabbable refractive pane/sphere behind it, and
      distinct background stripes or grid lines.
- [x] Arrange the foreground card beside part of the refractive silhouette so the surface remains
      visible but its displaced lookup crosses onto the card.
- [ ] Move the refractive object in front of and behind the card; verify the card is eligible only
      when it is behind the shaded transmissive interface.
- [ ] Cover desktop MSAA off/on, then both XR eyes after per-eye refraction snapshots exist.
- [ ] Include a near-contact/intersection case to tune the bias without obvious flicker or halos.

## Acceptance criteria

- No foreground opaque/cutout color is displaced into a transmissive surface from neighboring
  screen pixels.
- Background color remains refracted when its sampled depth is behind the transmissive interface.
- Rejection has a deterministic fallback and no black holes, transparent gaps, edge wrapping, or
  previous-frame trails.
- Color and depth inputs are from the same view and frame; XR never shares depth between eyes.
- The additional image memory, resolve/copy cost, and per-fragment depth read are measured.

## Non-goals

- Recovering geometry hidden behind the nearest opaque snapshot layer.
- Recursive refraction or correct ordering between overlapping transmissive surfaces.
- Refraction through ordinary transparent objects.
- Ray-traced intersections, depth peeling, or an unbounded per-fragment search.

## Follow-up configuration and Bloom boundary

- [Optional refraction foreground-depth comparison](refraction-depth-compare-configuration.md)
  tracks the per-material A/B switch and the observed case where foreground-origin Bloom extends
  beyond the source geometry's depth silhouette.

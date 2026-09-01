# Task: Separate Bloom input from final composition around refraction

Status: proposal / no implementation selected

## Purpose

Allow sharp refraction to respond to same-frame Bloom while preserving normal geometry-depth
occlusion between a refractive object and a nearer emissive source.

The current Bloom-before-refraction path composites blurred Bloom into the scene-color snapshot
before drawing refraction. That lets a refractive surface bend the glow, but the halo has no
matching source-depth or ownership information outside the emissive geometry silhouette. A
refractive object behind the source can therefore be depth-occluded by the solid source while
still drawing over its blurred halo.

This task proposes keeping the opaque scene and Bloom contribution separate until their ownership
around refraction is explicit.

## Proposed high-level order

```text
background + opaque/cutout + emissive geometry ──> main_color + geometry_depth
emissive extraction + blur ───────────────────────> blurred_bloom

main_color snapshot ──────────────────────────────> refraction input: scene color
blurred_bloom ────────────────────────────────────> refraction input: glow color
geometry_depth ───────────────────────────────────> normal depth test + candidate comparison

refraction draws into live main_color
final Bloom composite runs after refraction
```

The refraction shader (or an immediately preceding, renderer-owned composition stage) combines
the displaced scene-color and displaced Bloom inputs for a refractive fragment. The later final
Bloom composite supplies Bloom for the rest of the frame after refraction has completed.

## Required invariant: one Bloom contribution per final pixel

The proposed pass order is not sufficient by itself. If a refractive fragment samples
`blurred_bloom` and the final fullscreen Bloom composite also adds that same bloom over the
refractive silhouette, the result contains a doubled contribution: one displaced and one
unrefracted.

The implementation must choose a single, explicit policy for refractive pixels before the final
composite, such as:

1. write a refraction coverage mask and exclude/mask final Bloom under that coverage;
2. write the refraction result into a separate color target, then composite base color, refraction,
   and Bloom exactly once with a defined ordering; or
3. decide that final Bloom deliberately overlays refraction, in which case refraction must not
   include that Bloom contribution in its own sampled result.

The first two policies match the goal of a refractive object responding to the whole scene while
avoiding an unrefracted duplicate.

## Why this may improve the depth boundary

Keeping Bloom separate prevents the refraction pass from treating blurred color as though it had
the geometry depth of the pixel beneath it. The normal depth attachment still rejects refractive
fragments behind the solid emissive object. The remaining design question is whether glow derived
from that foreground object should be visible in a refractive surface outside the object’s solid
silhouette, and which coverage/ownership rule expresses that answer.

Separating inputs makes that policy a deliberate final-composition decision rather than an
accidental consequence of a precomposited scene-color snapshot.

## Efficiency direction

Prefer reusing the existing blurred Bloom target and the existing non-postprocessed scene-color
copy. The baseline proposal should avoid creating a second full-resolution `main_color + Bloom`
snapshot solely for refraction.

A later implementation can compare the cost of:

- two sampled inputs in the refraction shader plus a coverage mask; versus
- one precomposed refraction input plus source-depth/ownership metadata for Bloom; versus
- a dedicated final composition target that resolves ownership in one fullscreen pass.

Do not select among these from estimated cost alone; measure full-resolution and half-resolution
Bloom with MSAA on and off.

## Acceptance criteria for a future implementation

- A refractive object behind an emissive opaque source remains hidden by the source’s solid core.
- The visible Bloom policy around that source is intentional and stable, not determined by a
  background depth value beneath a blur halo.
- A refractive surface can receive the selected same-frame Bloom contribution without an
  unrefracted or doubled final Bloom layer over its silhouette.
- Bloom stays one contribution per final pixel according to the selected coverage policy.
- Existing opaque, cutout, ordinary transparent, overlay, MSAA, resize, and no-Bloom paths retain
  their current contracts.

## Non-goals

- Do not implement source-depth reconstruction, depth peeling, or recursive transmission in this
  task proposal.
- Do not change the existing `Refraction.depth_compare` material control; it remains a useful A/B
  diagnostic but cannot assign source ownership to blurred Bloom.
- Do not decide XR policy until per-eye refraction snapshots and Bloom targets are available.

## Related work

- [Refraction behind emissive Bloom depth ordering bug](../bugs/refraction-behind-emissive-bloom-depth-ordering.md)
- [Bloom-before-refraction capture](refraction-postprocess-composite-capture.md)
- [Optional refraction foreground-depth comparison](refraction-depth-compare-configuration.md)
- [Refraction material specification](../spec/material/refraction.md)

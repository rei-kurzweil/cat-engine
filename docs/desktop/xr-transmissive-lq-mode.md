# XR transmissive low-quality shared-source mode

Status: design sketch. Implement only after the correct per-eye rough
transmission path exists and has headset measurements.

## Purpose

XR rough transmission normally needs a same-frame scene-colour snapshot and
filtered pyramid for **each** eye. This page proposes an explicit low-quality
mode for performance-constrained hardware: build one snapshot/pyramid from the
left eye and let both eyes' rough-transmission draws sample it.

This shares only the expensive transmissive input. It does **not** render the
world once and present the same final image to both eyes:

```text
normal XR geometry             left eye  -> left output
                               right eye -> right output

shared-source rough transmission
left opaque/cutout (+ Bloom) -> one snapshot -> one rough pyramid
left rough draw  ------------------------------> samples shared pyramid
right rough draw ------------------------------> samples shared pyramid
```

The surface silhouette, depth testing, alpha blend, and placement remain
stereo because the rough-transmission mesh is still drawn once per eye. The
content visible *through* the material has no correct right-eye parallax, and
is therefore only an approximation. This is acceptable only as an opt-in,
clearly named quality reduction.

## Proposed authoring API

Add a typed renderer setting and expose it through one MMS builder:

```rust
pub enum XrTransmissiveMode {
    PerEye,
    SharedLeftEye,
}

RendererSettingsComponent::with_xr_transmissive_mode(mode: XrTransmissiveMode)
```

```mms
// Default; correct stereo transmissive sampling.
RendererSettings.xr_transmissive_mode("per_eye")

// Low quality; one left-eye snapshot and rough pyramid serve both eyes.
RendererSettings.xr_transmissive_mode("shared_left_eye")
```

`per_eye` is the default and must remain the default when no builder is
present. Only the two exact strings above are valid. Reject any other string at
MMS construction/configuration time; do not silently fall back to low quality.

The mode belongs on `RendererSettings`, rather than on `RoughTransmission`,
because it changes view-family resource ownership and pass scheduling for all
visible transmissive draws. It is a device/performance policy, not an artistic
property of one pane of glass.

The first implementation applies this setting to `RoughTransmission` only.
Sharp `Refraction` stays unsupported in XR until it has its own per-eye
correctness work. In particular, `shared_left_eye` must not be presented as a
valid sharp-refraction mode merely because both materials use a scene texture.

## Why the low-quality source is the left eye

Using the already-rendered left eye as the shared source is the only version of
this mode that actually avoids a scene render. A true cyclopean/centre-eye
snapshot would require a third opaque/cutout render, and would likely erase the
intended saving. Averaging left and right snapshots creates disocclusion ghosts
and is not a valid substitute for a centre view.

The asymmetry is intentional and documented: left-eye rough transmission is
registered with its source; right-eye rough transmission reuses the left image.
Alternating source eyes frame-to-frame is forbidden because it would flicker.

## Expected quality and cost

`SharedLeftEye` is most defensible for heavily frosted material. The 1/8–1/32
rough-pyramid levels hide much of the eye-to-eye source mismatch; roughness near
zero exposes it as apparent motion/parallax error inside the glass. The
transmission surface itself still has stereo depth, which can make the mismatch
particularly noticeable around sharp background edges.

What it saves:

- one full-resolution immutable transmissive snapshot and its capture/copy;
- one rough-transmission pyramid allocation; and
- one per-frame pyramid generation sequence.

For stereo, this approximately halves those **transmissive-specific** resources
and filtering passes relative to a per-eye implementation.

What it does not save:

- left/right opaque, cutout, transparent, overlay, and rough mesh draws;
- normal left/right output images;
- per-eye post-processing/Bloom needed for the final eye images; or
- the scene's normal stereo depth/parallax outside the material.

Do not describe this as “single-pass XR” or expect it to halve total XR frame
time. Measure it against the isolated snapshot/pyramid timing and then against
whole-frame GPU time.

## Renderer work

### Shared plumbing

1. Add `XrTransmissiveMode` to `RendererSettingsComponent`, serialize it, and
   register its builder in both the direct and configured MMS registries.
2. Propagate the resolved enum through `RenderableSystem` into `VisualWorld`,
   alongside the existing `transmission_depth_compare` renderer setting.
3. Add round-trip/default/invalid-token tests. Existing scenes with no setting
   must resolve to `PerEye`.

### Correct mode: `per_eye`

1. Give each XR eye an independent snapshot (and matching depth only when the
   material path needs it) after opaque/cutout and any pre-transmission Bloom
   composition.
2. Give each eye an independent 1/2, 1/4, 1/8, 1/16, 1/32 rough pyramid.
3. Bind the matching eye's snapshot/pyramid to its rough draw. Never rely on
   eye loop order as an implicit descriptor choice.

This is the reference implementation. It must be implemented and visually
verified first; the low-quality path should share its allocation and recording
helpers rather than become a separate renderer.

### Low-quality mode: `shared_left_eye`

1. During eye 0, capture the sampleable opaque-plus-Bloom scene source into a
   dedicated shared target, then generate the shared rough pyramid.
2. Draw eye 0 rough-transmission meshes using that target and pyramid.
3. During eye 1, bind the same immutable views for rough-transmission meshes.
   The queue order must prove the eye-0 writes finish before eye-1 sampling.
4. Preserve separate per-eye final rendering and final Bloom/output handling.
   The right eye's ordinary scene must not be replaced with the left image.
5. Allocate no shared targets when there is no visible nonzero-roughness
   rough-transmission instance. A roughness-zero material needs only the shared
   sharp snapshot, not the filtered pyramid.
6. Recreate targets on XR extent, format, MSAA/post-process configuration, or
   XR session/view-count changes. Keep in-flight lifetime tracking explicit;
   do not reuse a target merely because the current CPU eye loop advanced.

The current `submit_xr_eye_offscreen` path calls the common draw builder with
no scene snapshot or rough pyramid. The first XR work is therefore to add an
XR target owner and pass the appropriate views into that builder; this setting
then selects one owner per eye versus one shared owner.

## Validation plan

Use [`examples/rough-transmission-xr.mms`](../../examples/rough-transmission-xr.mms),
which deliberately contains no refraction and has roughness `0`, `.25`, `.5`,
`.75`, and `1` over high-contrast opaque/emissive content.

For each mode, verify both eyes independently:

- the source is same-frame and orientation/viewport-correct;
- no uninitialized pixels, cross-eye target aliasing, or descriptor lifetime
  failures occur through resize/session restart;
- roughness increases the filtered footprint in both eyes;
- left-eye output matches the per-eye reference in `shared_left_eye` mode;
- right-eye mismatch is visible, bounded, and documented—especially at
  roughness `0` and `.25`;
- high roughness is subjectively stable under head yaw, translation, and near
  foreground edges; and
- MSAA, Bloom on/off, and no-emissive scenes remain correct.

Record GPU timestamps, allocated image bytes, and pass counts for no
transmission, per-eye rough transmission, and shared-left rough transmission.
Report both per-eye work and total frame time on at least one target headset.

## Non-goals

- sharing a final left-eye image with the right eye;
- a free centre-eye render or blending the two eye images;
- automatic switching based on frame time or material intensity;
- using this mode for sharp refraction in the first XR implementation;
- changing the default away from per-eye correctness; and
- claiming that this solves general XR rendering performance.

## Exit decision

Keep `shared_left_eye` only if measured savings are material and the high
roughness cases are comfortable in a headset. If the visible mismatch remains
objectionable, retain per-eye only; the setting is an optional optimization,
not a required product mode.

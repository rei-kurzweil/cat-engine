# Configurable Piecewise-Linear Toon Light Ramp

## Summary

Replace discrete light quantization with a configurable brightness ramp evaluated after all lights are accumulated. The ramp uses up to eight `(input illumination, output brightness)` control points and linearly interpolates between them, preserving shaded transitions while allowing steep toon-band changes.

## Key Changes

- Replace `LightQuantizationComponent`/`LightQuantization` with `LightRampComponent`/`LightRamp`; remove the old `steps()` API and rename its internal registration intent.
- Add Rust construction through `LightRampComponent::from_points(Vec<[f32; 2]>) -> Result<_, LightRampError>` and MMS construction through `LightRamp.points([[input, output], ...])`.
- Require 2–8 finite points, strictly increasing inputs, and all coordinates within `0..=1`; reject invalid Rust and MMS definitions.
- Use this near-hard three-band default:
  `[(0.00, 0.00), (0.28, 0.08), (0.34, 0.42), (0.62, 0.50), (0.68, 0.86), (1.00, 1.00)]`.
- Replace `quant_steps` throughout visual instances, batching, material caching, and `MaterialUBO` with a fixed-capacity ramp containing a point count and eight padded points.
- In the fragment shader, retain additive light and RGB accumulation, then linearly sample the ramp using `light_amount`. Clamp below/above the authored range to the first/last output.
- Preserve mixed-light chromaticity with:
  `mixed_rgb = light_rgb / light_amount`, then apply the sampled scalar brightness. Ambient light remains separate and unmodified.
- Update existing examples, MMS tokens (`LR`), serialization, registry entries, and generated-facing documentation to the new component name and constructor.

## Test Plan

- Validate accepted ramps and every rejection case: too few/many points, non-finite values, out-of-range coordinates, duplicates, and unordered inputs.
- Test exact control-point values, interpolation within segments, and endpoint clamping.
- Test the default curve at its shallow band regions and steep transition regions.
- Test MMS construction and serialization round trips for nested point arrays.
- Test that identical ramps batch together while different ramps produce distinct material state.
- Compile the GLSL shader with `glslangValidator`, run relevant Rust tests, and run `cargo check`.

## Assumptions

- This is intentionally a breaking replacement; no compatibility alias or `steps()` preset remains.
- Ramps control brightness only and never recolor the accumulated RGB mixture.
- Eight points are stored directly in the material uniform; no ramp texture or additional sampler binding is introduced.
- Combined illumination remains clamped by the ramp endpoints, so additional intensity above the last point changes color weighting but not brightness.

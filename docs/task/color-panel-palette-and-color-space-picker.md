# Task: color panel palette and color-space picker

Date: 2026-08-08

Status: todo / exploration

## Goal

Make the color panel useful for both quick, broadly compatible color selection and richer spatial
color exploration.

The default experience must remain representable as a flat 2D panel. It should offer at least 64
colors with substantially better saturation and more even coverage than the current 16-swatch
palette.

## Current state

`assets/components/panels.mms::color_panel_body()` authors 16 fixed swatches followed by a
single-selection component. The colors mix neutrals, muted colors, and a few saturated colors, but
do not systematically cover hue, saturation, or lightness.

Relevant seams:

- `assets/components/panels.mms`
- `src/engine/ecs/system/editor/paint_panel.rs`
- `src/engine/ecs/system/editor_paint_system.rs`
- the editor-settings panel's option/selection rows as a UI pattern

## Proposed modes to explore

### 1. Flat palette — required default

- At least 64 selectable colors.
- Organized predictably rather than as an arbitrary list.
- Prefer saturated, clearly distinguishable colors while retaining a small neutral ramp.
- Candidate layout: eight hue columns by eight value/lightness or saturation rows.
- Every swatch continues to produce the existing RGBA selection payload.
- Must work on desktop and XR without requiring 3D interaction.

This mode is the compatibility baseline and should ship even if volumetric modes are deferred.

### 2. HSL volume

Explore a cylindrical or otherwise spatial representation:

- angle = hue
- radius = saturation
- height = lightness

The UI needs a clear selected-color marker and a way to pick interior samples without ambiguity.
Consider slices or a movable cross-section if a solid volume is visually or interactively noisy.

### 3. RGB volume

Explore a cube representation:

- X/Y/Z map to R/G/B
- corners expose the full additive-color gamut
- selection resolves to the same RGBA payload as the flat palette

This may be useful diagnostically even if HSL is more intuitive for authoring.

## Mode-selection UI

Add a single-selection control near the top of the color panel using the same interaction model as
the editor-settings mode rows. Candidate options:

- `Palette`
- `HSL`
- `RGB`

The panel owns the selected presentation mode; paint/color application continues to own the
selected RGBA value. Switching modes should preserve the current color when possible.

## Open decisions

- Generate the 64-color palette from a documented formula or author an explicit curated table?
- Should the flat layout be hue × lightness, hue × saturation, or grouped RGB ramps?
- Does alpha remain fixed at `1.0`, receive a separate control, or become another later mode?
- For volumetric picking, do we raycast individual samples, intersect a volume/slice, or use a
  tool-specific mapping?
- Should HSL/RGB modes exist in the flat panel as 2D slices before true 3D volumes are attempted?

## Acceptance criteria

- [ ] The default flat mode exposes at least 64 colors.
- [ ] Hue coverage is visibly even and the majority of chromatic swatches are fairly saturated.
- [ ] A neutral/low-saturation range remains available.
- [ ] Swatches are ordered so neighboring colors have an understandable relationship.
- [ ] Exactly one color selection remains active and existing color-tool behavior still receives
      an RGBA value.
- [ ] A mode selector is designed for `Palette`, `HSL`, and `RGB`, even if the first implementation
      ships only the flat mode.
- [ ] Desktop and XR can use the default mode without relying on depth/volume picking.
- [ ] Automated coverage checks swatch count, payload validity, selection exclusivity, and mode
      persistence.

## Non-goals for the first slice

- A full material editor.
- HDR/wide-gamut color management.
- Replacing the existing paint/color application pipeline.
- Requiring volumetric interaction before the improved 2D palette can land.


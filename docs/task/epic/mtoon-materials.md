# Native editor assets and MToon-oriented materials

Date: 2026-09-06

Status: proposed planning epic

## Purpose

Track four related but independently shippable improvements for anime-style model authoring:

1. show image and GLB files directly in the editor Assets panel without requiring an authored
   `.mms` wrapper file;
2. expose the textures embedded in a GLB as child assets beneath that GLB;
3. add a texture-driven toon material with separate lit and shade maps; and
4. first add a simpler anime material that derives its shade color from the ordinary albedo and
   includes threshold/ramp controls and rim lighting.

The shared goal is to make imported character assets visible, inspectable, and useful from the
editor while improving anime-face lighting without adding branches for unrelated shading models to
the existing toon shader.

This epic is a planning document. Each numbered slice below should become its own focused task.

## Decisions

- An asset does not need an `.mms` file on disk to appear in the Assets panel.
- MMS module exports and native files should converge on the same catalog/item behavior after
  discovery. The engine may create an in-memory factory or equivalent adapter for a native file,
  but it must not write or require a generated wrapper beside that file.
- The first native formats are `.png`, `.jpg`, `.jpeg`, `.dds`, and `.glb`, matched
  case-insensitively. Broader image-format support is not implied by the decoder accepting it.
- An image file represents a texture asset. A GLB file represents a model asset.
- A GLB may initially use a generic cube/model icon and its filename instead of a live 3D preview.
  Embedded-texture inspection should land before work on rendered GLB thumbnails.
- The two new anime/toon behaviors are separate material/shading models. They must not become
  per-fragment mode branches in the existing general toon pipeline.
- The simple albedo-derived anime material is the first shading implementation. The dual-map
  material follows after its texture inputs can be selected and configured cleanly.
- "MToon" in this document describes the intended anime/toon use case. Full VRM MToon
  compatibility is not claimed unless a later task explicitly maps and validates the MToon
  specification.

## Current seams

The existing `AssetSystem` catalogs only direct-child `.mms` files and turns their exported
functions into `AssetItem`s. Preview and placement then assume that each item can be spawned as an
MMS function. Native files therefore require a catalog source/instantiation strategy rather than a
special UI-only list.

The GLTF loader already decodes embedded images, retains stable keys of the form
`{gltf_name}:{image_name_or_index}`, uploads them, and attaches base-color textures to imported
primitives. That decoding and identity work should be shared with editor inspection instead of
parsing or uploading a GLB a second time solely for the panel.

The current toon fragment shader has one base texture and a `quant_steps` light control. The
planned materials need explicit schemas and independent pipeline identities. They should consume
the fragment-program/automatic-vertex-family architecture in
[Materials v2](materials-v2.md), or use a deliberately temporary compatibility bridge if they land
before it.

## Dependency and recommended delivery order

```text
Ticket 4: simple albedo-derived anime material (first shading slice)
    independent of native asset discovery

Ticket 1: native image + GLB catalog entries
                    |
                    v
Ticket 2: GLB embedded-texture child assets
                    |
                    v
Ticket 3: dual-map lit/shade anime material authoring

Later, separate work: rendered GLB thumbnails and outlines
```

Ticket 2 depends on a GLB having a native catalog entry, but its GLB parsing/model work can be
developed in parallel with Ticket 1 once the catalog interface is agreed. Ticket 3's renderer work
can also begin independently; its complete editor workflow benefits from Tickets 1 and 2 because
authors can then select ordinary or embedded textures. Ticket 4 is intentionally first among the
shader tasks because it proves the lighting response without needing multiple authored maps.

## Ticket 1: catalog native image and GLB files

### Goal

Show supported files in the Assets panel and make them behave like other asset items without
requiring persistent MMS wrappers.

### Work

- [ ] Generalize the catalog item/source model so it can represent an MMS export, an image file,
      or a GLB file with a stable typed identity.
- [ ] Preserve the existing rule that only public MMS exports become placeable items; native-file
      discovery must not accidentally expose editor-internal modules or arbitrary shader/audio
      files.
- [ ] Define configured/default native roots. The current default MMS root is
      `assets/components/`, while the repository keeps files in `assets/images/`,
      `assets/textures/`, and `assets/models/`; do not rely on a recursive scan of only the current
      component root.
- [ ] Discover `.png`, `.jpg`, `.jpeg`, `.dds`, and `.glb` case-insensitively in deterministic
      path order and avoid duplicate entries when roots overlap.
- [ ] Adapt an image entry in memory to the equivalent of a `Texture`-backed preview/placeable
      asset. Use the actual image as its panel preview when decoding succeeds.
- [ ] Adapt a GLB entry in memory to the equivalent of `GLTF.new(uri)`. Initially render its name
      and either the existing placeholder or a generic cube/model icon; do not block the item on a
      live model preview.
- [ ] Extend selection and paint/placement payloads so consumers receive the typed asset identity
      and source URI, not an MMS-only `module::export` key disguised as a file.
- [ ] Keep failed/unsupported files visible when practical, with a noninteractive/error preview
      and an actionable diagnostic; one bad file must not abort the rest of the catalog.
- [ ] Do not generate, persist, or watch synthetic `.mms` wrapper files.

### Acceptance criteria

- A configured directory containing one file of each supported extension shows one entry per file.
- PNG/JPEG/DDS entries display an image preview and retain their correct source URI when selected
  or placed.
- A GLB entry displays its filename and a placeholder/generic model icon without instantiating a
  full animated model in the panel.
- Existing MMS module headings, exported-function previews, selection, and placement continue to
  work.
- Catalog order and keys are stable across runs, uppercase extensions work, and overlapping roots
  do not duplicate items.
- No wrapper files are created on disk.

### Tests

- Catalog tests for every supported extension, mixed case, deterministic order, duplicate roots,
  ignored formats, and per-file failures.
- Panel projection tests for image-preview, GLB-placeholder, and existing MMS-export rows.
- Selection/placement tests proving each native kind resolves to the expected component factory or
  typed payload.

## Ticket 2: expose GLB embedded textures beneath the model

### Goal

Under each GLB item, show a section analogous to an MMS module section that lists every embedded
image/texture by name and displays it as an independently selectable image asset.

The intended hierarchy is:

```text
model.glb                         [generic model icon]
  Textures
    face_base_color               [image preview]
    face_shade                    [image preview]
    2                             [image preview]
```

### Work

- [ ] Extract a reusable GLB resource-inspection result from the existing GLTF decode/cache path.
      The Assets panel must not maintain a second, divergent GLB parser or a second GPU upload.
- [ ] Represent embedded images with stable identities based on GLB identity plus image index.
      Human-readable names are labels, not the sole identity, because names may be absent or
      duplicated.
- [ ] Label each image with its authored glTF image name when present and a deterministic numeric
      fallback when absent. Disambiguate duplicate display names without changing stable identity.
- [ ] Project a `Textures` subsection immediately below the owning GLB row, using the same section
      visual language as module headings.
- [ ] Preview the decoded image and allow it to participate in the same selection/configuration
      flow as a standalone image asset.
- [ ] Define lifetime and cache ownership so collapsing/rebuilding the panel does not repeatedly
      decode or upload all embedded images.
- [ ] Surface unsupported/corrupt embedded images as individual errors without hiding the GLB row
      or its other valid images.
- [ ] Keep the first slice limited to embedded images. Meshes, materials, animations, and skeletons
      are not additional Assets-panel children in this ticket.

### Acceptance criteria

- A GLB with named and unnamed embedded images shows all of them once, directly below its GLB row.
- Every child has a deterministic label, stable identity, and correct preview.
- Selecting a child yields a texture reference usable by later material configuration.
- Two images with the same authored name remain distinct.
- Reopening/rebuilding the panel reuses the loader/cache result and does not produce duplicate GPU
  textures.
- A malformed image does not prevent valid siblings or the parent GLB from appearing.

### Tests

- Fixture coverage for named, unnamed, duplicate-name, unused, and corrupt embedded images.
- Identity tests across reload and panel rebuild.
- A loader/cache test proving inspection and model instantiation share decoded resource metadata.
- Panel hierarchy, preview, selection, and partial-failure tests.

## Ticket 3: dual-map lit/shade anime material

### Goal

Add a dedicated material that chooses between an authored fully lit map and an authored shade map
according to scene lighting, with a configurable transition band.

### Initial shading contract

Let `light_amount` be the combined, nonnegative light response after the renderer's supported light
types and attenuation are accumulated. Two ordered thresholds define the blend:

```text
shade region       transition/ramp                    lit region
-------------------|==================================|-------------------
            shade_threshold                       lit_threshold
```

The material samples both maps and blends from shade to lit with a smooth ramp between the two
thresholds. Below the shade threshold it shows the shade map; above the lit threshold it shows the
lit map. Exact smoothing math is an implementation detail, but the endpoints and ordering are
part of the authored contract.

### Work

- [ ] Add a dedicated fragment program/material definition and pipeline identity rather than a
      mode flag in `toon-mesh.frag`.
- [ ] Support `lit_map`, `shade_map`, `shade_threshold`, and `lit_threshold` as named, typed
      material inputs in Rust and MMS.
- [ ] Validate finite thresholds and require `shade_threshold <= lit_threshold`; define a useful
      hard-step result when the two are equal.
- [ ] Preserve source alpha/cutout behavior and define which map owns alpha. The initial
      recommendation is that the lit/albedo map owns opacity while RGB is blended between maps.
- [ ] Use the same accumulated point, directional, and spot-light semantics as the engine's normal
      lit materials unless a difference is explicitly documented.
- [ ] Support static and cached-deformed/skinned meshes through automatic vertex-family resolution,
      not authored `SKINNED_*` material variants.
- [ ] Specify missing-map behavior and actionable load/schema errors; do not silently bind an
      unrelated texture.
- [ ] Add editor/MMS examples using both standalone files and GLB-embedded texture references once
      those asset tickets are available.

### Acceptance criteria

- The same material definition renders on static and cached-deformed/skinned meshes.
- Below, inside, and above the transition band produce the shade map, a stable blend, and the lit
  map respectively.
- Threshold equality produces a deterministic hard boundary, and reversed/nonfinite inputs are
  rejected or normalized according to the documented API contract.
- Alpha/cutout, multiple light types, no-light behavior, and overlapping lights have focused
  coverage.
- Many instances sharing the same material remain batchable; ordinary threshold changes do not
  compile a new shader or rebuild geometry.

## Ticket 4: simple albedo-derived anime material with rim lighting

### Goal

First implement a useful anime-face material that needs only the model's ordinary albedo texture.
Lighting acts as the condition for displaying either the unbrightened albedo or a precisely
controlled tinted shade, with a configurable transition and rim light.

This material is intentionally separate from both the current quantized toon shader and the
dual-map material. A scene choosing one shading model should not pay for runtime branches belonging
to the others.

### Initial shading contract

- The lit state is the sampled albedo at its authored brightness. Direct lighting never makes it
  brighter than that texture.
- The shade state is derived from albedo using a configurable shade tint/color and shade strength.
  The default should be a subtle darker warm/red tint suitable for anime skin, while allowing a
  cool/blue tint or neutral darkening.
- `shade_threshold` and `lit_threshold` bound a smooth transition exactly as in Ticket 3. Equal
  thresholds request a hard step.
- Ambient light must not cause the lit state to exceed albedo. The implementation must document
  whether ambient participates in the threshold decision or only supplies a minimum shade floor.
- Rim lighting is view-dependent, separately configurable, and added without washing the whole
  surface above its authored albedo. At minimum expose rim color, rim strength, and rim power or
  width.
- The first slice does not include outlines.

Illustrative API shape; exact names are not fixed:

```rust
let material = AnimeMaterial::new()
    .with_shade_threshold(0.35)
    .with_lit_threshold(0.55)
    .with_shade_color([0.72, 0.50, 0.54])
    .with_shade_strength(0.30)
    .with_rim_color([1.0, 0.85, 0.92])
    .with_rim_strength(0.18)
    .with_rim_power(4.0);
```

MMS should expose equivalent typed builder methods/properties and serialize them losslessly.

### Work

- [ ] Specify the shade-tint operation precisely—recommended: darken albedo by shade strength and
      multiply or mix toward the shade color—so defaults are reproducible across Rust and GLSL.
- [ ] Add the dedicated anime fragment program/material definition and its parameter schema.
- [ ] Add Rust builder methods plus equivalent MMS construction, setters/properties, and
      serialization.
- [ ] Implement the two-threshold light ramp, clamped lit state, tinted shade state, and
      view-dependent rim term.
- [ ] Validate colors and finite/ranged scalar inputs at the material boundary.
- [ ] Support static and cached-deformed/skinned meshes via automatic vertex-family resolution or
      the temporary compatibility bridge documented by Materials v2.
- [ ] Add a representative anime-face/humanoid example with controls for threshold, shade tint,
      shade strength, and rim response.
- [ ] Measure pipeline count, batch count, and frame time against the existing toon material on a
      many-instance scene.

### Acceptance criteria

- With rim disabled, fully lit pixels never exceed the sampled albedo because of direct lighting.
- Low light produces the configured darker/tinted shade, high light produces the original albedo,
  and the interval between thresholds produces a stable ramp.
- Warm, cool, and neutral shade tints are expressible without replacing the albedo texture.
- Rim controls are visually independent from the light/shade threshold and behave consistently as
  the camera moves.
- Rust and MMS expose equivalent values, and an MMS save/load round trip preserves them.
- Static and animated/skinned models use the same authored material definition.
- Choosing this material uses its own resolved pipeline and does not add an anime-mode branch to
  existing toon draws.

### Tests and proof scene

- Shader/reference tests at representative `light_amount` and view-normal values, including both
  thresholds and a zero-width ramp.
- Validation tests for NaN/infinity, reversed thresholds, out-of-range strengths, and colors.
- Serialization and runtime-update tests for every exposed builder/property value.
- Golden or captured views of a representative face under front, side, back, and moving point
  lights, with rim off and on.
- A batching/performance comparison for existing toon versus the anime material over many static
  and cached-deformed instances.

## Cross-ticket requirements

- Stable identity must be distinct from a display label. File renames may create a new identity;
  duplicate basenames and duplicate embedded image names must not collide.
- Filesystem paths and GLB virtual texture keys must have one documented normalization policy.
- Native adapters must use the same selection, preview, placement, and eventual material-input
  contracts as MMS exports rather than growing parallel editor behavior.
- Texture decode/upload ownership remains outside panel UI components.
- Material values are typed and validated. Do not expose raw UBO bytes or shader-specific offsets
  to Rust or MMS authors.
- Ordinary parameter changes should dirty only the smallest relevant material/instance GPU data;
  they must not rebuild meshes or unrelated pipelines.
- All new UI rows must remain scrollable and must not let preview geometry steal panel pointer
  events.

## Deferred work and non-goals

- rendered or animated 3D thumbnails for GLB files;
- outlines, inverted-hull passes, screen-space edge detection, or outline authoring;
- automatic inference of which embedded image is a lit map or shade map;
- exposing GLB meshes, materials, skeletons, or animations as child assets;
- extracting embedded images to files unless explicitly requested by the user;
- full VRM MToon import or conformance;
- a general node-based material editor; and
- merging the new material modes into one branch-heavy uber-shader.

## Questions to settle in the focused tickets

1. Should native roots be configured as a list, inferred from the project `assets/` layout, or
   represented by one recursive catalog with explicit inclusion/exclusion rules?
2. Does placing an image create a textured quad by default, or is an image initially only a
   selectable texture value for paint/material workflows?
3. Should GLB texture children be always expanded, collapsed by default, or lazily populated when
   the GLB row is expanded?
4. What URI syntax should MMS use for a GLB-embedded texture while keeping image-index identity
   stable if its display name changes?
5. For the simple anime material, should ambient light affect the lit/shade threshold or only the
   minimum visible shade color?
6. Should rim light be capped to albedo in the first slice, or may the explicitly configured rim
   color be brighter while the direct lit state remains capped?

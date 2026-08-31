# Task: Refraction visual diagnostics and bevel follow-up

Status: tracked; follows post-process composite capture.

## Sphere diagnosis

The sharp-refraction fragment shader does **not** use mesh UVs. It derives its base sample from
`gl_FragCoord / viewport`, transforms the interpolated mesh normal into view space, and offsets the
screen lookup using `refract`. A sphere’s unusual look is therefore a normal/front-back or
single-interface issue until a diagnostic proves otherwise.

- [ ] Add a deliberate, project-owned transmission diagnostic scene/asset: high-contrast lines,
      labeled orientation marks, and controlled color bands. Do not import a random stock texture
      just to test UVs.
- [ ] Add a sphere/pane comparison fixture that can separately display normals, base screen UV,
      refracted UV, and sampled color.
- [ ] Verify normal transformation under uniform and non-uniform object scale; document any
      non-uniform-scale restriction or use an inverse-transpose normal matrix.
- [ ] Decide front/back policy for closed refractive meshes. The current two-sided, single-interface
      approximation may rasterize both sphere faces; evaluate front-face-only, culling, or an
      explicit thickness model against the diagnostic fixture.
- [ ] Keep actual mesh-UV verification separate. Add UV-grid diagnostics only when a material path
      samples authored textures; it is not a prerequisite for current screen-space refraction.

## Bevel follow-up

- [ ] Design a `Bevel` component/modifier for generated or authored box-like meshes, with width,
      segments, and safe topology/normal rules.
- [ ] Make it compose with renderable/mesh ownership without silently rebaking unrelated geometry.
- [ ] Add a small bevel to the refraction-pane fixture and compare its rounded edge normal response
      with the hard-edged control.

The bevel work is deliberately after the diagnostic and Bloom-composite work: it should improve a
known-good glass response, not mask a sampling or front/back error.

# Task: wire imported morph factors into shared rendering

Parent: [Morph targets and editor panel](morph-targets-and-editor-panel.md)  
Renderer design: [Morph deformation cache plumbing](morph-deformation-cache-plumbing.md)

## Current checkpoint

glTF import, per-instance `GLTFComponent` morph state, semantic blink mapping,
and compute-shader morph support exist in the working tree.  They are not yet
connected end to end: the Vulkano renderer currently creates one dummy morph
delta and one dummy active-morph record per dispatch and writes zero for every
job's `active_morph_base` and `active_morph_count`.

OpenXR is not a separate morph renderer. It, desktop, mirrors, and extraction
all consume the same cached deformation output from Vulkano; once the shared
compute path is correct, the remaining XR work is validation and scheduling
regression coverage.

## Checklist

### Stable identity and dirty propagation

- [x] Import target-major POSITION/NORMAL deltas into `CpuMesh`.
- [x] Retain stable `(node, primitive, target)` identity and per-instance base
  and driver factors on `GLTFComponent`.
- [ ] Map each rendered GLTF primitive/instance to its `MorphTargetKey` range
  without relying on target labels.
- [ ] Detect effective factor changes and mark only the affected deformable
  visual instance dirty.
- [ ] Release a driver back to its imported base value without a stale GPU
  palette entry.

### Vulkano shared compute path

- [ ] Build one immutable device-local `GpuMorphDelta` arena from imported
  `CpuMesh.morph_targets`; keep its target-major offsets with the GPU mesh.
- [ ] Allocate a stable `GpuActiveMorph` palette range per visual instance,
  sized for that primitive's target count.
- [ ] Repack only a dirty instance's nonzero effective factors into its palette
  range and upload just that range.
- [ ] Set `GpuDeformationJob.active_morph_base` and
  `active_morph_count` from the instance range instead of zero.
- [ ] Do not allocate or upload dummy morph buffers in frames that have no
  morph-capable meshes or active targets.
- [ ] Preserve the existing morph-before-skin compute order and the morph-only
  no-skin-buffer-safe shader behavior.
- [ ] Add renderer counters for active targets, palette upload bytes, and
  morph-caused dirty jobs.

### Tests and visual validation

- [ ] Unit-test the palette: base vs driver precedence, epsilon exclusion,
  negative values, and release-to-base behavior.
- [ ] Add CPU-reference and GPU-path coverage for one/multiple active targets,
  morph-before-skin ordering, and morph-only geometry.
- [ ] Add a focused Bisket blink fixture proving that the two mapped labels fan
  out across its relevant primitives and visibly deform.
- [ ] Verify desktop, mirror, emissive extraction, OpenXR left eye, and OpenXR
  right eye reuse the same deformation generation.
- [ ] Record desktop validation on GTX 1050 Ti Mobile and desktop/XR validation
  on GTX 1080, including the no-active-morph baseline.
- [ ] Confirm tracker loss restores base morph values and produces no permanent
  blink in the VTuber mirror example.

## Completion

`EyesClosedAmount` from generic OSC visibly drives Bisket blink targets in
`examples/vtuber-eye-tracking-mirror.mms`; unchanged frames do no morph
uploads or dispatches; all render consumers, including both OpenXR eyes, show
the same cached result.

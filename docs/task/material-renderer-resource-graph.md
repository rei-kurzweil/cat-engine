# Task: Material and renderer resource graph specification

Status: tracked; start after sharp refraction works in desktop and XR.

## Purpose

Create `docs/spec/material/index.md` as the entry point for shading-model and renderer-resource
documentation. Its primary artifact should make renderer-owned images, conditional work, pipeline
creation, and desktop/XR multiplication understandable without reading the Vulkan backend.

The refraction specification in `docs/spec/material/refraction.md` is the first model-specific
document. This task must extend that structure rather than duplicating its details in one large
page.

## Work

- [ ] Inventory every built-in shading model, beginning with toon, unlit, emissive toon,
      refraction, rough transmission, grid, and mirror.
- [ ] Document each model's required vertex family, fragment stage, descriptors, render state,
      phase, optional inputs, and static/cached-deformed pipeline variants.
- [ ] Separate mandatory renderer nodes from nodes enabled only by visible materials, configured
      post-processing, MSAA, mirrors/captures, runtime textures, or XR.
- [ ] Inventory every potentially allocated renderer-owned image with format, sample count,
      extent/layer rule, frame/view multiplicity, usage flags, producer, consumers, and lifetime.
- [ ] Inventory pipeline identities that may be created and the conditions that create/select them;
      distinguish authored material identity from cached Vulkan pipeline identity.
- [ ] Model desktop window, each XR eye, monoscopic mirrors, stereoscopic mirrors, and runtime
      captures as explicit view families. Show which resources are shared and which multiply per
      eye, frame slot, or capture.
- [ ] Record pass-boundary and synchronization edges: writes, resolves/copies, sampled reads,
      attachment reloads, publication, and presentation/submission.
- [ ] Include cost annotations or formulas for image bytes, full-screen reads/writes, draw passes,
      and conditional shader work. Do not present hardware-independent timing estimates as facts.

## Graph data contract

Define graph nodes with at least:

- stable identifier and human label;
- kind: view, render phase, image, resolve/copy, shader model, or pipeline;
- required versus conditional activation predicate;
- format, extent rule, sample count, and multiplicity for image nodes;
- vertex/fragment/state key for pipeline nodes; and
- desktop/XR/capture applicability.

Define directed edges with at least:

- operation: render, read, write, resolve, copy, load, publish, or present;
- source and destination resource state where relevant;
- per-frame/per-view frequency; and
- the material or renderer feature that activates the edge.

## Rendering the map

- [ ] Evaluate a source-controlled graph format that can render in Markdown and remain readable in
      review. Prefer Mermaid flowcharts for the exact resource DAG; use Mermaid mindmaps only for
      the higher-level shading-model hierarchy.
- [ ] If Mermaid cannot express conditional edges and repeated XR-eye subgraphs clearly, add a
      small deterministic generator that produces an SVG checked into `docs/spec/material/` from a
      reviewable data file.
- [ ] Make stale graph detection testable: graph identifiers should be searchable against renderer
      resource/pipeline names, with a documented update checklist for new materials and images.

## Exit gate

`docs/spec/material/index.md` links model-specific specifications and contains or embeds a graph
where a reader can trace, node by node and edge by edge, what images and pipelines may exist for a
desktop frame or two XR eyes and exactly which material/configuration predicates activate them.


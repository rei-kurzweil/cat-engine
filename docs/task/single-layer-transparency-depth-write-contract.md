# Define and optimize the single-layer transparency contract

Status: proposed / design decision required

Related:

- `docs/bugs/layout-background-transparency-order-varies-between-launches.md`
- `docs/task/layout-background-multilayer-transparency-correctness.md`
- `docs/task/layout-transparent-background-overlap-classification.md`

## Goal

Decide what `single-layer transparent` means in Mittens and whether that path should enable depth
writes as its defining optimization.

## Contract question

A useful strict definition is:

> Along every covered view ray, the surface is the only transparent layer before the nearest opaque
> surface.

Under that contract, transparent depth writes can reject later transparent fragments and preserve
aggressive batching. The contract is not satisfied merely because an object was independently
tagged “single layer.” It stops being single-layer when another transparent surface is visible in
front of or behind it at the same screen position.

Depth-writing alpha also needs an explicit policy for transparent texture texels. Fully transparent
texels must not become invisible depth occluders; cutout/discard or another documented rule may be
required.

## Questions to resolve

- Should the fast path enable depth writes?
- Is single-layer classification an author promise, a runtime guarantee, or both?
- How should a fast-path surface interact with a multilayer group elsewhere in the scene and when
  their screen coverage begins to overlap?
- Should blended texture holes use discard, a cutout path, or remain unsupported on this path?
- Do names such as `single_layer`, `alpha_depth_write`, and `multiple_layers` communicate the actual
  guarantees clearly enough?
- What should the safe default be for newly authored translucent renderables?

## Work tracker

- [ ] Document the current transparent phases, depth-test settings, depth-write settings, sorting,
      and batching behavior.
- [ ] Define the correctness contract and failure behavior of the fast path.
- [ ] Add fixtures for one transparent layer over opaque geometry, non-overlapping transparent
      objects, overlapping transparent objects, and transparent texture holes.
- [ ] Prototype depth writes on the fast path and compare overdraw, draw calls, and frame time.
- [ ] Validate interaction with the sorted multilayer phase and camera motion.
- [ ] Decide whether existing MMS/runtime terminology requires migration or compatibility aliases.
- [ ] Publish the final authoring and engine-internal classification rules.

## Performance questions

Measure rather than assume:

- GPU fragment work rejected by transparent depth writes in overdraw-heavy scenes.
- CPU sorting and render-stream construction avoided by the fast path.
- Effects on early depth testing, blending, and texture-heavy alpha surfaces.
- Desktop and XR behavior, including the cost of two view-dependent transparent streams.

## Acceptance criteria

- The fast path has a precise, testable meaning.
- Its depth behavior is intentional and covered by renderer tests.
- Incorrect overlap cannot silently masquerade as supported multilayer blending in engine-owned
  content.
- The chosen optimization demonstrates a measurable benefit on a representative workload.

## Non-goals

- Automatically classifying layout-generated backgrounds; that is tracked separately.
- Solving arbitrary intersecting transparent geometry.


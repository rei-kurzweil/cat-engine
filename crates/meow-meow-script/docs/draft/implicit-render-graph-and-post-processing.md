# Implicit render graphs and post-processing composition

Status: deferred design note; do not implement during the RuntimeSpec cutover

## Motivation

Mittens currently requires post-processing components to have a particular
authored topology:

```mms
RenderGraph {
    EmissivePass {
        BlurPass { radius_ndc(0.05) }
    }
    Bloom { intensity(1.2) }
}
```

That topology reflects the current engine registration mechanism more than it
reflects an essential MMS language rule. A friendlier scene may reasonably be
able to say:

```mms
Bloom { intensity(1.2) }
```

and let Mittens create or select the required render-graph resource. In that
model, `RenderGraph` remains available as an explicit grouping and
configuration node, but is not required for the common case.

This is an engine-host semantic question, not a reason to expand the current
`RuntimeSpec` migration. The existing explicit topology remains normative
until this note is resolved and implemented separately.

## Current Mittens behavior

- `RenderGraphComponent` registers the post-processing configuration.
- Registration scans its direct children.
- A direct `BloomComponent` child enables bloom and supplies bloom settings.
- A direct `EmissivePassComponent` child configures the emissive pass.
- A `BlurPassComponent` is read when it is a child of that emissive pass.
- `BloomComponent`, `EmissivePassComponent`, and `BlurPassComponent` do not
  independently register a global post-processing configuration.
- Consequently, a root-level `Bloom {}` currently creates an ECS component
  but does not activate bloom.
- `EmissiveComponent` has useful behavior without bloom: it marks renderable
  content as emissive. Bloom is the optional post-process that spreads that
  emissive contribution into neighboring pixels.
- When an emissive-pass blur is configured, the current renderer uses its blur
  radius in preference to the bloom radius; otherwise it falls back to the
  bloom radius.

## Possible ergonomic model

Treat post-processing declarations as contributions to one resolved render
graph for a render scope:

- A standalone `Bloom {}` requests bloom and causes an implicit graph to
  exist.
- A standalone `EmissivePass {}` requests or configures the emissive source
  pass even when no bloom consumes it, which may still be useful when its
  output texture is published.
- A standalone `BlurPass {}` either needs an explicit input/owner or produces
  a clear error; silently guessing which pass to blur is likely too magical.
- An explicit `RenderGraph { ... }` groups contributions and provides an
  unambiguous place for graph-wide settings.
- Synthetic engine resources need not appear as authored MMS components
  unless reflection and serialization deliberately expose them.

This sugar should be implemented by the Mittens host/engine integration. It
should not add Mittens-specific grammar or post-processing knowledge to the
host-neutral `meow-meow-script` crate.

## Open questions

1. What is the render scope: one graph per `VisualWorld`, window, camera, or
   authored scene root?
2. If both implicit contributions and an explicit `RenderGraph` exist, does
   the explicit graph absorb them, override them, or make the ambiguity an
   error?
3. What happens when multiple bloom declarations exist in one scope: last
   authored wins, merge, multiple passes, or a validation error?
4. Does an `EmissivePass` without bloom render only when an output texture is
   requested, or is it always a meaningful standalone pass?
5. Is a standalone `BlurPass` invalid, implicitly attached to the nearest
   producer, or expressed later with an explicit input/output resource edge?
6. Which setting controls blur when both `BlurPass.radius_ndc` and
   `Bloom.radius_ndc` are authored? The current implementation prefers the
   explicit blur pass, but that precedence needs to be specified.
7. How are graph contributions updated or removed when live components are
   mutated, detached, or destroyed?
8. Should reflection, serialization, and the REPL show an implicit
   `RenderGraph`, or only the declarations the author actually created?
9. Do multiple renderer/window resources require explicit graph selection
   before implicit behavior can be safe?

## Deferred acceptance criteria

- Preserve the current explicit `RenderGraph` form.
- Make the simple standalone-bloom form deterministic and testable.
- Specify multiplicity, precedence, scope, lifetime, and serialization before
  changing runtime behavior.
- Test standalone and combined `Emissive`, `EmissivePass`, `BlurPass`, and
  `Bloom` cases against the actual renderer.
- Land only after the one RuntimeSpec/host boundary is canonical, so this
  behavior is implemented once rather than in both evaluators.


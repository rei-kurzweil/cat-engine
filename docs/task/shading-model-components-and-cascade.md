# Task: Cascading shading-model components

Status: proposed follow-up.

## Purpose

Make the authored material API explicit and low-ceremony. A renderable should receive its shading
model from ordinary component-tree structure, rather than from scattered renderer-specific material
selection rules.

The proposed built-in authored models are:

- `Toon` — the default current toon lighting model.
- `MeowToon` — a future MToon-compatible approximation with its own documented inputs.
- `Unlit` — texture times resolved instance color, with no lighting or bloom contribution.

This is a design task only. It does not add `Toon` or `MeowToon`, change the current `Unlit`
behavior, or introduce a general custom-shader API.

## Intended authoring rule

Shading-model components cascade like CSS through the authored component tree:

1. A model on an ancestor applies to descendant renderables.
2. The nearest applicable ancestor wins.
3. A shading-model component directly beneath a renderable wins over every ancestor.
4. Multiple competing model components at the same precedence level are an explicit authoring
   error, not child-order-dependent behavior.
5. With no authored model component, preserve the current default: `Toon`.

For example:

```mms
Toon {
    R.cube() {}

    Unlit {
        R.sphere() {}
        R.cube() {
            Toon {}
        }
    }
}
```

The sphere is unlit; the nested cube is toon shaded because its direct child is more specific.

## Design constraints

- Resolve one semantic `ShadingModel` before selecting static, skinned, cached-deformed, opaque,
  transparent, cutout, and clipped pipeline variants. Geometry and render-state cross-products are
  renderer details, not authored component names.
- Define interaction with transmission, mirror, grid, text, and implicit-surface outputs instead
  of silently giving one path precedence. Transmission is likely a distinct material family rather
  than a shading-model modifier.
- Apply the same cascade consistently to ordinary renderables, implicit-surface baked visuals, and
  generated glyph/mesh outputs where a component-tree owner exists.
- Keep regular use short: no mandatory `Material {}` wrapper, resource declaration, or shader
  identifier should be required for the built-ins.
- Decide whether `Emissive` remains a separate output modifier or becomes part of a model; preserve
  today’s color/opacity/texture semantics until that decision is made.
- Keep `MeowToon` narrowly specified as an imitation/compatibility target, not an unbounded PBR or
  custom-material facility.

## Implementation sketch

- [ ] Define a small semantic enum and one authoritative cascade resolver.
- [ ] Make `Unlit` use that resolver rather than only the current immediate-child special case.
- [ ] Add `Toon` as an explicit component whose result is identical to the default path.
- [ ] Specify the smallest useful `MeowToon` input set and its fallback behavior.
- [ ] Route every semantic model through existing pipeline-family selection without duplicating
      authoring components for static/skinned/transparent variants.
- [ ] Add diagnostics for conflicts and tests for nearest-ancestor/direct-child precedence.
- [ ] Document the final component API alongside the material renderer resource graph task.

## Acceptance criteria

- An author can set a built-in shading model for a subtree in one component expression.
- Descendant and direct-child override behavior is deterministic, documented, and tested.
- Existing scenes without a model component retain their toon appearance.
- Pipeline selection, implicit surfaces, and generated renderables agree on the resolved semantic
  model.

# Transmissive materials: ECS and MMS authoring contract

Date: 2026-08-30

Status: implemented (renderer integration intentionally deferred)

Parent epic: [Transmissive materials: refraction and rough transmission](epic/transmissive-materials.md)

## Purpose

Define the smallest author-facing and ECS-facing contract for sharp refraction and rough
transmission before adding Vulkan pipelines, scene-color capture, or shaders.

This task resolves the material-instance and MMS spelling portion of Phase 0 in the parent epic. It
does not implement transmission rendering.

## Decisions

1. `Refraction` and `RoughTransmission` are distinct MMS component types and distinct ECS component
   types. They are not modes of one authored component and are not flags on the toon shader.
2. A transmission component is attached directly beneath the `Renderable` it styles.
3. A renderable may have at most one transmission component. Attaching both models, or two copies
   of either model, is an authoring error rather than a last-child-wins rule.
4. Transmission selection does not inherit through arbitrary ancestors in the first slice. This
   keeps material ownership and lifetime unambiguous.
5. Existing `Color` resolution remains responsible for tint and compositing alpha. For a
   transmissive renderable, resolved `C.rgba(r, g, b, a)` means transmission tint `(r, g, b)` and
   output coverage/alpha `a`.
6. The transmission component owns optical inputs: IOR, effective thickness, refraction strength,
   edge fade, and—only for rough transmission—roughness.
7. `MaterialHandle` remains renderer resource identity. The ECS components do not introduce public
   `REFRACTION`, `ROUGH_TRANSMISSION`, or skinned cross-product handle constants.
8. Do not add an `M` MMS shortform or a general shader-material API as part of this task. Full names
   are clear, fit the existing component-expression model, and avoid committing to a material
   namespace before one exists.

## MMS surface

Both components support a bare default form:

```mms
R.cube() {
    C.rgba(0.85, 0.95, 1.0, 0.75)
    Refraction
}
```

```mms
R.cube() {
    C.rgba(0.85, 0.95, 1.0, 0.75)
    RoughTransmission
}
```

Each optical input is also available as a builder call:

```mms
R.cube() {
    C.rgba(0.85, 0.95, 1.0, 0.75)
    Refraction.ior(1.45).thickness(0.08).strength(1.0).edge_fade(0.02)
}
```

```mms
R.cube() {
    C.rgba(0.85, 0.95, 1.0, 0.75)
    RoughTransmission.ior(1.45).thickness(0.08).strength(1.0).edge_fade(0.02).roughness(0.4)
}
```

The builder names are deliberately shared between the two types where their meaning is shared.
`roughness` is not accepted by `Refraction`.

## ECS representation

Use one shared validated parameter block and two authored component types, approximately:

```rust
pub struct TransmissionOptions {
    pub ior: f32,
    pub thickness: f32,
    pub strength: f32,
    pub edge_fade: f32,
}

pub struct RefractionComponent {
    pub options: TransmissionOptions,
}

pub struct RoughTransmissionComponent {
    pub options: TransmissionOptions,
    pub roughness: f32,
}
```

Initial defaults:

- `ior = 1.5`
- `thickness = 0.1`
- `strength = 1.0`
- `edge_fade = 0.02` in normalized viewport coordinates
- `roughness = 0.35`

Construction and builder calls validate at the authoring boundary:

- IOR must be finite and at least `1.0`.
- Thickness and strength must be finite and non-negative.
- Edge fade must be finite and in `0.0..=0.5`.
- Roughness must be finite and in `0.0..=1.0`.

Invalid MMS input should report the component, parameter, supplied value, and accepted range. Do not
silently replace non-finite input or clamp an out-of-range authored value.

Both components implement ordinary component initialization and `to_mms_ast()` serialization. The
serialized form omits values equal to defaults and preserves a stable builder order:
`ior`, `thickness`, `strength`, `edge_fade`, then `roughness`.

## Resolution and renderer handoff

Add one authoritative resolver for a renderable:

```text
Renderable component ID
  -> inspect immediate children
  -> zero transmission children: ordinary material path
  -> exactly one: validated TransmissiveModel plus resolved Color
  -> more than one: explicit authoring error
```

The resolved semantic value should match the parent epic's renderer vocabulary:

```rust
pub enum TransmissiveModel {
    Refraction(RefractionOptions),
    RoughTransmission(RoughTransmissionOptions),
}
```

Static versus cached-deformed/skinned geometry is not part of this value. The renderer will select
that vertex-stage variant independently when the typed material pipeline is implemented.

The first slice may expose this resolved value to tests without assigning a new numeric
`MaterialHandle` or adding a Vulkan pipeline. The later renderer task will carry it into the
dedicated transmissive render-stream phase and material-instance storage.

## Focused implementation slice

- [x] Add the shared options type with defaults and fallible validation.
- [x] Add `RefractionComponent` and `RoughTransmissionComponent` with builder methods.
- [x] Export both components through the ECS component module.
- [x] Register both component types, constructors, and builder signatures in every active MMS
      construction path.
- [x] Add no initialization intent yet: no renderer state consumes this semantic value in this
      slice, so there is nothing to invalidate. Do not add renderer handle switching.
- [x] Implement stable `to_mms_ast()` output for both components.
- [x] Add the immediate-child resolver and duplicate-material error.
- [x] Document both components in the MMS component guide.
- [x] Add focused refraction and rough-transmission desktop examples with four 4:4:1 panels,
      movable cameras, the Kawaii star background, and bloom.
- [x] Add a mixed XR-only example with both material types and no desktop camera or avatar.

## Tests

- Bare `Refraction` and `RoughTransmission` materialize with documented defaults.
- Every builder accepts a valid boundary value and rejects non-finite/out-of-range values.
- `Refraction` rejects `roughness` at MMS validation time.
- Each canonical example parses, materializes, serializes, reparses, and preserves its values.
- The resolver finds one immediate transmission child and ignores transmission components on
  unrelated ancestors.
- Two transmission children produce a deterministic, actionable error.
- Resolved `Color` becomes tint and alpha without changing existing non-transmissive color behavior.
- Static and cached-deformed renderables resolve to the same authored `TransmissiveModel`.
- The three demo scenes materialize with their expected material counts; desktop scenes have one
  `Camera3D`, while the XR-only scene has one `CameraXR` and no `Camera3D`.

## Acceptance criteria

- MMS authors can express both material models with defaults or typed builder calls.
- The ECS owns validated, serializable authored parameters independently of Vulkan resources.
- Material selection has one deterministic immediate-child rule and rejects ambiguity.
- Color/tint semantics and every parameter's units, range, and default are documented.
- No new shader, scene-color image, render pass, public material-handle cross-product, or general
  custom-material facility is introduced.

## Follow-up

Once this contract is implemented, continue with Phase 0 of the parent epic: introduce typed
material definitions and the dedicated transmissive render-stream phase. After that boundary is
stable, implement the same-frame per-view opaque scene snapshot and diagnostic sampling shader.

## Stop condition

Stop when both material forms round-trip through MMS, all three demo scenes materialize, resolution
is deterministic from a renderable's immediate children, and validated semantic material data is
exposed for the later renderer task. Any
Vulkan work, animation API, shared mutable material instances, shader hot reload, or custom shader
authoring belongs to a follow-up task.

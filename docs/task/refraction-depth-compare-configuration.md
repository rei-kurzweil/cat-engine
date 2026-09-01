# Task: Optional refraction foreground-depth comparison

Status: scoped; implement before changing Bloom/depth interaction.

## Purpose

Expose the current foreground-depth rejection as an authored per-material switch so the same scene
can compare the original displaced screen-space lookup with the depth-aware lookup:

```mms
Refraction.depth_compare(false) {}
```

The default remains `true`. Existing scenes must retain the current depth-aware behavior when the
builder is absent.

This first slice is an A/B control, not a resource optimization. It must keep creating and binding
the existing scene-depth snapshot even when every visible refraction disables the comparison. Once
the visual behavior is understood, a separate slice may aggregate visible material requirements
and omit depth preparation for an all-disabled view.

## Observed Bloom boundary

The current depth comparison successfully removes the solid center of foreground opaque and
emissive-opaque objects from displaced refraction lookups, but foreground-origin Bloom can remain
visible in the refracted result.

This follows from the current resources rather than showing that the geometry-depth comparison did
not run:

```text
foreground emissive geometry ----> scene depth at the geometry silhouette
             |
             +--> emissive extraction --> spatial blur -----+
                                                           |
opaque scene color + blurred Bloom ------------------------+--> refraction color snapshot
opaque/cutout geometry depth ---------------------------------> refraction depth snapshot
```

Bloom is composited into `snapshot.color` before refraction in
`VulkanoState::build_draw_batches_command_buffer`. The matching `snapshot.depth` still represents
geometry, not the source ownership of blurred color. Inside the solid foreground object's
silhouette, candidate depth is nearer than the transmissive fragment and the lookup is rejected.
Outside that silhouette, the blur halo may occupy color pixels whose depth belongs to background
geometry. Those candidates pass the comparison, so the foreground-origin glow remains eligible for
refraction.

Fixing that halo requires a separate representation or policy, such as source-depth/ownership
metadata for Bloom, depth-aware Bloom compositing, or foreground/background Bloom separation. Do
not choose one of those designs in this task. The toggle exists so those later changes can be
compared against both current modes.

## Authoring contract

- `Refraction.depth_compare(bool)` is a constructor/builder call.
- The default is `true`.
- Serialization omits the call when `true` and emits `.depth_compare(false)` when disabled.
- The setting is stored in common `TransmissionOptions`, allowing
  `RoughTransmission.depth_compare(bool)` to use the same contract when that renderer path exists.
- `false` means the shader uses the displaced, clamped UV without consulting scene depth. It may
  therefore pull foreground opaque/emissive geometry into the refractive surface.
- The switch affects candidate acceptance only. It does not enable/disable Bloom inclusion, change
  snapshot resolution, change the foreground bias, or recover hidden background color.

## Implementation seams

### 1. Component model and MMS serialization

`src/engine/ecs/component/transmission.rs`

- Add `depth_compare: bool` to `TransmissionOptions` with a default of `true`.
- Add a boolean-specific builder path shared by `Refraction` and `RoughTransmission`; the current
  `apply_builder(..., f32)` API cannot receive this call.
- Serialize the non-default value with `ce_helpers::b(false)`.
- Extend component default, builder, validation, resolver, and round-trip tests.

Do not encode the flag into the sign or a magic value of IOR, thickness, strength, or edge fade.
Those four floats already have authored meanings and validation contracts.

### 2. Strict and compatibility MMS registries

`src/scripting/runtime_config.rs`

- Declare `depth_compare` with a one-boolean signature for both `Refraction` and
  `RoughTransmission`.

`src/scripting/configured_registry.rs`

- Route `depth_compare` through `bool_arg`; the existing direct transmission path assumes every
  call has one `f32` argument.

`src/scripting/component_registry.rs`

- Route the compatibility path through `arg_bool` before its current numeric transmission
  dispatch.

Add strict RuntimeSpec and compatibility parsing tests so the method cannot work in only one MMS
execution path.

### 3. ECS-to-visual propagation

`src/engine/ecs/system/renderable_system.rs`

- Propagate the resolved option alongside the current four-float transmission payload for static
  and cached-deformed refraction materials.

`src/engine/ecs/system/implicit_surface_system.rs`

- Preserve the option for implicit surfaces using `Refraction`; do not leave this path with an
  accidental hard-coded default.

`src/engine/graphics/visual_world.rs`

- Add a distinct transmission flags field to `VisualInstance` and mark instance data dirty when it
  changes.
- Define a named `DEPTH_COMPARE` bit rather than exposing an unstructured numeric convention to
  component systems.

### 4. Instance and shader interface

`src/engine/graphics/vulkano_renderer.rs`

- Add a `u32` transmission-flags member to `InstanceData` and populate every instance-buffer
  construction path.
- Add the matching per-instance vertex attribute to the static vertex input layout. The current
  transmission vector occupies location 12 at byte offset 96; use a separately documented
  location/offset and update layout tests rather than repacking that vector.

`assets/shaders/toon-mesh.vert` and `assets/shaders/cached-skinned-toon-mesh.vert`

- Forward the flag as a flat integer varying in the shared mesh-surface interface.

`assets/shaders/refraction-mesh.frag`

- Only sample `scene_depth` and apply the foreground fallback when `DEPTH_COMPARE` is set.
- Preserve the existing one scene-color lookup in both modes.
- Preserve edge fade, viewport clamping, tint, and alpha behavior.

Use one refraction pipeline for this first slice. A separate depth-unaware pipeline/batch variant is
not justified until GPU measurement shows the flat per-instance branch is significant.

### 5. Renderer resource behavior

For this task, do not change:

- `WindowRefractionTargets` allocation;
- color/depth resolve or copy ordering;
- global descriptor-set binding 3;
- the Bloom-before-refraction composite; or
- the activation predicate for scene snapshots.

Keeping those stable makes `depth_compare(true)` versus `false` a shader-behavior A/B rather than a
comparison that also changes passes, synchronization, or image allocation.

## A/B fixture

Update `examples/refraction.mms` so the setting can be compared without changing shader code:

- retain at least one default/explicit `depth_compare(true)` object;
- add or designate a comparable `depth_compare(false)` object;
- keep both grabbable so each can move in front of and behind the opaque card and emissive lines;
- compare solid foreground geometry and the Bloom halo separately; and
- label the modes in scene names or nearby text so screenshots are unambiguous.

The expected current result is:

| Candidate source | Depth compare on | Depth compare off |
| --- | --- | --- |
| Opaque/emissive core in front | displaced lookup rejected | displaced foreground color appears |
| Bloom halo outside source silhouette | usually remains eligible | remains eligible |
| Geometry behind transmission | displaced lookup accepted | displaced lookup accepted |

## Acceptance criteria

- [ ] Existing `Refraction` authoring without the builder behaves exactly as
      `depth_compare(true)`.
- [ ] `Refraction.depth_compare(false)` parses through strict and compatibility MMS paths and
      round-trips through component serialization.
- [ ] Mixed enabled/disabled refractive objects can share one view and one refraction snapshot.
- [ ] Enabled fragments retain the current foreground rejection and disabled fragments skip the
      depth texture read/fallback.
- [ ] Static, cached-deformed, and implicit-surface refraction preserve the authored flag.
- [ ] The desktop example provides a labeled, grabbable A/B comparison for opaque cores and Bloom
      halos.
- [ ] Focused component, scripting, visual-world, shader-interface, and transmissive example tests
      pass with MSAA on and off.
- [ ] No claim is made that this switch fixes foreground-origin Bloom.

## Deferred follow-ups

- Aggregate per-view depth requirements and omit the depth snapshot/resolve/copy when all visible
  transmissive materials disable comparison.
- Design and test a representation that associates blurred Bloom contribution with source depth or
  foreground/background ownership.
- Apply the chosen policy per XR eye after XR transmission snapshots exist.
- Apply the same acceptance rule to rough-transmission filtering without blurring rejected
  foreground contribution back into valid samples.

## Related work

- [Foreground-depth leakage](refraction-foreground-depth-leakage.md)
- [Transmission quality, depth, and MSAA configuration review](../review/transmissive-quality-depth-and-msaa-configuration.md)
- [Refraction material specification](../spec/material/refraction.md)
- [Bloom-before-refraction capture](refraction-postprocess-composite-capture.md)

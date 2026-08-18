# mittens-engine 0.8.0 release notes (draft)

## Breaking change: humanoid mapping is GLTF-owned

`AvatarControl` no longer accepts the legacy per-bone-name configuration
surface. The old `BoneMappingSystem` and AVC bone-name fields/builders have
been removed rather than retained as compatibility aliases.

Humanoid semantics now belong to one `HumanoidBoneMap` associated with the
avatar's GLTF. `AvatarControl` consumes the resulting retained report; it does
not search arbitrary component names itself.

### Migration

For conventionally named humanoid rigs, remove the repeated bone-name
configuration. AVC requests the default Auto map once the GLTF skin is ready:

```mms
AVC {
  T { GLTF.new("avatar.glb") {} }
}
```

For rigs that need overrides, attach one `HumanoidBoneMap` below the GLTF and
express only the exceptional slots:

```mms
GLTF.new("avatar.glb") {
  HumanoidBoneMap.new()
    .slot("left_hand", "#wrist_l")
    .slot("camera_anchor", "#camera_socket")
    .absent("neck")
}
```

An explicit `.slot(...)` or `.absent(...)` wins over inference; use
`.automap_disable()` when every populated slot must be authored deliberately.
Reusable presets are available under `assets/components/humanoid_bone_maps/`.

See [Rigging and controlling humanoid characters](../how_to/rigging_and_controlling_humanoid_characters.md)
for the supported slot model, resolver behavior, and AVC requirements.

## Still required before publication

This note records the API migration only. The 0.8 release remains gated on the
selected desktop/headset smoke checks and the editor grid, paint, and accordion
reliability work listed in the release roadmap.

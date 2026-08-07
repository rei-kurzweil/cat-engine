# Rigging and controlling humanoid characters

`AvatarControl` (AVC) consumes a semantic humanoid map owned by the avatar's GLTF. For a
conventionally named rig, no bone strings are needed:

```mms
InputXR.on() {
  T {
    AVC {
      T { GLTF.new("avatar.glb") {} }
      CTLXR.new(true, Left, GripAim) { T {} }
      CTLXR.new(true, Right, GripAim) { T {} }
      CameraXR {}
    }
  }
}
```

AVC requests implicit Auto mapping after the GLTF skin joints are available. Head must validate
before AVC mutates the armature. Left and right arms activate independently when upper arm, lower
arm, and hand validate. A missing neck disables only the optional rest-pin.

For unusual rigs, author exceptions beneath the GLTF. Explicit references and absences override
Auto:

```mms
GLTF.new("avatar.glb") {
  HumanoidBoneMap.new()
    .slot("head", "#custom_head")
    .slot("left_hand", "#wrist_l")
    .absent("neck")
}
```

Call `.automap_disable()` when every unspecified slot must remain unresolved. Explicitly mapped
eye and hand landmarks can still produce deterministic eye-midpoint and hand-basis values in this
mode.

Reusable VRoid, Bisket, and PC-Rei factories are under
`assets/components/humanoid_bone_maps/`. The named avatar factories share the VRoid convention but
remain separately discoverable.

The hand basis uses middle-proximal to middle-distal for anatomical forward and little-proximal to
index-proximal for palm width. An authored `JointRetargetBasis` for the same hand is an expert
override and takes precedence. AVC applies the retained correction absolutely during controller,
grip/aim, and articulated-hand source changes, so correction never accumulates frame to frame.

Camera anchoring resolves an explicit arbitrary transform first, then a semantic central anchor,
then a generated eye midpoint, and finally the mapped head. A bad explicit camera selector stays
visible in diagnostics while the fallback remains operational.

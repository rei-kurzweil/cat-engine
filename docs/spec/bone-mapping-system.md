# Humanoid bone-map resolver

Date: 2026-08-06

The former `BoneMappingSystem` was an unused stateless helper and was never integrated with AVC.
It has been removed. Humanoid resolution now lives in `HumanoidBoneMapSystem` and retains one
generation-numbered report per owning GLTF.

The resolver consumes `HumanoidBoneMapComponent`, restricts anatomical results to that GLTF's skin
joints, requires unique explicit selectors, and records ambiguity instead of selecting the first
match. AVC consumes resolved slot IDs, not bone-name fields. An AVC with no authored map requests an
implicit Auto report.

See [the implementation task](../task/humanoid-bone-map-automapping-and-mms-presets.md) for the
public slots, precedence rules, lifecycle, camera order, hand-basis landmarks, and release scope.

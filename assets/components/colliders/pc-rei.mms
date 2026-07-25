// Explicit PC-Rei spring collision volumes. The model contains similarly named
// imported helper nodes, but secondary motion binds only authored colliders.
export fn pc_rei_colliders() {
    return SpringColliders {
        SpringCollider.sphere("[name='J_Bip_C_Head']", 0.11) { name = "pc_rei_collider_head" }
        SpringCollider.sphere("[name='J_Bip_C_Neck']", 0.055) { name = "pc_rei_collider_neck" }
        SpringCollider.sphere("[name='J_Bip_C_UpperChest']", 0.075) { name = "pc_rei_collider_upper_chest" }
        SpringCollider.sphere("[name='J_Bip_C_Spine']", 0.09) { name = "pc_rei_collider_spine" }
        SpringCollider.sphere("[name='J_Bip_C_Hips']", 0.13) { name = "pc_rei_collider_hips" }
        SpringCollider.spheres(["[name='J_Bip_L_Hand']", "[name='J_Bip_R_Hand']"], 0.045) { name = "pc_rei_colliders_hands" }
        SpringCollider.spheres(["[name='J_Bip_L_UpperArm']", "[name='J_Bip_R_UpperArm']"], 0.05) { name = "pc_rei_colliders_upper_arms" }
    }
}

// PC-Rei's authored secondary skeleton contains five two-joint hair chains,
// two ear chains, two bust chains, and a single tail root with a virtual end.
export fn pc_rei_secondary_motion() {
    let hair_roots = [
        "[name='hair-mid-top']",
        "[name='hair-left-front-top']",
        "[name='hair-right-front-top']",
        "[name='hair-left-back-top']",
        "[name='hair-right-back-top']",
    ]
    let ear_roots = [
        "[name='ear-left-bottom']",
        "[name='ear-right-bottom']",
    ]

    return SecondaryMotion {
        for root in hair_roots {
            SpringBone.from_root(root)
                .virtual_end_length_ratio(1.0)
                .stiffness(1.0)
                .drag_force(0.35)
                .gravity(3.0, [0, -1, 0])
                .colliders(["[name='pc_rei_collider_head']", "[name='pc_rei_collider_neck']", "[name='pc_rei_collider_upper_chest']"])
                .hit_radius(0.015)
        }
        for root in ear_roots {
            SpringBone.from_root(root)
                .virtual_end_length_ratio(1.0)
                .stiffness(1.5)
                .drag_force(0.45)
                .gravity(1.5, [0, -1, 0])
                .colliders(["[name='pc_rei_collider_head']"])
                .hit_radius(0.012)
        }
        SpringBone.from_root("[name='J_Sec_L_Bust1']")
            .virtual_end_length_ratio(1.0)
            .stiffness(4.0)
            .drag_force(0.60)
            .gravity(0.35, [0, -1, 0])
            .colliders(["[name='pc_rei_collider_upper_chest']", "[name='pc_rei_collider_spine']", "[name='pc_rei_colliders_hands']", "[name='pc_rei_colliders_upper_arms']"])
            .hit_radius(0.025)
        SpringBone.from_root("[name='J_Sec_R_Bust1']")
            .virtual_end_length_ratio(1.0)
            .stiffness(4.0)
            .drag_force(0.60)
            .gravity(0.35, [0, -1, 0])
            .colliders(["[name='pc_rei_collider_upper_chest']", "[name='pc_rei_collider_spine']", "[name='pc_rei_colliders_hands']", "[name='pc_rei_colliders_upper_arms']"])
            .hit_radius(0.025)
        SpringBone.from_root("[name='canine-tail']")
            .virtual_end_length_ratio(1.0)
            .stiffness(1.0)
            .drag_force(0.25)
            .gravity(0.8, [0, -1, 0])
            .colliders(["[name='pc_rei_collider_spine']", "[name='pc_rei_collider_hips']"])
            .hit_radius(0.035)
    }
}

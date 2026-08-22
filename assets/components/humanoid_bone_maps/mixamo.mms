// Mixamo humanoid landmark convention, including its explicit eye joints.
export fn mixamo_humanoid_bone_map() {
    return HumanoidBoneMap.new()
        .slot("head", "[name='mixamorig:Head']")
        .slot("left_eye", "[name='mixamorig:LeftEye']")
        .slot("right_eye", "[name='mixamorig:RightEye']")
        .slot("left_upper_arm", "[name='mixamorig:LeftArm']")
        .slot("left_lower_arm", "[name='mixamorig:LeftForeArm']")
        .slot("left_hand", "[name='mixamorig:LeftHand']")
        .slot("right_upper_arm", "[name='mixamorig:RightArm']")
        .slot("right_lower_arm", "[name='mixamorig:RightForeArm']")
        .slot("right_hand", "[name='mixamorig:RightHand']")
}

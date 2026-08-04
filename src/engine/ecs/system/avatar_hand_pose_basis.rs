use crate::engine::ecs::component::{
    BoneRestPoseComponent, ComponentRef, GLTFComponent, TransformComponent,
};
use crate::engine::ecs::{ComponentId, World};
use crate::engine::graphics::primitives::Transform;
use crate::utils::math::{
    mat_to_quat, mat4_identity, mat4_mul, shortest_arc_quat, vec3_cross, vec3_dot, vec3_len,
    vec3_normalize, vec3_scale, vec3_sub,
};

/// Immutable avatar-authored fingertip frame, expressed relative to the hand bone.
/// Local `-Z` follows the configured middle-finger direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvatarHandPoseBasis {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}

/// Rest-pose anatomical directions expressed in the imported hand bone's frame.
/// These are diagnostic measurements only; they do not affect the applied basis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvatarPalmDiagnostics {
    pub distal_forward: [f32; 3],
    pub whole_middle_forward: [f32; 3],
    pub palm_longitudinal: [f32; 3],
    pub thumbward: [f32; 3],
    pub little_to_index: [f32; 3],
}

/// Resolve and derive the configured middle-finger frame from GLTF rest-pose data.
/// `Ok(None)` means the GLTF nodes have not been spawned yet and the caller should retry.
pub fn resolve_avatar_hand_pose_basis(
    world: &World,
    model_root: ComponentId,
    hand_bone: ComponentId,
    finger: &[ComponentRef; 3],
    hand_up: Option<&ComponentRef>,
    palm_width: Option<&[ComponentRef; 2]>,
) -> Result<Option<AvatarHandPoseBasis>, String> {
    let gltf_id = find_descendant_gltf(world, model_root).ok_or("avatar GLTF was not found")?;
    let gltf = world
        .get_component_by_id_as::<GLTFComponent>(gltf_id)
        .ok_or("avatar GLTF disappeared")?;
    if gltf.spawned_node_transforms.is_empty() {
        return Ok(None);
    }

    let resolve = |reference: &ComponentRef| -> Result<ComponentId, String> {
        let matches: Vec<_> = gltf
            .spawned_node_transforms
            .iter()
            .copied()
            .filter(|id| match reference {
                ComponentRef::Guid(guid) => world.component_id_by_guid(*guid) == Some(*id),
                ComponentRef::Query(query) => world.component_matches_selector(*id, query),
            })
            .collect();
        if matches.len() != 1 {
            return Err(format!(
                "finger selector {} matched {} avatar nodes (expected exactly one)",
                component_ref_surface(reference),
                matches.len()
            ));
        }
        Ok(matches[0])
    };

    let root = resolve(&finger[0])?;
    let middle = resolve(&finger[1])?;
    let tip = resolve(&finger[2])?;
    let hand_up = hand_up.map(resolve).transpose()?;
    let palm_width = match palm_width {
        Some([index, little]) => Some([resolve(index)?, resolve(little)?]),
        None => None,
    };
    derive_avatar_hand_pose_basis(world, hand_bone, [root, middle, tip], hand_up, palm_width)
        .map(Some)
}

pub fn derive_avatar_hand_pose_basis(
    world: &World,
    hand_bone: ComponentId,
    [root, middle, tip]: [ComponentId; 3],
    hand_up: Option<ComponentId>,
    palm_width: Option<[ComponentId; 2]>,
) -> Result<AvatarHandPoseBasis, String> {
    if !is_descendant(world, middle, root) || !is_descendant(world, tip, middle) {
        return Err("configured finger joints are not an ancestral root/middle/tip chain".into());
    }
    // Resolve the root as well as the final segment to validate that the complete
    // configured chain belongs to this hand.
    let root_model = rest_model_relative_to(world, hand_bone, root)
        .ok_or("root finger joint is not beneath the AVC hand bone")?;
    let middle_model = rest_model_relative_to(world, hand_bone, middle)
        .ok_or("middle finger joint is not beneath the AVC hand bone")?;
    let tip_model = rest_model_relative_to(world, hand_bone, tip)
        .ok_or("tip finger joint is not beneath the AVC hand bone")?;
    let root_position = [root_model[3][0], root_model[3][1], root_model[3][2]];
    let middle_position = [middle_model[3][0], middle_model[3][1], middle_model[3][2]];
    let tip_position = [tip_model[3][0], tip_model[3][1], tip_model[3][2]];
    let finger_segment = if palm_width.is_some() {
        vec3_sub(tip_position, root_position)
    } else {
        vec3_sub(tip_position, middle_position)
    };
    if vec3_len(finger_segment) <= 1e-6 {
        return Err("finger's selected rest-space direction has zero length".into());
    }
    let direction = vec3_normalize(finger_segment);
    let up_landmark_direction = if let Some([index, little]) = palm_width {
        let index_model = rest_model_relative_to(world, hand_bone, index)
            .ok_or("index-root landmark is not beneath the AVC hand bone")?;
        let little_model = rest_model_relative_to(world, hand_bone, little)
            .ok_or("little-root landmark is not beneath the AVC hand bone")?;
        let index_position = [index_model[3][0], index_model[3][1], index_model[3][2]];
        let little_position = [little_model[3][0], little_model[3][1], little_model[3][2]];
        Some(vec3_sub(index_position, little_position))
    } else if let Some(hand_up) = hand_up {
        let hand_up_model = rest_model_relative_to(world, hand_bone, hand_up)
            .ok_or("hand-up landmark is not beneath the AVC hand bone")?;
        let hand_up_position = [
            hand_up_model[3][0],
            hand_up_model[3][1],
            hand_up_model[3][2],
        ];
        Some(vec3_sub(hand_up_position, root_position))
    } else {
        None
    };
    let rotation = if let Some(up_landmark_direction) = up_landmark_direction {
        let projected_up = vec3_sub(
            up_landmark_direction,
            vec3_scale(direction, vec3_dot(up_landmark_direction, direction)),
        );
        if vec3_len(projected_up) <= 1e-6 {
            return Err("palm-up landmark is collinear with the finger direction".into());
        }
        let y = vec3_normalize(projected_up);
        let z = vec3_scale(direction, -1.0);
        let x = vec3_normalize(vec3_cross(y, z));
        let y = vec3_normalize(vec3_cross(z, x));
        mat_to_quat([
            [x[0], x[1], x[2], 0.0],
            [y[0], y[1], y[2], 0.0],
            [z[0], z[1], z[2], 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    } else {
        shortest_arc_quat([0.0, 0.0, -1.0], direction)
    };
    Ok(AvatarHandPoseBasis {
        position: tip_position,
        rotation,
    })
}

/// Resolve extra palm landmarks and measure competing orientation axes without
/// changing the basis used by AVC. `Ok(None)` means GLTF initialization is pending.
pub fn resolve_avatar_palm_diagnostics(
    world: &World,
    model_root: ComponentId,
    hand_bone: ComponentId,
    finger: &[ComponentRef; 3],
    thumb_root: &ComponentRef,
    palm: &[ComponentRef; 2],
) -> Result<Option<AvatarPalmDiagnostics>, String> {
    let gltf_id = find_descendant_gltf(world, model_root).ok_or("avatar GLTF was not found")?;
    let gltf = world
        .get_component_by_id_as::<GLTFComponent>(gltf_id)
        .ok_or("avatar GLTF disappeared")?;
    if gltf.spawned_node_transforms.is_empty() {
        return Ok(None);
    }

    let resolve = |reference: &ComponentRef| -> Result<ComponentId, String> {
        let matches: Vec<_> = gltf
            .spawned_node_transforms
            .iter()
            .copied()
            .filter(|id| match reference {
                ComponentRef::Guid(guid) => world.component_id_by_guid(*guid) == Some(*id),
                ComponentRef::Query(query) => world.component_matches_selector(*id, query),
            })
            .collect();
        if matches.len() != 1 {
            return Err(format!(
                "palm diagnostic selector {} matched {} avatar nodes (expected exactly one)",
                component_ref_surface(reference),
                matches.len()
            ));
        }
        Ok(matches[0])
    };
    let [root, middle, tip] = [
        resolve(&finger[0])?,
        resolve(&finger[1])?,
        resolve(&finger[2])?,
    ];
    let thumb = resolve(thumb_root)?;
    let index = resolve(&palm[0])?;
    let little = resolve(&palm[1])?;

    let position = |bone: ComponentId| -> Result<[f32; 3], String> {
        let model = rest_model_relative_to(world, hand_bone, bone)
            .ok_or("palm diagnostic landmark is not beneath the AVC hand bone")?;
        Ok([model[3][0], model[3][1], model[3][2]])
    };
    let root = position(root)?;
    let middle = position(middle)?;
    let tip = position(tip)?;
    let thumb = position(thumb)?;
    let index = position(index)?;
    let little = position(little)?;
    let direction = |from: [f32; 3], to: [f32; 3], label: &str| {
        let value = vec3_sub(to, from);
        if vec3_len(value) <= 1e-6 {
            Err(format!("{label} diagnostic direction has zero length"))
        } else {
            Ok(vec3_normalize(value))
        }
    };

    Ok(Some(AvatarPalmDiagnostics {
        distal_forward: direction(middle, tip, "distal middle")?,
        whole_middle_forward: direction(root, tip, "whole middle")?,
        palm_longitudinal: direction([0.0; 3], root, "hand-to-middle-root")?,
        thumbward: direction(root, thumb, "middle-root-to-thumb-root")?,
        little_to_index: direction(little, index, "little-root-to-index-root")?,
    }))
}

fn component_ref_surface(reference: &ComponentRef) -> String {
    match reference {
        ComponentRef::Guid(guid) => format!("@uuid:{guid}"),
        ComponentRef::Query(query) => query.clone(),
    }
}

fn find_descendant_gltf(world: &World, root: ComponentId) -> Option<ComponentId> {
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if world.get_component_by_id_as::<GLTFComponent>(id).is_some() {
            return Some(id);
        }
        stack.extend_from_slice(world.children_of(id));
    }
    None
}

fn is_descendant(world: &World, mut id: ComponentId, ancestor: ComponentId) -> bool {
    for _ in 0..64 {
        if id == ancestor {
            return true;
        }
        let Some(parent) = world.parent_of(id) else {
            return false;
        };
        id = parent;
    }
    false
}

fn rest_model_relative_to(
    world: &World,
    ancestor: ComponentId,
    descendant: ComponentId,
) -> Option<[[f32; 4]; 4]> {
    let mut ids = Vec::new();
    let mut current = Some(descendant);
    while let Some(id) = current {
        if id == ancestor {
            ids.reverse();
            return Some(ids.into_iter().fold(mat4_identity(), |model, id| {
                let local = world
                    .children_of(id)
                    .iter()
                    .find_map(|child| world.get_component_by_id_as::<BoneRestPoseComponent>(*child))
                    .map(|rest| {
                        let mut transform = Transform::default();
                        transform.translation = rest.translation;
                        transform.rotation = rest.rotation;
                        transform.scale = rest.scale;
                        transform.recompute_model();
                        transform.model
                    })
                    .or_else(|| {
                        world
                            .get_component_by_id_as::<TransformComponent>(id)
                            .map(|transform| transform.transform.model)
                    })
                    .unwrap_or_else(mat4_identity);
                mat4_mul(model, local)
            }));
        }
        if world
            .get_component_by_id_as::<TransformComponent>(id)
            .is_some()
        {
            ids.push(id);
        }
        current = world.parent_of(id);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::math::{quat_conjugate, quat_mul, quat_rotate_vec3};

    fn basis_for(direction: [f32; 3]) -> AvatarHandPoseBasis {
        let mut world = World::default();
        let hand = world.add_component(TransformComponent::new());
        let root = world.add_component(TransformComponent::new());
        let middle = world.add_component(TransformComponent::new());
        let tip = world.add_component(TransformComponent::new());
        world.add_child(hand, root).unwrap();
        world.add_child(root, middle).unwrap();
        world.add_child(middle, tip).unwrap();
        for (bone, translation) in [(root, direction), (middle, direction), (tip, direction)] {
            let rest = world.add_component(BoneRestPoseComponent::new(
                translation,
                [0.0, 0.0, 0.0, 1.0],
                [1.0; 3],
            ));
            world.add_child(bone, rest).unwrap();
        }
        derive_avatar_hand_pose_basis(&world, hand, [root, middle, tip], None, None).unwrap()
    }

    #[test]
    fn correction_aim_aligns_plus_x_plus_y_and_minus_z_finger_bases() {
        for direction in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]] {
            let basis = basis_for(direction);
            let correction = quat_conjugate(basis.rotation);
            let final_rotation = quat_mul(correction, basis.rotation);
            let final_forward = quat_rotate_vec3(final_rotation, [0.0, 0.0, -1.0]);
            for axis in 0..3 {
                assert!((final_forward[axis] - [0.0, 0.0, -1.0][axis]).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn thumb_landmark_resolves_forward_and_roll() {
        let mut world = World::default();
        let hand = world.add_component(TransformComponent::new());
        let root = world.add_component(TransformComponent::new());
        let middle = world.add_component(TransformComponent::new());
        let tip = world.add_component(TransformComponent::new());
        let thumb = world.add_component(TransformComponent::new());
        world.add_child(hand, root).unwrap();
        world.add_child(root, middle).unwrap();
        world.add_child(middle, tip).unwrap();
        world.add_child(hand, thumb).unwrap();
        for (bone, translation) in [
            (root, [0.0, 1.0, 0.0]),
            (middle, [0.0, 1.0, 0.0]),
            (tip, [0.0, 1.0, 0.0]),
            (thumb, [1.0, 1.0, 0.0]),
        ] {
            let rest = world.add_component(BoneRestPoseComponent::new(
                translation,
                [0.0, 0.0, 0.0, 1.0],
                [1.0; 3],
            ));
            world.add_child(bone, rest).unwrap();
        }

        let basis =
            derive_avatar_hand_pose_basis(&world, hand, [root, middle, tip], Some(thumb), None)
                .unwrap();
        let authored_forward = quat_rotate_vec3(basis.rotation, [0.0, 0.0, -1.0]);
        let authored_up = quat_rotate_vec3(basis.rotation, [0.0, 1.0, 0.0]);
        for axis in 0..3 {
            assert!((authored_forward[axis] - [0.0, 1.0, 0.0][axis]).abs() < 1e-5);
            assert!((authored_up[axis] - [1.0, 0.0, 0.0][axis]).abs() < 1e-5);
        }
        let correction = quat_conjugate(basis.rotation);
        let final_rotation = quat_mul(correction, basis.rotation);
        let final_forward = quat_rotate_vec3(final_rotation, [0.0, 0.0, -1.0]);
        let final_up = quat_rotate_vec3(final_rotation, [0.0, 1.0, 0.0]);
        for axis in 0..3 {
            assert!((final_forward[axis] - [0.0, 0.0, -1.0][axis]).abs() < 1e-5);
            assert!((final_up[axis] - [0.0, 1.0, 0.0][axis]).abs() < 1e-5);
        }
    }

    #[test]
    fn knuckle_width_overrides_thumb_root_for_palm_roll() {
        let mut world = World::default();
        let hand = world.add_component(TransformComponent::new());
        let root = world.add_component(TransformComponent::new());
        let middle = world.add_component(TransformComponent::new());
        let tip = world.add_component(TransformComponent::new());
        let thumb = world.add_component(TransformComponent::new());
        let index = world.add_component(TransformComponent::new());
        let little = world.add_component(TransformComponent::new());
        world.add_child(hand, root).unwrap();
        world.add_child(root, middle).unwrap();
        world.add_child(middle, tip).unwrap();
        world.add_child(hand, thumb).unwrap();
        world.add_child(hand, index).unwrap();
        world.add_child(hand, little).unwrap();
        for (bone, translation) in [
            (root, [0.0, 1.0, 0.0]),
            (middle, [0.0, 1.0, 0.0]),
            (tip, [0.0, 1.0, 0.0]),
            (thumb, [0.0, 0.0, 1.0]),
            (index, [1.0, 1.0, 0.0]),
            (little, [-1.0, 1.0, 0.0]),
        ] {
            let rest = world.add_component(BoneRestPoseComponent::new(
                translation,
                [0.0, 0.0, 0.0, 1.0],
                [1.0; 3],
            ));
            world.add_child(bone, rest).unwrap();
        }

        let basis = derive_avatar_hand_pose_basis(
            &world,
            hand,
            [root, middle, tip],
            Some(thumb),
            Some([index, little]),
        )
        .unwrap();
        let authored_forward = quat_rotate_vec3(basis.rotation, [0.0, 0.0, -1.0]);
        let authored_up = quat_rotate_vec3(basis.rotation, [0.0, 1.0, 0.0]);
        for axis in 0..3 {
            assert!((authored_forward[axis] - [0.0, 1.0, 0.0][axis]).abs() < 1e-5);
            assert!((authored_up[axis] - [1.0, 0.0, 0.0][axis]).abs() < 1e-5);
        }
    }
}

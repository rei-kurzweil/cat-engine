use crate::engine::ecs::component::{
    BoneRestPoseComponent, ComponentRef, GLTFComponent, TransformComponent,
};
use crate::engine::ecs::{ComponentId, World};
use crate::engine::graphics::primitives::Transform;
use crate::utils::math::{mat4_identity, mat4_mul};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedRestAttachment {
    pub anchor: ComponentId,
    pub target: ComponentId,
    pub anchor_to_target_rest: [[f32; 4]; 4],
}

/// Resolve a declaration strictly within one imported GLTF and calculate its immutable rest
/// transform. `Ok(None)` means node import is still in flight.
pub fn resolve_rest_attachment(
    world: &World,
    owner: ComponentId,
    anchor: &ComponentRef,
    target: &ComponentRef,
) -> Result<Option<ResolvedRestAttachment>, String> {
    let gltf = world
        .get_component_by_id_as::<GLTFComponent>(owner)
        .ok_or("rest attachment owner is not a GLTF")?;
    if gltf.spawned_node_transforms.is_empty() {
        return Ok(None);
    }
    let anchor = resolve_in_gltf(world, gltf, anchor)?;
    let target = resolve_in_gltf(world, gltf, target)?;
    let anchor_to_target_rest = rest_model_relative_to(world, anchor, target)
        .ok_or("rest attachment target is not beneath its anchor")?;
    Ok(Some(ResolvedRestAttachment {
        anchor,
        target,
        anchor_to_target_rest,
    }))
}

pub fn find_descendant_gltf(world: &World, root: ComponentId) -> Option<ComponentId> {
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if world.get_component_by_id_as::<GLTFComponent>(id).is_some() {
            return Some(id);
        }
        stack.extend_from_slice(world.children_of(id));
    }
    None
}

fn resolve_in_gltf(
    world: &World,
    gltf: &GLTFComponent,
    reference: &ComponentRef,
) -> Result<ComponentId, String> {
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
            "rest attachment reference {} matched {} imported nodes (expected exactly one)",
            component_ref_surface(reference),
            matches.len()
        ));
    }
    Ok(matches[0])
}

fn component_ref_surface(reference: &ComponentRef) -> String {
    match reference {
        ComponentRef::Guid(guid) => format!("@uuid:{guid}"),
        ComponentRef::Query(query) => query.clone(),
    }
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

    #[test]
    fn computes_rest_offset_for_an_ordinary_imported_transform() {
        let mut world = World::default();
        let gltf_id = world.add_component(GLTFComponent::new("test.glb"));
        let anchor = world.add_component_boxed_named("anchor", Box::new(TransformComponent::new()));
        let target = world.add_component_boxed_named("target", Box::new(TransformComponent::new()));
        world.add_child(gltf_id, anchor).unwrap();
        world.add_child(anchor, target).unwrap();
        let rest = world.add_component(BoneRestPoseComponent::new(
            [1.0, 2.0, 3.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0; 3],
        ));
        world.add_child(target, rest).unwrap();
        world
            .get_component_by_id_as_mut::<GLTFComponent>(gltf_id)
            .unwrap()
            .spawned_node_transforms = vec![anchor, target];

        let resolved = resolve_rest_attachment(
            &world,
            gltf_id,
            &ComponentRef::Query("#anchor".into()),
            &ComponentRef::Query("#target".into()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(resolved.anchor, anchor);
        assert_eq!(resolved.target, target);
        assert_eq!(
            [
                resolved.anchor_to_target_rest[3][0],
                resolved.anchor_to_target_rest[3][1],
                resolved.anchor_to_target_rest[3][2],
            ],
            [1.0, 2.0, 3.0]
        );
    }
}

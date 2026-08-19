use crate::engine::ecs::component::{
    BoundsComponent, ColorComponent, ComponentRef, EditorComponent, EmissiveComponent,
    GLTFComponent, MeshComponent, OpacityComponent, OverlayComponent, RaycastableComponent,
    RenderableComponent, SelectableComponent, SerializeComponent, TransformComponent,
    TransformParentComponent,
};
use crate::engine::ecs::system::{GLTFSystem, MeshBoundsSystem, MeshOutputKind};
use crate::engine::ecs::{ComponentId, IntentValue, SignalEmitter, World};
use crate::engine::graphics::RenderAssets;
use crate::engine::graphics::VisualWorld;
use std::collections::{HashMap, HashSet};

const BOUNDS_EDGE_THICKNESS: f32 = 0.01;
const BOUNDS_EMISSIVE_INTENSITY: f32 = 2.0;
const BOUNDS_OPACITY: f32 = 0.95;

#[derive(Debug, Clone, Copy)]
struct BoundsMarker {
    target: ComponentId,
    root: ComponentId,
}

/// Draws imported GLTF and generated mesh-output bounds without inserting
/// debug nodes into authored hierarchies.
#[derive(Debug, Default)]
pub struct GltfBoundsVisualizationSystem {
    markers: HashMap<ComponentId, Vec<BoundsMarker>>,
    mesh_output_markers: HashMap<ComponentId, BoundsMarker>,
}

impl GltfBoundsVisualizationSystem {
    pub fn tick_with_queue(
        &mut self,
        world: &mut World,
        gltf_system: &GLTFSystem,
        mesh_bounds: &MeshBoundsSystem,
        mesh_bounds_visible: bool,
        _visuals: &mut VisualWorld,
        render_assets: &mut RenderAssets,
        emit: &mut dyn SignalEmitter,
    ) {
        self.cleanup(world);

        for gltf_id in gltf_system.tracked_components() {
            let Some(gltf) = world.get_component_by_id_as::<GLTFComponent>(gltf_id) else {
                continue;
            };
            if !gltf.spawned {
                continue;
            }
            if gltf.bounds_visible {
                self.ensure_markers(world, render_assets, emit, gltf_id);
            } else {
                self.remove_markers(emit, gltf_id);
            }
        }

        self.sync_mesh_output_markers(world, render_assets, emit, mesh_bounds, mesh_bounds_visible);
    }

    fn cleanup(&mut self, world: &World) {
        self.markers.retain(|gltf_id, markers| {
            if world.get_component_record(*gltf_id).is_none() {
                return false;
            }
            markers.retain(|marker| {
                world.get_component_record(marker.root).is_some()
                    && world.get_component_record(marker.target).is_some()
            });
            true
        });
    }

    fn ensure_markers(
        &mut self,
        world: &mut World,
        render_assets: &mut RenderAssets,
        emit: &mut dyn SignalEmitter,
        gltf_id: ComponentId,
    ) {
        let existing_targets: HashSet<ComponentId> = self
            .markers
            .get(&gltf_id)
            .into_iter()
            .flatten()
            .map(|marker| marker.target)
            .collect();
        let mut stack = world
            .get_component_by_id_as::<GLTFComponent>(gltf_id)
            .map(|gltf| gltf.spawned_node_transforms.clone())
            .unwrap_or_default();
        let mut additions = Vec::new();

        while let Some(node_transform) = stack.pop() {
            let children = world.children_of(node_transform).to_vec();
            for child in children {
                if world
                    .get_component_by_id_as::<TransformComponent>(child)
                    .is_some()
                {
                    stack.push(child);
                }
                if existing_targets.contains(&child)
                    || world
                        .get_component_by_id_as::<RenderableComponent>(child)
                        .and_then(RenderableComponent::get_handle)
                        .is_none()
                    || !world.children_of(child).iter().any(|&sidecar| {
                        world
                            .get_component_by_id_as::<MeshComponent>(sidecar)
                            .is_some()
                    })
                {
                    continue;
                }
                let Some(bounds) = world.children_of(child).iter().find_map(|&sidecar| {
                    world
                        .get_component_by_id_as::<BoundsComponent>(sidecar)
                        .map(|bounds| bounds.local)
                }) else {
                    continue;
                };
                additions.push(spawn_marker(world, render_assets, emit, child, bounds));
            }
        }

        self.markers.entry(gltf_id).or_default().extend(additions);
    }

    fn remove_markers(&mut self, emit: &mut dyn SignalEmitter, gltf_id: ComponentId) {
        let Some(markers) = self.markers.get_mut(&gltf_id) else {
            return;
        };
        for marker in markers.drain(..) {
            emit.push_intent_now(
                marker.root,
                IntentValue::RemoveSubtree {
                    component_id: marker.root,
                },
            );
        }
    }

    fn sync_mesh_output_markers(
        &mut self,
        world: &mut World,
        render_assets: &mut RenderAssets,
        emit: &mut dyn SignalEmitter,
        mesh_bounds: &MeshBoundsSystem,
        visible: bool,
    ) {
        let outputs: Vec<_> = mesh_bounds
            .outputs()
            .filter(|output| {
                output.kind == MeshOutputKind::CombineMesh
                    && is_within_editor_scope(world, output.owner)
            })
            .collect();
        let live_owners: HashSet<_> = outputs.iter().map(|output| output.owner).collect();

        let stale: Vec<_> = self
            .mesh_output_markers
            .iter()
            .filter_map(|(&owner, marker)| {
                (!live_owners.contains(&owner)
                    || world.get_component_record(marker.root).is_none()
                    || world.get_component_record(marker.target).is_none())
                .then_some((owner, marker.root))
            })
            .collect();
        for (owner, root) in stale {
            self.mesh_output_markers.remove(&owner);
            emit.push_intent_now(root, IntentValue::RemoveSubtree { component_id: root });
        }

        if !visible {
            for marker in self.mesh_output_markers.values() {
                emit.push_intent_now(
                    marker.root,
                    IntentValue::RemoveSubtree {
                        component_id: marker.root,
                    },
                );
            }
            self.mesh_output_markers.clear();
            return;
        }

        for output in outputs {
            if self.mesh_output_markers.contains_key(&output.owner)
                || world.get_component_record(output.owner).is_none()
            {
                continue;
            }
            let marker = spawn_marker(world, render_assets, emit, output.owner, output.local);
            self.mesh_output_markers.insert(output.owner, marker);
        }
    }
}

/// Bounds overlays are an editor aid.  Generated outputs only qualify when
/// their stable owner lives below an authored `ED {}` root.
fn is_within_editor_scope(world: &World, owner: ComponentId) -> bool {
    let mut current = Some(owner);
    while let Some(component) = current {
        if world
            .get_component_by_id_as::<EditorComponent>(component)
            .is_some()
        {
            return true;
        }
        current = world.parent_of(component);
    }
    false
}

fn spawn_marker(
    world: &mut World,
    render_assets: &mut RenderAssets,
    emit: &mut dyn SignalEmitter,
    target: ComponentId,
    bounds: crate::engine::graphics::bounds::Aabb,
) -> BoundsMarker {
    let center = bounds.center();
    let local = TransformComponent::new()
        .with_position(center[0], center[1], center[2])
        .with_scale(bounds.width(), bounds.height(), bounds.depth());
    let target_guid = world
        .get_component_record(target)
        .expect("bounds target must exist")
        .guid;
    let root = world.add_component_boxed_named(
        "mesh_bounds_marker",
        Box::new(
            TransformParentComponent::new().with_target_source(ComponentRef::Guid(target_guid)),
        ),
    );
    let local = world.add_component(local);
    let selectable = world.add_component(SelectableComponent::off());
    let serialize = world.add_component(SerializeComponent::off());
    let overlay = world.add_component(OverlayComponent::new());
    let renderable = world.add_component(RenderableComponent::wireframe_box(
        render_assets,
        BOUNDS_EDGE_THICKNESS,
    ));
    let raycastable = world.add_component(RaycastableComponent::disabled());
    let color = world.add_component(ColorComponent::rgba(1.0, 0.35, 0.015, 1.0));
    let emissive = world.add_component(EmissiveComponent::new(BOUNDS_EMISSIVE_INTENSITY));
    let opacity = world.add_component(OpacityComponent::new().with_opacity(BOUNDS_OPACITY));
    let _ = world.add_child(root, local);
    let _ = world.add_child(root, selectable);
    let _ = world.add_child(root, serialize);
    let _ = world.add_child(local, overlay);
    let _ = world.add_child(overlay, renderable);
    let _ = world.add_child(renderable, raycastable);
    let _ = world.add_child(renderable, color);
    let _ = world.add_child(renderable, emissive);
    let _ = world.add_child(renderable, opacity);
    world.init_component_tree(root, emit);

    BoundsMarker { target, root }
}

use crate::engine::ecs::component::{
    Camera3DComponent, CameraXRComponent, ComponentRef, DraggableComponent, DraggablePlane,
    DraggableTarget, QueryRootMode, RaycastableComponent, SelectableComponent, SerializeComponent,
    TransformComponent, parse_scoped_query,
};
use crate::engine::ecs::system::TransformSystem;
use crate::engine::ecs::{ComponentId, SignalEmitter, World};
use crate::engine::ecs::{EventSignal, IntentValue, PointerActivationSource, RxWorld, SignalKind};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

type GestureKey = (u8, ComponentId, ComponentId);

#[derive(Debug, Default)]
pub struct DraggableSystem {
    handlers_installed: bool,
    active: Arc<Mutex<HashMap<GestureKey, Option<ResolvedDraggable>>>>,
    diagnostics: Arc<Mutex<HashSet<String>>>,
}

impl DraggableSystem {
    pub fn install_handlers(&mut self, rx: &mut RxWorld) {
        if self.handlers_installed {
            return;
        }
        let active = Arc::clone(&self.active);
        let diagnostics = Arc::clone(&self.diagnostics);
        rx.add_global_handler_closure(SignalKind::DragStart, move |world, _emit, env| {
            let Some(EventSignal::DragStart {
                activation_source,
                raycaster,
                renderable,
                ..
            }) = env.event.as_ref()
            else {
                return;
            };
            if *activation_source != PointerActivationSource::Trigger {
                return;
            }
            let key = gesture_key(*activation_source, *raycaster, *renderable);
            let resolved = resolve_draggable_for_hit(world, *renderable, &diagnostics);
            active
                .lock()
                .expect("draggable gesture lock")
                .insert(key, resolved);
        });

        let active = Arc::clone(&self.active);
        let diagnostics = Arc::clone(&self.diagnostics);
        rx.add_global_handler_closure(SignalKind::DragMove, move |world, emit, env| {
            let Some(EventSignal::DragMove {
                activation_source,
                raycaster,
                renderable,
                delta_world,
                screen_pos_px: _,
                ..
            }) = env.event.as_ref()
            else {
                return;
            };
            let supported_activation = *activation_source == PointerActivationSource::Trigger;
            if !supported_activation {
                return;
            }
            let key = gesture_key(*activation_source, *raycaster, *renderable);
            let resolved = active
                .lock()
                .expect("draggable gesture lock")
                .get(&key)
                .copied()
                .unwrap_or_else(|| resolve_draggable_for_hit(world, *renderable, &diagnostics));
            let Some(resolved) = resolved else {
                return;
            };
            let owner = resolved.target;
            let local_delta = constrained_parent_local_delta(
                world,
                owner,
                resolved.plane,
                *delta_world,
                Some(*raycaster),
            );
            let Some(transform) = world.get_component_by_id_as::<TransformComponent>(owner) else {
                return;
            };
            let mut translation = transform.transform.translation;
            for i in 0..3 {
                translation[i] += local_delta[i];
            }
            emit.push_intent_now(
                owner,
                IntentValue::UpdateTransform {
                    component_id: owner,
                    translation,
                    rotation_quat_xyzw: transform.transform.rotation,
                    scale: transform.transform.scale,
                },
            );
        });
        let active = Arc::clone(&self.active);
        rx.add_global_handler_closure(SignalKind::DragEnd, move |_world, _emit, env| {
            let Some(EventSignal::DragEnd {
                activation_source,
                raycaster,
                renderable,
                ..
            }) = env.event.as_ref()
            else {
                return;
            };
            active
                .lock()
                .expect("draggable gesture lock")
                .remove(&gesture_key(*activation_source, *raycaster, *renderable));
        });
        self.handlers_installed = true;
    }

    pub fn register(
        &mut self,
        world: &mut World,
        draggable: ComponentId,
        emit: &mut dyn SignalEmitter,
    ) {
        if world
            .get_component_by_id_as::<DraggableComponent>(draggable)
            .is_some_and(|component| !component.enabled)
        {
            return;
        }
        let Some(owner) = world.parent_of(draggable).filter(|id| {
            world
                .get_component_by_id_as::<TransformComponent>(*id)
                .is_some()
        }) else {
            return;
        };
        cache_explicit_target(world, draggable, owner, &self.diagnostics);
        let has_immediate_raycastable = world.children_of(owner).iter().any(|child| {
            world
                .get_component_by_id_as::<RaycastableComponent>(*child)
                .is_some()
        });
        if has_immediate_raycastable {
            return;
        }
        let raycastable = world.add_component_boxed_named(
            "draggable_generated_raycastable",
            Box::new(RaycastableComponent::enabled()),
        );
        let serialize = world.add_component(SerializeComponent::off());
        let _ = world.add_child(raycastable, serialize);
        if world.add_child(owner, raycastable).is_ok() {
            world.init_component_tree(raycastable, emit);
        }
    }
}

fn world_delta_to_parent_local(
    world: &World,
    owner: ComponentId,
    delta_world: [f32; 3],
) -> [f32; 3] {
    let Some(parent) = world.parent_of(owner) else {
        return delta_world;
    };
    let Some(parent_world) = TransformSystem::world_model(world, parent) else {
        return delta_world;
    };
    let Some(inv) = crate::utils::math::mat4_inverse(parent_world) else {
        return delta_world;
    };
    [
        inv[0][0] * delta_world[0] + inv[1][0] * delta_world[1] + inv[2][0] * delta_world[2],
        inv[0][1] * delta_world[0] + inv[1][1] * delta_world[1] + inv[2][1] * delta_world[2],
        inv[0][2] * delta_world[0] + inv[1][2] * delta_world[1] + inv[2][2] * delta_world[2],
    ]
}

fn constrained_parent_local_delta(
    world: &World,
    target: ComponentId,
    plane: DraggablePlane,
    delta_world: [f32; 3],
    raycaster: Option<ComponentId>,
) -> [f32; 3] {
    match plane {
        DraggablePlane::Object => {
            let mut local = world_delta_to_parent_local(world, target, delta_world);
            local[2] = 0.0;
            local
        }
        DraggablePlane::Camera => {
            let constrained_world = raycaster
                .and_then(|raycaster| camera_plane_world_axes(world, raycaster))
                .map(|axes| project_onto_world_axes(delta_world, axes))
                .unwrap_or(delta_world);
            world_delta_to_parent_local(world, target, constrained_world)
        }
        DraggablePlane::WorldAxes(axes) => {
            let projected_world = project_onto_world_axes(delta_world, axes);
            world_delta_to_parent_local(world, target, projected_world)
        }
    }
}

fn camera_plane_world_axes(world: &World, raycaster: ComponentId) -> Option<[[f32; 3]; 2]> {
    let mut current = Some(raycaster);
    while let Some(id) = current {
        if world
            .get_component_by_id_as::<Camera3DComponent>(id)
            .is_some()
            || world
                .get_component_by_id_as::<CameraXRComponent>(id)
                .is_some()
        {
            return TransformSystem::world_model(world, id).map(world_xy_axes);
        }
        current = world.parent_of(id);
    }

    world
        .all_components()
        .find(|id| {
            world
                .get_component_by_id_as::<CameraXRComponent>(*id)
                .is_some_and(|camera| camera.enabled)
        })
        .and_then(|camera| TransformSystem::world_model(world, camera))
        .map(world_xy_axes)
}

fn world_xy_axes(model: [[f32; 4]; 4]) -> [[f32; 3]; 2] {
    [
        [model[0][0], model[0][1], model[0][2]],
        [model[1][0], model[1][1], model[1][2]],
    ]
}

fn project_onto_world_axes(delta: [f32; 3], axes: [[f32; 3]; 2]) -> [f32; 3] {
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let normalize = |v: [f32; 3]| {
        let length = dot(v, v).sqrt();
        [v[0] / length, v[1] / length, v[2] / length]
    };
    let first = normalize(axes[0]);
    let second_rejected = {
        let along_first = dot(axes[1], first);
        [
            axes[1][0] - first[0] * along_first,
            axes[1][1] - first[1] * along_first,
            axes[1][2] - first[2] * along_first,
        ]
    };
    let second = normalize(second_rejected);
    let first_amount = dot(delta, first);
    let second_amount = dot(delta, second);
    [
        first[0] * first_amount + second[0] * second_amount,
        first[1] * first_amount + second[1] * second_amount,
        first[2] * first_amount + second[2] * second_amount,
    ]
}

#[derive(Debug, Clone, Copy)]
struct ResolvedDraggable {
    target: ComponentId,
    plane: DraggablePlane,
}

/// Resolve a renderable hit to its draggable movement target.
///
/// An enabled Selectable sidecar on the same owner wins over Draggable. Handle-style
/// `Draggable.parent()` markers resolve to the owner's parent Transform.
pub fn draggable_owner_for_hit(world: &World, renderable: ComponentId) -> Option<ComponentId> {
    let diagnostics = Arc::new(Mutex::new(HashSet::new()));
    resolve_draggable_for_hit_readonly(world, renderable, &diagnostics)
        .map(|resolved| resolved.target)
}

fn gesture_key(
    activation_source: PointerActivationSource,
    raycaster: ComponentId,
    renderable: ComponentId,
) -> GestureKey {
    let source = match activation_source {
        PointerActivationSource::Trigger => 0,
        PointerActivationSource::Grip => 1,
    };
    (source, raycaster, renderable)
}

#[derive(Debug, Clone, Copy)]
struct DraggableMarker {
    marker: ComponentId,
    owner: ComponentId,
    plane: DraggablePlane,
}

fn find_draggable_marker(world: &World, renderable: ComponentId) -> Option<DraggableMarker> {
    let mut current = Some(renderable);
    while let Some(id) = current {
        if world
            .get_component_by_id_as::<TransformComponent>(id)
            .is_some()
        {
            let selectable_wins = world
                .get_component_by_id_as::<SelectableComponent>(id)
                .is_some_and(|selectable| selectable.enabled)
                || world.children_of(id).iter().any(|child| {
                    world
                        .get_component_by_id_as::<SelectableComponent>(*child)
                        .is_some_and(|selectable| selectable.enabled)
                });
            if selectable_wins {
                return None;
            }
            if let Some((marker, draggable)) = world.children_of(id).iter().find_map(|child| {
                world
                    .get_component_by_id_as::<DraggableComponent>(*child)
                    .map(|component| (*child, component))
            }) {
                if !draggable.enabled {
                    return None;
                }
                return Some(DraggableMarker {
                    marker,
                    owner: id,
                    plane: draggable.plane,
                });
            }
        }
        current = world.parent_of(id);
    }
    None
}

fn resolve_draggable_for_hit(
    world: &mut World,
    renderable: ComponentId,
    diagnostics: &Arc<Mutex<HashSet<String>>>,
) -> Option<ResolvedDraggable> {
    let marker = find_draggable_marker(world, renderable)?;
    let target = resolve_marker_motion_target(world, marker.marker, marker.owner, diagnostics)?;
    Some(ResolvedDraggable {
        target,
        plane: marker.plane,
    })
}

fn resolve_draggable_for_hit_readonly(
    world: &World,
    renderable: ComponentId,
    diagnostics: &Arc<Mutex<HashSet<String>>>,
) -> Option<ResolvedDraggable> {
    let marker = find_draggable_marker(world, renderable)?;
    let component = world.get_component_by_id_as::<DraggableComponent>(marker.marker)?;
    let target = match &component.target {
        DraggableTarget::Owner => Some(marker.owner),
        DraggableTarget::ParentTransform => nearest_parent_transform(world, marker.owner),
        DraggableTarget::Explicit(reference) => component
            .target_id
            .filter(|id| is_transform(world, *id))
            .or_else(|| resolve_explicit_unique(world, marker.owner, reference)),
    };
    if target.is_none() {
        diagnose_target(diagnostics, marker.marker, &component.target);
    }
    target.map(|target| ResolvedDraggable {
        target,
        plane: marker.plane,
    })
}

fn resolve_marker_motion_target(
    world: &mut World,
    marker: ComponentId,
    owner: ComponentId,
    diagnostics: &Arc<Mutex<HashSet<String>>>,
) -> Option<ComponentId> {
    let (mode, cached, was_bound) = {
        let component = world.get_component_by_id_as::<DraggableComponent>(marker)?;
        (
            component.target.clone(),
            component.target_id,
            component.target_was_bound,
        )
    };
    match mode {
        DraggableTarget::Owner => Some(owner),
        DraggableTarget::ParentTransform => nearest_parent_transform(world, owner),
        DraggableTarget::Explicit(reference) => {
            if let Some(target) = cached.filter(|id| is_transform(world, *id)) {
                return Some(target);
            }
            if cached.is_some()
                && let Some(component) =
                    world.get_component_by_id_as_mut::<DraggableComponent>(marker)
            {
                component.target_id = None;
            }
            if matches!(reference, ComponentRef::Guid(_)) && was_bound {
                diagnose_target(diagnostics, marker, &DraggableTarget::Explicit(reference));
                return None;
            }
            let resolved = resolve_explicit_unique(world, owner, &reference)
                .filter(|id| is_transform(world, *id));
            if let Some(component) = world.get_component_by_id_as_mut::<DraggableComponent>(marker)
            {
                component.target_id = resolved;
                component.target_was_bound |= resolved.is_some();
            }
            if resolved.is_none() {
                diagnose_target(diagnostics, marker, &DraggableTarget::Explicit(reference));
            }
            resolved
        }
    }
}

fn cache_explicit_target(
    world: &mut World,
    marker: ComponentId,
    owner: ComponentId,
    diagnostics: &Arc<Mutex<HashSet<String>>>,
) {
    let _ = resolve_marker_motion_target(world, marker, owner, diagnostics);
}

fn nearest_parent_transform(world: &World, owner: ComponentId) -> Option<ComponentId> {
    let mut current = world.parent_of(owner);
    while let Some(candidate) = current {
        if is_transform(world, candidate) {
            return Some(candidate);
        }
        current = world.parent_of(candidate);
    }
    None
}

fn is_transform(world: &World, id: ComponentId) -> bool {
    world
        .get_component_by_id_as::<TransformComponent>(id)
        .is_some()
}

fn resolve_explicit_unique(
    world: &World,
    owner: ComponentId,
    reference: &ComponentRef,
) -> Option<ComponentId> {
    match reference {
        ComponentRef::Guid(guid) => world.component_id_by_guid(*guid),
        ComponentRef::Query(query) => {
            let scoped = parse_scoped_query(query);
            let mut roots = Vec::new();
            match scoped.root_mode {
                QueryRootMode::SelfSubtree => roots.push(owner),
                QueryRootMode::ParentScope { levels_up } => {
                    let mut root = owner;
                    for _ in 0..levels_up {
                        root = world.parent_of(root)?;
                    }
                    roots.push(root);
                }
                QueryRootMode::WorldRoot => roots.extend(world.world_roots()),
            }
            let selector = scoped.selector.trim();
            if selector.is_empty() {
                return None;
            }
            let mut matches: Vec<_> = roots
                .into_iter()
                .flat_map(|root| world.find_all_components(root, selector))
                .collect();
            matches.sort_by_key(|id| format!("{id:?}"));
            matches.dedup();
            (matches.len() == 1).then(|| matches[0])
        }
    }
}

fn diagnose_target(
    diagnostics: &Arc<Mutex<HashSet<String>>>,
    marker: ComponentId,
    target: &DraggableTarget,
) {
    let message =
        format!("[Draggable] marker {marker:?} has no unique Transform target for {target:?}");
    if diagnostics
        .lock()
        .expect("draggable diagnostics lock")
        .insert(message.clone())
    {
        eprintln!("{message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ecs::component::{RenderableComponent, SelectableComponent};
    use crate::engine::ecs::{CommandQueue, Signal};

    #[test]
    fn registration_generates_runtime_raycastable_and_nested_hits_resolve_to_owner() {
        let mut world = World::default();
        let owner = world.add_component(TransformComponent::new());
        let draggable = world.add_component(DraggableComponent::new());
        let nested = world.add_component(TransformComponent::new());
        let deep = world.add_component(TransformComponent::new());
        let renderable = world.add_component(RenderableComponent::cube());
        world.add_child(owner, draggable).unwrap();
        world.add_child(owner, nested).unwrap();
        world.add_child(nested, deep).unwrap();
        world.add_child(deep, renderable).unwrap();

        let mut queue = CommandQueue::new();
        let mut system = DraggableSystem::default();
        system.register(&mut world, draggable, &mut queue);

        let generated = world
            .children_of(owner)
            .iter()
            .copied()
            .find(|child| world.component_label(*child) == Some("draggable_generated_raycastable"))
            .expect("generated raycastable");
        assert!(
            world
                .get_component_by_id_as::<RaycastableComponent>(generated)
                .is_some()
        );
        assert!(world.children_of(generated).iter().any(|child| {
            world
                .get_component_by_id_as::<SerializeComponent>(*child)
                .is_some_and(|serialize| !serialize.enabled)
        }));
        assert_eq!(draggable_owner_for_hit(&world, renderable), Some(owner));
    }

    #[test]
    fn explicit_owner_raycastable_is_preserved() {
        let mut world = World::default();
        let owner = world.add_component(TransformComponent::new());
        let draggable = world.add_component(DraggableComponent::new());
        let explicit = world.add_component(RaycastableComponent::disabled());
        world.add_child(owner, draggable).unwrap();
        world.add_child(owner, explicit).unwrap();
        let mut queue = CommandQueue::new();
        DraggableSystem::default().register(&mut world, draggable, &mut queue);
        assert_eq!(
            world
                .children_of(owner)
                .iter()
                .filter(|child| {
                    world
                        .get_component_by_id_as::<RaycastableComponent>(**child)
                        .is_some()
                })
                .count(),
            1
        );
        assert!(
            !world
                .get_component_by_id_as::<RaycastableComponent>(explicit)
                .unwrap()
                .enable
        );
    }

    #[test]
    fn selectable_on_wins_over_draggable_but_selectable_off_does_not() {
        let mut world = World::default();
        let owner = world.add_component(TransformComponent::new());
        let draggable = world.add_component(DraggableComponent::new());
        let selectable = world.add_component(SelectableComponent::on());
        let renderable = world.add_component(RenderableComponent::cube());
        world.add_child(owner, draggable).unwrap();
        world.add_child(owner, selectable).unwrap();
        world.add_child(owner, renderable).unwrap();

        assert_eq!(draggable_owner_for_hit(&world, renderable), None);
        world
            .get_component_by_id_as_mut::<SelectableComponent>(selectable)
            .unwrap()
            .enabled = false;
        assert_eq!(draggable_owner_for_hit(&world, renderable), Some(owner));
    }

    #[test]
    fn parent_handle_resolves_to_the_complete_parent_transform() {
        let mut world = World::default();
        let panel = world.add_component(TransformComponent::new());
        let title_bar = world.add_component(TransformComponent::new());
        let draggable = world.add_component(DraggableComponent::parent());
        let renderable = world.add_component(RenderableComponent::cube());
        world.add_child(panel, title_bar).unwrap();
        world.add_child(title_bar, draggable).unwrap();
        world.add_child(title_bar, renderable).unwrap();

        assert_eq!(draggable_owner_for_hit(&world, renderable), Some(panel));
    }

    #[test]
    fn explicit_target_moves_only_the_uniquely_resolved_transform() {
        let mut world = World::default();
        let scope = world.add_component(TransformComponent::new());
        let owner = world.add_component(TransformComponent::new());
        let target = world.add_component_boxed_named("target", Box::new(TransformComponent::new()));
        let draggable = world.add_component(DraggableComponent::explicit(ComponentRef::Query(
            "../#target".to_string(),
        )));
        let renderable = world.add_component(RenderableComponent::cube());
        world.add_child(scope, owner).unwrap();
        world.add_child(scope, target).unwrap();
        world.add_child(owner, draggable).unwrap();
        world.add_child(owner, renderable).unwrap();

        let mut rx = RxWorld::default();
        DraggableSystem::default().install_handlers(&mut rx);
        let raycaster = ComponentId::default();
        rx.dispatch_event_handlers(
            &mut world,
            &Signal::event(
                renderable,
                EventSignal::DragStart {
                    activation_source: PointerActivationSource::Trigger,
                    raycaster,
                    renderable,
                    hit_point: [0.0; 3],
                    ray_dir_world: [0.0, 0.0, -1.0],
                    screen_pos_px: None,
                },
            ),
        );
        rx.dispatch_event_handlers(
            &mut world,
            &Signal::event(
                renderable,
                EventSignal::DragMove {
                    activation_source: PointerActivationSource::Trigger,
                    raycaster,
                    renderable,
                    hit_point: [0.0; 3],
                    delta_world: [0.5, 0.25, 1.0],
                    screen_pos_px: None,
                    screen_delta_px: None,
                },
            ),
        );
        let intents = rx.drain_ready_intents();
        assert!(intents.iter().any(|signal| matches!(
            signal.intent.as_ref().map(|intent| &intent.value),
            Some(IntentValue::UpdateTransform { component_id, translation, .. })
                if *component_id == target && *translation == [0.5, 0.25, 0.0]
        )));
        assert!(!intents.iter().any(|signal| matches!(
            signal.intent.as_ref().map(|intent| &intent.value),
            Some(IntentValue::UpdateTransform { component_id, .. }) if *component_id == owner
        )));
    }

    #[test]
    fn selector_targets_stay_cached_while_live_and_rebind_after_deletion() {
        let mut world = World::default();
        let scope = world.add_component(TransformComponent::new());
        let owner = world.add_component(TransformComponent::new());
        let first = world.add_component_boxed_named("target", Box::new(TransformComponent::new()));
        let marker = world.add_component(DraggableComponent::explicit(ComponentRef::Query(
            "../#target".to_string(),
        )));
        world.add_child(scope, owner).unwrap();
        world.add_child(scope, first).unwrap();
        world.add_child(owner, marker).unwrap();
        let diagnostics = Arc::new(Mutex::new(HashSet::new()));

        assert_eq!(
            resolve_marker_motion_target(&mut world, marker, owner, &diagnostics),
            Some(first)
        );
        world.get_component_record_mut(first).unwrap().name.clear();
        let replacement =
            world.add_component_boxed_named("target", Box::new(TransformComponent::new()));
        world.add_child(scope, replacement).unwrap();
        assert_eq!(
            resolve_marker_motion_target(&mut world, marker, owner, &diagnostics),
            Some(first),
            "a live cached selector target must remain sticky"
        );

        world.remove_component_subtree(first).unwrap();
        assert_eq!(
            resolve_marker_motion_target(&mut world, marker, owner, &diagnostics),
            Some(replacement),
            "a selector may bind its replacement after deletion"
        );
    }

    #[test]
    fn guid_targets_do_not_rebind_after_the_bound_component_is_deleted() {
        let mut world = World::default();
        let owner = world.add_component(TransformComponent::new());
        let target = world.add_component(TransformComponent::new());
        let guid = world.get_component_record(target).unwrap().guid;
        let marker = world.add_component(DraggableComponent::explicit(ComponentRef::Guid(guid)));
        world.add_child(owner, marker).unwrap();
        let diagnostics = Arc::new(Mutex::new(HashSet::new()));

        assert_eq!(
            resolve_marker_motion_target(&mut world, marker, owner, &diagnostics),
            Some(target)
        );
        world.remove_component_subtree(target).unwrap();
        let replacement = world.add_component(TransformComponent::new());
        world.get_component_record_mut(replacement).unwrap().guid = guid;
        assert_eq!(
            resolve_marker_motion_target(&mut world, marker, owner, &diagnostics),
            None
        );
    }

    #[test]
    fn captured_target_does_not_switch_during_an_active_gesture() {
        let mut world = World::default();
        let scope = world.add_component(TransformComponent::new());
        let owner = world.add_component(TransformComponent::new());
        let target = world.add_component_boxed_named("target", Box::new(TransformComponent::new()));
        let marker = world.add_component(DraggableComponent::explicit(ComponentRef::Query(
            "../#target".to_string(),
        )));
        let renderable = world.add_component(RenderableComponent::cube());
        world.add_child(scope, owner).unwrap();
        world.add_child(scope, target).unwrap();
        world.add_child(owner, marker).unwrap();
        world.add_child(owner, renderable).unwrap();

        let mut rx = RxWorld::default();
        DraggableSystem::default().install_handlers(&mut rx);
        let raycaster = ComponentId::default();
        rx.dispatch_event_handlers(
            &mut world,
            &Signal::event(
                renderable,
                EventSignal::DragStart {
                    activation_source: PointerActivationSource::Trigger,
                    raycaster,
                    renderable,
                    hit_point: [0.0; 3],
                    ray_dir_world: [0.0, 0.0, -1.0],
                    screen_pos_px: None,
                },
            ),
        );
        world.remove_component_subtree(target).unwrap();
        let replacement =
            world.add_component_boxed_named("target", Box::new(TransformComponent::new()));
        world.add_child(scope, replacement).unwrap();
        rx.dispatch_event_handlers(
            &mut world,
            &Signal::event(
                renderable,
                EventSignal::DragMove {
                    activation_source: PointerActivationSource::Trigger,
                    raycaster,
                    renderable,
                    hit_point: [0.0; 3],
                    delta_world: [1.0, 0.0, 0.0],
                    screen_pos_px: None,
                    screen_delta_px: None,
                },
            ),
        );
        assert!(rx.drain_ready_intents().iter().all(|signal| !matches!(
            signal.intent.as_ref().map(|intent| &intent.value),
            Some(IntentValue::UpdateTransform { component_id, .. }) if *component_id == replacement
        )));
    }

    #[test]
    fn desktop_and_xr_trigger_drag_move_draggable() {
        let mut world = World::default();
        let owner = world.add_component(TransformComponent::new());
        let draggable = world.add_component(DraggableComponent::new());
        let renderable = world.add_component(RenderableComponent::cube());
        world.add_child(owner, draggable).unwrap();
        world.add_child(owner, renderable).unwrap();

        let mut rx = RxWorld::default();
        DraggableSystem::default().install_handlers(&mut rx);
        let drag = |screen_pos_px| {
            Signal::event(
                renderable,
                EventSignal::DragMove {
                    activation_source: PointerActivationSource::Trigger,
                    raycaster: ComponentId::default(),
                    renderable,
                    hit_point: [1.0, 2.0, 3.0],
                    delta_world: [0.25, -0.5, 1.0],
                    screen_pos_px,
                    screen_delta_px: Some((4.0, 8.0)),
                },
            )
        };

        rx.dispatch_event_handlers(&mut world, &drag(Some((20.0, 30.0))));
        let intents = rx.drain_ready_intents();
        assert!(intents.iter().any(|signal| matches!(
            signal.intent.as_ref().map(|intent| &intent.value),
            Some(IntentValue::UpdateTransform { component_id, translation, .. })
                if component_id == &owner
                    && *translation == [0.25, -0.5, 0.0]
        )));

        rx.dispatch_event_handlers(&mut world, &drag(None));
        assert!(rx.drain_ready_intents().iter().any(|signal| matches!(
            signal.intent.as_ref().map(|intent| &intent.value),
            Some(IntentValue::UpdateTransform { component_id, .. }) if component_id == &owner
        )));
    }

    #[test]
    fn camera_and_world_axis_planes_constrain_deltas() {
        let mut world = World::default();
        let owner = world.add_component(TransformComponent::new());

        assert_eq!(
            constrained_parent_local_delta(
                &world,
                owner,
                DraggablePlane::Camera,
                [1.0, 2.0, 3.0],
                None,
            ),
            [1.0, 2.0, 3.0]
        );
        let camera_rig = world.add_component(TransformComponent::new());
        let camera = world.add_component(Camera3DComponent::default());
        world.add_child(camera_rig, camera).unwrap();
        assert_eq!(
            constrained_parent_local_delta(
                &world,
                owner,
                DraggablePlane::Camera,
                [1.0, 2.0, 3.0],
                Some(camera),
            ),
            [1.0, 2.0, 0.0]
        );
        let projected = constrained_parent_local_delta(
            &world,
            owner,
            DraggablePlane::WorldAxes([[1.0, 0.0, 0.0], [0.0, 0.0, 2.0]]),
            [1.0, 2.0, 3.0],
            None,
        );
        assert!((projected[0] - 1.0).abs() < 1e-6);
        assert!(projected[1].abs() < 1e-6);
        assert!((projected[2] - 3.0).abs() < 1e-6);
    }
}

//! Direct component construction for the RuntimeSpec launch-scene slice.
//!
//! This consumes the crate-owned MMS DTO directly. Components outside the
//! bounded slice return `None`, allowing the caller to use the frozen legacy
//! compatibility path without partially mutating the world.

use meow_meow_script as mms;

use crate::engine::ecs::component::{
    AmbientLightComponent, BackgroundColorComponent, BloomComponent, BlurPassComponent,
    Camera3DComponent, ColorComponent, DirectionalLightComponent, EmissiveComponent,
    EmissivePassComponent, PointerComponent, RaycastableComponent, RenderGraphComponent,
    RenderableComponent, RendererSettingsComponent, TransformComponent,
};
use crate::engine::ecs::{ComponentId, SignalEmitter, World};

use super::host::MittensHost;
use super::runtime_config::{ComponentInitializerKind, MittensBinding};

const DIRECT_COMPONENTS: &[&str] = &[
    "AmbientLight",
    "BackgroundColor",
    "Bloom",
    "BlurPass",
    "Camera3D",
    "Color",
    "DirectionalLight",
    "Emissive",
    "EmissivePass",
    "Pointer",
    "Raycastable",
    "RenderGraph",
    "Renderable",
    "RendererSettings",
    "Transform",
];

pub(crate) fn try_spawn_tree(
    tree: &mms::MaterializedCE,
    bindings: &mms::ImplementationBindings<MittensBinding>,
    world: &mut World,
    emit: &mut dyn SignalEmitter,
    initialize: bool,
) -> Option<Result<ComponentId, String>> {
    match tree_is_direct(tree, bindings) {
        Ok(true) => {}
        Ok(false) => return None,
        Err(error) => return Some(Err(error)),
    }
    Some(spawn_tree_uninitialized(tree, bindings, world).map(|root| {
        if initialize {
            world.init_component_tree(root, emit);
        }
        root
    }))
}

fn tree_is_direct(
    tree: &mms::MaterializedCE,
    bindings: &mms::ImplementationBindings<MittensBinding>,
) -> Result<bool, String> {
    if tree.deferred_block.is_some() || !tree.positionals.is_empty() {
        return Ok(false);
    }
    let Some(operation_id) = tree.constructor.operation_id else {
        return Ok(false);
    };
    let component = match bindings.get(operation_id) {
        Some(MittensBinding::ComponentConstructor { component, name })
            if *name == tree.constructor.name.as_deref() => *component,
        Some(binding) => {
            return Err(format!(
                "{operation_id:?} resolved to {binding:?}, not the selected constructor for {}",
                tree.component_type,
            ));
        }
        None => return Err(format!("unknown component constructor ID {operation_id:?}")),
    };
    if component != tree.component_type {
        return Err(format!(
            "component constructor {operation_id:?} creates {component}, not {}",
            tree.component_type
        ));
    }
    if !DIRECT_COMPONENTS.contains(&component) {
        return Ok(false);
    }
    for call in &tree.initializer_calls {
        let Some(id) = call.operation_id else { return Ok(false) };
        match bindings.get(id) {
            Some(MittensBinding::ComponentInitializer { component: bound_component, name, kind: ComponentInitializerKind::Call })
                if *bound_component == component && *name == call.name => {}
            Some(binding) => return Err(format!("{id:?} resolved to {binding:?}, not {component} call initializer '{}'", call.name)),
            None => return Err(format!("unknown call initializer ID {id:?}")),
        }
    }
    for property in &tree.properties {
        let Some(id) = property.operation_id else { return Ok(false) };
        match bindings.get(id) {
            Some(MittensBinding::ComponentInitializer { component: bound_component, name, kind: ComponentInitializerKind::Property })
                if *bound_component == component && *name == property.name => {}
            Some(binding) => return Err(format!("{id:?} resolved to {binding:?}, not {component} property initializer '{}'", property.name)),
            None => return Err(format!("unknown property initializer ID {id:?}")),
        }
    }
    for child in &tree.children {
        if let mms::CeChild::Spawn(child) = child {
            if !tree_is_direct(child, bindings)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn spawn_tree_uninitialized(
    tree: &mms::MaterializedCE,
    bindings: &mms::ImplementationBindings<MittensBinding>,
    world: &mut World,
) -> Result<ComponentId, String> {
    let operation_id = tree.constructor.operation_id.ok_or_else(|| {
        format!(
            "{} has no configured constructor operation",
            tree.component_type
        )
    })?;
    let component = match bindings.get(operation_id) {
        Some(MittensBinding::ComponentConstructor { component, name })
            if *name == tree.constructor.name.as_deref() => *component,
        _ => {
            return Err(format!(
                "{operation_id:?} is not the selected Mittens component constructor"
            ));
        }
    };
    let id = create_component(
        world,
        component,
        tree.constructor.name.as_deref(),
        &tree.constructor.arguments,
    )?;

    for call in &tree.initializer_calls {
        apply_call(world, id, component, &call.name, &call.arguments)?;
    }
    apply_node_properties(world, id, &tree.properties)?;

    if !tree.positionals.is_empty() {
        return Err(format!(
            "direct {component} construction does not accept positional values"
        ));
    }
    for child in &tree.children {
        let child = match child {
            mms::CeChild::Spawn(child) => spawn_tree_uninitialized(child, bindings, world)?,
            mms::CeChild::Attach(handle) => MittensHost::component_id(*handle),
        };
        world
            .add_child(id, child)
            .map_err(|error| format!("attach child to {component} failed: {error}"))?;
    }
    Ok(id)
}

fn create_component(
    world: &mut World,
    component: &str,
    constructor: Option<&str>,
    args: &[mms::Value],
) -> Result<ComponentId, String> {
    let id = match component {
        "Transform" => world.add_component(TransformComponent::new()),
        "Renderable" if constructor == Some("cube") => {
            world.add_component(RenderableComponent::cube())
        }
        "Renderable" => {
            return Err("direct Renderable slice currently supports only .cube()".into());
        }
        "Color" if constructor == Some("rgba") => world.add_component(ColorComponent::rgba(
            f32_arg(args, 0)?,
            f32_arg(args, 1)?,
            f32_arg(args, 2)?,
            f32_arg(args, 3)?,
        )),
        "Color" => world.add_component(ColorComponent::new()),
        "Emissive" => world.add_component(match constructor {
            Some("off") => EmissiveComponent::off(),
            _ => EmissiveComponent::on(),
        }),
        "AmbientLight" if constructor == Some("rgb") => world.add_component(
            AmbientLightComponent::rgb(f32_arg(args, 0)?, f32_arg(args, 1)?, f32_arg(args, 2)?),
        ),
        "AmbientLight" => world.add_component(AmbientLightComponent::new()),
        "BackgroundColor" => world.add_component(BackgroundColorComponent::new()),
        "RenderGraph" => world.add_component(match constructor {
            Some("off") => RenderGraphComponent::off(),
            _ => RenderGraphComponent::on(),
        }),
        "EmissivePass" => world.add_component(EmissivePassComponent::new()),
        "Bloom" => world.add_component(BloomComponent::new()),
        "BlurPass" => world.add_component(BlurPassComponent::new()),
        "Camera3D" => world.add_component(Camera3DComponent::new()),
        "Pointer" => world.add_component(match constructor {
            Some("disabled") => PointerComponent::disabled(),
            _ => PointerComponent::new(),
        }),
        "Raycastable" => world.add_component(match constructor {
            Some("disabled") => RaycastableComponent::disabled(),
            Some("drag_only") => RaycastableComponent::drag_only(),
            Some("click_only") => RaycastableComponent::click_only(),
            _ => RaycastableComponent::enabled(),
        }),
        "DirectionalLight" => world.add_component(DirectionalLightComponent::new()),
        "RendererSettings" => world.add_component(match constructor {
            Some("msaa_off") => RendererSettingsComponent::msaa_off(),
            _ => RendererSettingsComponent::new(),
        }),
        _ => {
            return Err(format!(
                "component {component} is outside the direct launch-scene slice"
            ));
        }
    };

    if let Some(constructor) = constructor {
        let factory_only = matches!(
            (component, constructor),
            ("Renderable", "cube")
                | ("Color", "rgba")
                | ("Emissive", "on" | "off")
                | ("AmbientLight", "rgb")
                | ("RenderGraph", "on" | "off")
                | ("Pointer", "disabled")
                | ("Raycastable", "disabled" | "drag_only" | "click_only" | "enabled")
                | ("RendererSettings", "msaa_off")
        );
        if !factory_only {
            apply_call(world, id, component, constructor, args)?;
        }
    }
    Ok(id)
}

fn apply_call(
    world: &mut World,
    id: ComponentId,
    component: &str,
    method: &str,
    args: &[mms::Value],
) -> Result<(), String> {
    match component {
        "Transform" => {
            let current = world
                .get_component_by_id_as::<TransformComponent>(id)
                .ok_or("missing Transform after construction")?
                .clone();
            let updated = match method {
                "position" => {
                    current.with_position(f32_arg(args, 0)?, f32_arg(args, 1)?, f32_arg(args, 2)?)
                }
                "scale" => {
                    current.with_scale(f32_arg(args, 0)?, f32_arg(args, 1)?, f32_arg(args, 2)?)
                }
                "rotation" | "rotation_euler" => current.with_rotation_euler(
                    f32_arg(args, 0)?,
                    f32_arg(args, 1)?,
                    f32_arg(args, 2)?,
                ),
                _ => return Err(format!("unsupported direct Transform call '{method}'")),
            };
            *world
                .get_component_by_id_as_mut::<TransformComponent>(id)
                .ok_or("missing Transform after construction")? = updated;
        }
        "Emissive" if method == "intensity" => {
            world
                .get_component_by_id_as_mut::<EmissiveComponent>(id)
                .ok_or("missing Emissive after construction")?
                .intensity = f32_arg(args, 0)?.max(0.0);
        }
        "Bloom" => {
            let bloom = world
                .get_component_by_id_as_mut::<BloomComponent>(id)
                .ok_or("missing Bloom after construction")?;
            match method {
                "on" => bloom.enabled = true,
                "off" => bloom.enabled = false,
                "enabled" => bloom.enabled = bool_arg(args, 0)?,
                "intensity" => bloom.intensity = f32_arg(args, 0)?.max(0.0),
                "radius_ndc" => bloom.radius_ndc = f32_arg(args, 0)?.max(0.0),
                "emissive_scale" => bloom.emissive_scale = f32_arg(args, 0)?.max(0.0),
                "half_res" => bloom.half_res = bool_arg(args, 0)?,
                "output_texture" => bloom.output_texture = Some(string_arg(args, 0)?.into()),
                _ => return Err(format!("unsupported direct Bloom call '{method}'")),
            }
        }
        "BlurPass" => {
            let blur = world
                .get_component_by_id_as_mut::<BlurPassComponent>(id)
                .ok_or("missing BlurPass after construction")?;
            match method {
                "on" => blur.enabled = true,
                "off" => blur.enabled = false,
                "enabled" => blur.enabled = bool_arg(args, 0)?,
                "radius_ndc" => blur.radius_ndc = f32_arg(args, 0)?.max(0.0),
                "half_res" => blur.half_res = bool_arg(args, 0)?,
                _ => return Err(format!("unsupported direct BlurPass call '{method}'")),
            }
        }
        "Camera3D" => {
            let camera = world
                .get_component_by_id_as_mut::<Camera3DComponent>(id)
                .ok_or("missing Camera3D after construction")?;
            match method {
                "enabled" => camera.enabled = bool_arg(args, 0)?,
                "fov" => camera.fov_y_degrees = f32_arg(args, 0)?,
                "near" => camera.z_near = f32_arg(args, 0)?,
                "far" => camera.z_far = f32_arg(args, 0)?,
                _ => return Err(format!("unsupported direct Camera3D call '{method}'")),
            }
        }
        "DirectionalLight" => {
            let light = world
                .get_component_by_id_as_mut::<DirectionalLightComponent>(id)
                .ok_or("missing DirectionalLight after construction")?;
            match method {
                "intensity" => light.intensity = f32_arg(args, 0)?,
                "color" => light.color = [f32_arg(args, 0)?, f32_arg(args, 1)?, f32_arg(args, 2)?],
                _ => {
                    return Err(format!(
                        "unsupported direct DirectionalLight call '{method}'"
                    ));
                }
            }
        }
        "RendererSettings" if method == "window_size" => {
            let settings = world
                .get_component_by_id_as::<RendererSettingsComponent>(id)
                .ok_or("missing RendererSettings after construction")?;
            let updated = settings.with_window_size(u32_arg(args, 0)?, u32_arg(args, 1)?);
            *world
                .get_component_by_id_as_mut::<RendererSettingsComponent>(id)
                .ok_or("missing RendererSettings after construction")? = updated;
        }
        _ => return Err(format!("unsupported direct call {component}.{method}")),
    }
    Ok(())
}

fn apply_node_properties(
    world: &mut World,
    id: ComponentId,
    properties: &[mms::MaterializedProperty],
) -> Result<(), String> {
    for property in properties {
        match property.name.as_str() {
            "name" | "id" => {
                world
                    .get_component_record_mut(id)
                    .ok_or("component disappeared during construction")?
                    .name = value_as_string(&property.value)?.into();
            }
            "class" => {
                let record = world
                    .get_component_record_mut(id)
                    .ok_or("component disappeared during construction")?;
                record.classes = match &property.value {
                    mms::Value::String(value) => {
                        value.split_whitespace().map(str::to_owned).collect()
                    }
                    mms::Value::Array(values) => values
                        .iter()
                        .map(value_as_string)
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                    value => return Err(format!("class expects a string or string array, got {value:?}")),
                };
            }
            other => return Err(format!("unsupported direct node property '{other}'")),
        }
    }
    Ok(())
}

fn f32_arg(args: &[mms::Value], index: usize) -> Result<f32, String> {
    match args.get(index) {
        Some(mms::Value::Number(value)) => Ok(*value as f32),
        value => Err(format!(
            "expected f32 argument {}, got {value:?}",
            index + 1
        )),
    }
}

fn u32_arg(args: &[mms::Value], index: usize) -> Result<u32, String> {
    match args.get(index) {
        Some(mms::Value::Number(value)) => Ok(*value as u32),
        value => Err(format!(
            "expected u32 argument {}, got {value:?}",
            index + 1
        )),
    }
}

fn bool_arg(args: &[mms::Value], index: usize) -> Result<bool, String> {
    match args.get(index) {
        Some(mms::Value::Bool(value)) => Ok(*value),
        value => Err(format!(
            "expected bool argument {}, got {value:?}",
            index + 1
        )),
    }
}

fn string_arg(args: &[mms::Value], index: usize) -> Result<&str, String> {
    args.get(index)
        .ok_or_else(|| format!("missing string argument {}", index + 1))
        .and_then(value_as_string)
}

fn value_as_string(value: &mms::Value) -> Result<&str, String> {
    match value {
        mms::Value::String(value) => Ok(value),
        value => Err(format!("expected string, got {value:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatched_operation_id_is_rejected_before_world_mutation() {
        let configured = crate::scripting::runtime_config::build_mittens_runtime().unwrap();
        let mut tree = configured
            .runtime()
            .materialize_component(
                "Transform.position(1.0, 2.0, 3.0).scale(2.0, 2.0, 2.0) {}",
            )
            .unwrap();
        tree.constructor.operation_id = tree.initializer_calls[0].operation_id;

        let mut world = World::default();
        let mut emit = crate::engine::ecs::CommandQueue::new();
        let error = try_spawn_tree(
            &tree,
            configured.bindings(),
            &mut world,
            &mut emit,
            false,
        )
        .expect("a configured-ID mismatch must not use the legacy fallback")
        .unwrap_err();

        assert!(error.contains("not the selected constructor"), "{error}");
        assert_eq!(world.all_components().count(), 0);
    }

    #[test]
    fn universal_properties_use_bound_ids_on_the_direct_path() {
        let configured = crate::scripting::runtime_config::build_mittens_runtime().unwrap();
        let tree = configured
            .runtime()
            .materialize_component(
                "Transform { name = \"root\" class = [\"scene\", \"visible\"] }",
            )
            .unwrap();
        assert!(tree.properties.iter().all(|property| property.operation_id.is_some()));

        let mut world = World::default();
        let mut emit = crate::engine::ecs::CommandQueue::new();
        let id = try_spawn_tree(
            &tree,
            configured.bindings(),
            &mut world,
            &mut emit,
            false,
        )
        .expect("bound universal properties should stay on the direct path")
        .unwrap();
        let record = world.get_component_record(id).unwrap();
        assert_eq!(record.name, "root");
        assert_eq!(record.classes, ["scene", "visible"]);
    }
}

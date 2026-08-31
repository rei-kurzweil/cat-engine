use crate::engine::ecs::{ComponentId, World};

use super::Component;
use super::ce_helpers::{CeBuilder, ce, num};

pub const DEFAULT_TRANSMISSION_IOR: f32 = 1.5;
pub const DEFAULT_TRANSMISSION_THICKNESS: f32 = 0.1;
pub const DEFAULT_TRANSMISSION_STRENGTH: f32 = 1.0;
pub const DEFAULT_TRANSMISSION_EDGE_FADE: f32 = 0.02;
pub const DEFAULT_ROUGH_TRANSMISSION_ROUGHNESS: f32 = 0.35;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransmissionOptions {
    pub ior: f32,
    pub thickness: f32,
    pub strength: f32,
    pub edge_fade: f32,
}

impl Default for TransmissionOptions {
    fn default() -> Self {
        Self {
            ior: DEFAULT_TRANSMISSION_IOR,
            thickness: DEFAULT_TRANSMISSION_THICKNESS,
            strength: DEFAULT_TRANSMISSION_STRENGTH,
            edge_fade: DEFAULT_TRANSMISSION_EDGE_FADE,
        }
    }
}

fn validate_min(component: &str, parameter: &str, value: f32, min: f32) -> Result<f32, String> {
    if value.is_finite() && value >= min {
        Ok(value)
    } else {
        Err(format!(
            "{component}.{parameter} expected a finite value >= {min}; got {value}"
        ))
    }
}

fn validate_range(
    component: &str,
    parameter: &str,
    value: f32,
    min: f32,
    max: f32,
) -> Result<f32, String> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(format!(
            "{component}.{parameter} expected a finite value in {min}..={max}; got {value}"
        ))
    }
}

impl TransmissionOptions {
    pub fn set_ior(&mut self, component: &str, value: f32) -> Result<(), String> {
        self.ior = validate_min(component, "ior", value, 1.0)?;
        Ok(())
    }

    pub fn set_thickness(&mut self, component: &str, value: f32) -> Result<(), String> {
        self.thickness = validate_min(component, "thickness", value, 0.0)?;
        Ok(())
    }

    pub fn set_strength(&mut self, component: &str, value: f32) -> Result<(), String> {
        self.strength = validate_min(component, "strength", value, 0.0)?;
        Ok(())
    }

    pub fn set_edge_fade(&mut self, component: &str, value: f32) -> Result<(), String> {
        self.edge_fade = validate_range(component, "edge_fade", value, 0.0, 0.5)?;
        Ok(())
    }
}

fn apply_common_builder(
    options: &mut TransmissionOptions,
    component: &str,
    method: &str,
    value: f32,
) -> Result<bool, String> {
    match method {
        "ior" => options.set_ior(component, value)?,
        "thickness" => options.set_thickness(component, value)?,
        "strength" => options.set_strength(component, value)?,
        "edge_fade" => options.set_edge_fade(component, value)?,
        _ => return Ok(false),
    }
    Ok(true)
}

fn options_to_mms_ast(
    component: &str,
    options: TransmissionOptions,
) -> crate::scripting::ast::ComponentExpression {
    let mut expression = ce(component);
    if options.ior != DEFAULT_TRANSMISSION_IOR {
        expression = expression.with_call("ior", vec![num(options.ior as f64)]);
    }
    if options.thickness != DEFAULT_TRANSMISSION_THICKNESS {
        expression = expression.with_call("thickness", vec![num(options.thickness as f64)]);
    }
    if options.strength != DEFAULT_TRANSMISSION_STRENGTH {
        expression = expression.with_call("strength", vec![num(options.strength as f64)]);
    }
    if options.edge_fade != DEFAULT_TRANSMISSION_EDGE_FADE {
        expression = expression.with_call("edge_fade", vec![num(options.edge_fade as f64)]);
    }
    expression
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RefractionComponent {
    pub options: TransmissionOptions,
}

impl RefractionComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_builder(&mut self, method: &str, value: f32) -> Result<(), String> {
        if apply_common_builder(&mut self.options, "Refraction", method, value)? {
            Ok(())
        } else {
            Err(format!("Refraction: unknown builder '{method}'"))
        }
    }
}

impl Component for RefractionComponent {
    fn name(&self) -> &'static str {
        "refraction"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn to_mms_ast(&self, _world: &World) -> crate::scripting::ast::ComponentExpression {
        options_to_mms_ast("Refraction", self.options)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoughTransmissionComponent {
    pub options: TransmissionOptions,
    pub roughness: f32,
}

impl Default for RoughTransmissionComponent {
    fn default() -> Self {
        Self {
            options: TransmissionOptions::default(),
            roughness: DEFAULT_ROUGH_TRANSMISSION_ROUGHNESS,
        }
    }
}

impl RoughTransmissionComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_builder(&mut self, method: &str, value: f32) -> Result<(), String> {
        if apply_common_builder(&mut self.options, "RoughTransmission", method, value)? {
            return Ok(());
        }
        match method {
            "roughness" => {
                self.roughness = validate_range("RoughTransmission", "roughness", value, 0.0, 1.0)?;
                Ok(())
            }
            _ => Err(format!("RoughTransmission: unknown builder '{method}'")),
        }
    }
}

impl Component for RoughTransmissionComponent {
    fn name(&self) -> &'static str {
        "rough_transmission"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn to_mms_ast(&self, _world: &World) -> crate::scripting::ast::ComponentExpression {
        let mut expression = options_to_mms_ast("RoughTransmission", self.options);
        if self.roughness != DEFAULT_ROUGH_TRANSMISSION_ROUGHNESS {
            expression = expression.with_call("roughness", vec![num(self.roughness as f64)]);
        }
        expression
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransmissiveModel {
    Refraction(TransmissionOptions),
    RoughTransmission {
        options: TransmissionOptions,
        roughness: f32,
    },
}

pub fn resolve_transmissive_model(
    world: &World,
    renderable: ComponentId,
) -> Result<Option<TransmissiveModel>, String> {
    if world
        .get_component_by_id_as::<super::RenderableComponent>(renderable)
        .is_none()
    {
        return Err(format!(
            "transmission resolution target {renderable:?} is not a RenderableComponent"
        ));
    }

    let mut resolved = None;
    for &child in world.children_of(renderable) {
        let candidate =
            if let Some(component) = world.get_component_by_id_as::<RefractionComponent>(child) {
                Some(TransmissiveModel::Refraction(component.options))
            } else {
                world
                    .get_component_by_id_as::<RoughTransmissionComponent>(child)
                    .map(|component| TransmissiveModel::RoughTransmission {
                        options: component.options,
                        roughness: component.roughness,
                    })
            };

        if let Some(candidate) = candidate {
            if resolved.is_some() {
                return Err(format!(
                    "renderable {renderable:?} has multiple immediate transmission components; attach exactly one Refraction or RoughTransmission"
                ));
            }
            resolved = Some(candidate);
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_ranges_are_rejected_instead_of_clamped() {
        let mut refraction = RefractionComponent::new();
        assert!(refraction.apply_builder("ior", 0.99).is_err());
        assert!(refraction.apply_builder("thickness", f32::NAN).is_err());
        assert!(refraction.apply_builder("edge_fade", 0.51).is_err());

        let mut rough = RoughTransmissionComponent::new();
        assert!(rough.apply_builder("roughness", -0.01).is_err());
        assert!(rough.apply_builder("roughness", 1.01).is_err());
    }

    #[test]
    fn authored_range_boundaries_are_accepted() {
        let mut refraction = RefractionComponent::new();
        refraction.apply_builder("ior", 1.0).unwrap();
        refraction.apply_builder("thickness", 0.0).unwrap();
        refraction.apply_builder("strength", 0.0).unwrap();
        refraction.apply_builder("edge_fade", 0.5).unwrap();

        let mut rough = RoughTransmissionComponent::new();
        rough.apply_builder("roughness", 0.0).unwrap();
        rough.apply_builder("roughness", 1.0).unwrap();
    }

    #[test]
    fn resolver_requires_one_immediate_material_component() {
        let mut world = World::default();
        let renderable = world.add_component(super::super::RenderableComponent::cube());
        assert_eq!(resolve_transmissive_model(&world, renderable), Ok(None));

        let refraction = world.add_component(RefractionComponent::new());
        world.add_child(renderable, refraction).unwrap();
        assert_eq!(
            resolve_transmissive_model(&world, renderable),
            Ok(Some(TransmissiveModel::Refraction(
                TransmissionOptions::default()
            )))
        );

        let rough = world.add_component(RoughTransmissionComponent::new());
        world.add_child(renderable, rough).unwrap();
        assert!(
            resolve_transmissive_model(&world, renderable)
                .unwrap_err()
                .contains("multiple immediate transmission components")
        );
    }

    #[test]
    fn resolver_does_not_inherit_transmission_from_ancestors() {
        let mut world = World::default();
        let parent = world.add_component(crate::engine::ecs::component::TransformComponent::new());
        let inherited = world.add_component(RefractionComponent::new());
        let renderable = world.add_component(super::super::RenderableComponent::cube());
        world.add_child(parent, inherited).unwrap();
        world.add_child(parent, renderable).unwrap();

        assert_eq!(resolve_transmissive_model(&world, renderable), Ok(None));
    }

    #[test]
    fn material_model_resolution_is_independent_of_geometry_variant_handles() {
        use crate::engine::graphics::primitives::{CpuMeshHandle, MaterialHandle, Renderable};

        let mut world = World::default();
        let static_renderable = world.add_component(super::super::RenderableComponent::new(
            Renderable::new(CpuMeshHandle::CUBE, MaterialHandle::TOON_MESH),
        ));
        let deformed_renderable = world.add_component(super::super::RenderableComponent::new(
            Renderable::new(CpuMeshHandle::CUBE, MaterialHandle::SKINNED_TOON_MESH),
        ));
        let static_refraction = world.add_component(RefractionComponent::new());
        let deformed_refraction = world.add_component(RefractionComponent::new());
        world
            .add_child(static_renderable, static_refraction)
            .unwrap();
        world
            .add_child(deformed_renderable, deformed_refraction)
            .unwrap();

        assert_eq!(
            resolve_transmissive_model(&world, static_renderable),
            resolve_transmissive_model(&world, deformed_renderable)
        );
    }
}

use crate::engine::ecs::component::Component;
use crate::engine::ecs::component::ce_helpers::ce;

/// Selects the non-emissive, lighting-independent solid-color material for a
/// standard renderable. The renderer uses the normal color/opacity/texture
/// inputs but skips the toon light accumulation and emissive/Bloom paths.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnlitComponent;

impl Component for UnlitComponent {
    fn name(&self) -> &'static str {
        "unlit"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn to_mms_ast(
        &self,
        _world: &crate::engine::ecs::World,
    ) -> crate::scripting::ast::ComponentExpression {
        ce("Unlit")
    }
}

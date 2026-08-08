use super::{Component, ComponentRef, QueryRootMode, resolve_component_ref};
use crate::engine::ecs::{ComponentId, World};

/// Durable association between a manipulated transform and the grid that owns
/// its translation-snapping frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridBindingComponent {
    pub grid: ComponentRef,
}

impl GridBindingComponent {
    pub fn new(grid: ComponentRef) -> Self {
        Self { grid }
    }

    pub fn resolve_grid_transform(
        &self,
        world: &World,
        binding_component: Option<ComponentId>,
    ) -> Option<ComponentId> {
        resolve_component_ref(
            world,
            &self.grid,
            binding_component,
            QueryRootMode::WorldRoot,
        )
    }
}

impl Component for GridBindingComponent {
    fn name(&self) -> &'static str {
        "grid_binding"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn to_mms_ast(&self, _world: &World) -> crate::scripting::ast::ComponentExpression {
        use crate::engine::ecs::component::ce_helpers::*;
        let reference = match &self.grid {
            ComponentRef::Guid(guid) => s(&format!("@uuid:{guid}")),
            ComponentRef::Query(query) => s(query),
        };
        ce_call("GridBinding", "grid", vec![reference])
    }
}

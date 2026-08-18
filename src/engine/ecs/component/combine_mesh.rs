use crate::engine::ecs::component::Component;

/// Consolidates static descendant renderables into one system-owned visual.
///
/// Phase 1 always uses the first descendant renderable's material.
#[derive(Debug, Default, Clone, Copy)]
pub struct CombineMeshComponent {
    /// Retain source topology and rebuild the combined mesh after source edits.
    /// Default mode removes source subtrees once the replacement is live.
    pub keep_transforms: bool,
}

impl CombineMeshComponent {
    pub fn keep_transforms() -> Self {
        Self {
            keep_transforms: true,
        }
    }
}

impl Component for CombineMeshComponent {
    fn name(&self) -> &'static str {
        "combine_mesh"
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
        use crate::engine::ecs::component::ce_helpers::{CeBuilder, ce};
        let node = ce("CombineMesh");
        if self.keep_transforms {
            node.with_call("keep_transforms", vec![])
        } else {
            node
        }
    }
}

use crate::engine::ecs::component::{Component, ComponentRef};

/// Authored immutable rest-space attachment from an imported anchor to a target node.
///
/// The component deliberately owns no pose-source or orientation policy. Consumers resolve both
/// references within their owning GLTF and may combine the retained offset with an independently
/// authored joint basis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestAttachmentComponent {
    pub anchor: ComponentRef,
    pub target: ComponentRef,
}

impl RestAttachmentComponent {
    pub fn new(anchor: ComponentRef, target: ComponentRef) -> Self {
        Self { anchor, target }
    }
}

impl Component for RestAttachmentComponent {
    fn name(&self) -> &'static str {
        "rest_attachment"
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
        use crate::engine::ecs::component::ce_helpers::*;
        let surface = |reference: &ComponentRef| match reference {
            ComponentRef::Guid(guid) => s(&format!("@uuid:{guid}")),
            ComponentRef::Query(query) => s(query),
        };
        ce_call(
            "RestAttachment",
            "new",
            vec![surface(&self.anchor), surface(&self.target)],
        )
    }
}

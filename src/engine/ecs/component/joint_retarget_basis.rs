use crate::engine::ecs::component::{Component, ComponentRef};
use crate::engine::ecs::{ComponentId, IntentValue, SignalEmitter};

/// Authored declaration of a canonical two-axis rest basis for one imported joint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointRetargetBasisComponent {
    pub target: ComponentRef,
    pub forward_start: ComponentRef,
    pub forward_end: ComponentRef,
    pub up_start: ComponentRef,
    pub up_end: ComponentRef,
    component_id: Option<ComponentId>,
}

impl JointRetargetBasisComponent {
    pub fn new(
        target: ComponentRef,
        forward_start: ComponentRef,
        forward_end: ComponentRef,
        up_start: ComponentRef,
        up_end: ComponentRef,
    ) -> Self {
        Self {
            target,
            forward_start,
            forward_end,
            up_start,
            up_end,
            component_id: None,
        }
    }
}

impl Component for JointRetargetBasisComponent {
    fn name(&self) -> &'static str {
        "joint_retarget_basis"
    }

    fn set_id(&mut self, id: ComponentId) {
        self.component_id = Some(id);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn init(&mut self, emit: &mut dyn SignalEmitter, component: ComponentId) {
        self.component_id = Some(component);
        emit.push_intent_now(
            component,
            IntentValue::RegisterJointRetargetBasis {
                component_id: component,
            },
        );
    }

    fn cleanup(&mut self, emit: &mut dyn SignalEmitter, component: ComponentId) {
        emit.push_intent_now(
            component,
            IntentValue::UnregisterJointRetargetBasis {
                component_id: component,
            },
        );
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
            "JointRetargetBasis",
            "new",
            vec![
                surface(&self.target),
                surface(&self.forward_start),
                surface(&self.forward_end),
                surface(&self.up_start),
                surface(&self.up_end),
            ],
        )
    }
}

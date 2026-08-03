use crate::engine::ecs::component::{Component, ComponentRef};
use crate::engine::ecs::{ComponentId, IntentValue, SignalEmitter};

/// Plane used to constrain pointer-driven translation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DraggablePlane {
    Object,
    Camera,
    WorldAxes([[f32; 3]; 2]),
}

impl Default for DraggablePlane {
    fn default() -> Self {
        Self::Object
    }
}

/// Selects the transform moved by a draggable marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraggableTarget {
    /// Move the Transform that owns the marker.
    Owner,
    /// Move the nearest Transform above the marker owner.
    ParentTransform,
    /// Move exactly the uniquely resolved authored reference.
    Explicit(ComponentRef),
}

/// Marks a Transform subtree as movable by pointer drag gestures.
#[derive(Debug, Clone, PartialEq)]
pub struct DraggableComponent {
    pub enabled: bool,
    pub target: DraggableTarget,
    /// Runtime-only cache for an explicit target. The authored reference remains in `target`.
    pub target_id: Option<ComponentId>,
    /// Runtime-only sticky-binding bit. Once a GUID target dies it must never retarget.
    pub target_was_bound: bool,
    pub plane: DraggablePlane,
}

impl DraggableComponent {
    pub fn new() -> Self {
        Self::on()
    }
    pub fn on() -> Self {
        Self {
            enabled: true,
            target: DraggableTarget::Owner,
            target_id: None,
            target_was_bound: false,
            plane: DraggablePlane::Object,
        }
    }
    pub fn off() -> Self {
        Self {
            enabled: false,
            target: DraggableTarget::Owner,
            target_id: None,
            target_was_bound: false,
            plane: DraggablePlane::Object,
        }
    }
    pub fn parent() -> Self {
        Self {
            enabled: true,
            target: DraggableTarget::ParentTransform,
            target_id: None,
            target_was_bound: false,
            plane: DraggablePlane::Object,
        }
    }
    pub fn explicit(target: ComponentRef) -> Self {
        Self {
            enabled: true,
            target: DraggableTarget::Explicit(target),
            target_id: None,
            target_was_bound: false,
            plane: DraggablePlane::Object,
        }
    }
    pub fn with_plane(mut self, plane: DraggablePlane) -> Self {
        self.plane = plane;
        self
    }
}

impl Default for DraggableComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for DraggableComponent {
    fn name(&self) -> &'static str {
        "draggable"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn init(&mut self, emit: &mut dyn SignalEmitter, component: ComponentId) {
        emit.push_intent_now(
            component,
            IntentValue::RegisterDraggable {
                component_id: component,
            },
        );
    }
    fn to_mms_ast(
        &self,
        _world: &crate::engine::ecs::World,
    ) -> crate::scripting::ast::ComponentExpression {
        use crate::engine::ecs::component::ce_helpers::*;
        let expression = if !self.enabled {
            ce_call("Draggable", "off", vec![])
        } else {
            match &self.target {
                DraggableTarget::Owner => ce_call("Draggable", "on", vec![]),
                DraggableTarget::ParentTransform => ce_call("Draggable", "parent", vec![]),
                DraggableTarget::Explicit(reference) => {
                    let value = match reference {
                        ComponentRef::Guid(guid) => s(&format!("@uuid:{guid}")),
                        ComponentRef::Query(query) => s(query),
                    };
                    ce_call("Draggable", "target", vec![value])
                }
            }
        };
        match self.plane {
            DraggablePlane::Object => expression,
            DraggablePlane::Camera => expression.with_call("plane", vec![s("camera")]),
            DraggablePlane::WorldAxes(axes) => expression.with_call(
                "plane",
                vec![array(
                    axes.into_iter()
                        .map(|axis| array(nums(axis.into_iter().map(|v| v as f64))))
                        .collect(),
                )],
            ),
        }
    }
}

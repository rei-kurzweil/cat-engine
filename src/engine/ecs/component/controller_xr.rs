use crate::engine::ecs::ComponentId;
use crate::engine::ecs::component::{Component, ComponentRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerHand {
    Left,
    Right,
}

impl Default for ControllerHand {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerPoseKind {
    /// A pointing pose, typically used for ray-based UI interaction.
    Aim,
    /// A “held object” pose, typically used for attaching models/tools.
    Grip,
    /// Grip position combined with aim orientation from the same XR frame.
    GripAim,
}

impl Default for ControllerPoseKind {
    fn default() -> Self {
        Self::Aim
    }
}

/// Frame-local tracking source selected by the XR runtime for this hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControllerPoseSource {
    #[default]
    None,
    WristPalm,
    ControllerAim,
    ControllerGrip,
    ControllerGripAim,
}

/// Marker/config for an XR controller tracked pose.
///
/// Semantics:
/// - Attach a `TransformComponent` as a child of this component.
/// - the active XR runtime will drive that transform child from controller pose tracking.
#[derive(Debug, Clone, Default)]
pub struct ControllerXRComponent {
    pub enabled: bool,
    /// Runtime-only: true after a valid controller pose was applied this frame.
    pub pose_valid: bool,
    /// Runtime-only source selected for the current frame.
    pub active_pose_source: ControllerPoseSource,
    /// Runtime-only raw controller orientations for alignment diagnostics.
    pub raw_aim_rotation: Option<[f32; 4]>,
    pub raw_grip_rotation: Option<[f32; 4]>,
    pub hand: ControllerHand,
    pub pose: ControllerPoseKind,
    pub laser: bool,
    /// Optional avatar middle-finger chain used to place and orient the ray.
    pub avatar_finger: Option<[ComponentRef; 3]>,
    /// Optional thumb-root landmark used to resolve hand roll around the finger axis.
    pub avatar_hand_up: Option<ComponentRef>,
    /// Optional index/little proximal landmarks used to derive anatomical palm roll.
    pub avatar_palm_width: Option<[ComponentRef; 2]>,
    pub(crate) avatar_laser_warned: bool,

    // Cached ECS id (runtime-only). Filled during init.
    pub component_id: Option<ComponentId>,
}

impl ControllerXRComponent {
    pub fn new(enabled: bool, hand: ControllerHand, pose: ControllerPoseKind) -> Self {
        Self {
            enabled,
            pose_valid: false,
            active_pose_source: ControllerPoseSource::None,
            raw_aim_rotation: None,
            raw_grip_rotation: None,
            hand,
            pose,
            laser: false,
            avatar_finger: None,
            avatar_hand_up: None,
            avatar_palm_width: None,
            avatar_laser_warned: false,
            component_id: None,
        }
    }

    pub fn laser(mut self) -> Self {
        self.laser = true;
        self
    }

    pub fn laser_from_avatar_finger(
        mut self,
        root: ComponentRef,
        middle: ComponentRef,
        tip: ComponentRef,
    ) -> Self {
        self.laser = true;
        self.avatar_finger = Some([root, middle, tip]);
        self
    }

    pub fn laser_from_avatar_hand(
        mut self,
        root: ComponentRef,
        middle: ComponentRef,
        tip: ComponentRef,
        thumb_root: ComponentRef,
    ) -> Self {
        self.laser = true;
        self.avatar_finger = Some([root, middle, tip]);
        self.avatar_hand_up = Some(thumb_root);
        self
    }

    pub fn palm_from_avatar_knuckles(
        mut self,
        index_root: ComponentRef,
        little_root: ComponentRef,
    ) -> Self {
        self.avatar_palm_width = Some([index_root, little_root]);
        self
    }

    pub fn on_left_aim() -> Self {
        Self::new(true, ControllerHand::Left, ControllerPoseKind::Aim)
    }

    pub fn on_right_aim() -> Self {
        Self::new(true, ControllerHand::Right, ControllerPoseKind::Aim)
    }
}

impl Component for ControllerXRComponent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn name(&self) -> &'static str {
        "xr_hand"
    }

    fn set_id(&mut self, component: ComponentId) {
        self.component_id = Some(component);
    }

    fn init(&mut self, emit: &mut dyn crate::engine::ecs::SignalEmitter, component: ComponentId) {
        self.component_id = Some(component);
        emit.push_intent_now(
            component,
            crate::engine::ecs::IntentValue::RegisterControllerXr {
                component_id: component,
            },
        );
    }

    fn cleanup(
        &mut self,
        emit: &mut dyn crate::engine::ecs::SignalEmitter,
        component: ComponentId,
    ) {
        emit.push_intent_now(
            component,
            crate::engine::ecs::IntentValue::RemoveControllerXr {
                component_id: component,
            },
        );
    }

    fn to_mms_ast(
        &self,
        _world: &crate::engine::ecs::World,
    ) -> crate::scripting::ast::ComponentExpression {
        use crate::engine::ecs::component::ce_helpers::*;
        let hand = match self.hand {
            ControllerHand::Left => "Left",
            ControllerHand::Right => "Right",
        };
        let pose = match self.pose {
            ControllerPoseKind::Aim => "Aim",
            ControllerPoseKind::Grip => "Grip",
            ControllerPoseKind::GripAim => "GripAim",
        };
        let mut expression = ce_call("XRHand", "new", vec![b(self.enabled), s(hand), s(pose)]);
        if let Some([root, middle, tip]) = &self.avatar_finger {
            let reference = |value: &ComponentRef| match value {
                ComponentRef::Guid(guid) => s(&format!("@uuid:{guid}")),
                ComponentRef::Query(query) => s(query),
            };
            expression = if let Some(thumb_root) = &self.avatar_hand_up {
                expression.with_call(
                    "laser_from_avatar_hand",
                    vec![
                        reference(root),
                        reference(middle),
                        reference(tip),
                        reference(thumb_root),
                    ],
                )
            } else {
                expression.with_call(
                    "laser_from_avatar_finger",
                    vec![reference(root), reference(middle), reference(tip)],
                )
            };
        } else if self.laser {
            expression = expression.with_call("laser", vec![]);
        }
        if let Some([index_root, little_root]) = &self.avatar_palm_width {
            let reference = |value: &ComponentRef| match value {
                ComponentRef::Guid(guid) => s(&format!("@uuid:{guid}")),
                ComponentRef::Query(query) => s(query),
            };
            expression = expression.with_call(
                "palm_from_avatar_knuckles",
                vec![reference(index_root), reference(little_root)],
            );
        }
        expression
    }
}

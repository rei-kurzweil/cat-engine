use super::{Component, ComponentRef};
use crate::engine::ecs::ComponentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    Playing,
    Looping,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationStepDirection {
    Next,
    Previous,
}

#[derive(Debug, Clone)]
pub struct AnimationComponent {
    pub state: AnimationState,
    /// Explicit loop length in beats. `None` falls back to the derived
    /// default in `AnimationSystem` (`floor(max_keyframe_beat) + 1`).
    /// Authored via `Animation.length(beats)`.
    pub length_beats: Option<f64>,
    pub scope_source: Option<ComponentRef>,
    pub resolved_scope: Option<ComponentId>,

    component: Option<ComponentId>,
}

impl AnimationComponent {
    pub fn new() -> Self {
        Self {
            state: AnimationState::Looping,
            length_beats: None,
            scope_source: None,
            resolved_scope: None,
            component: None,
        }
    }

    pub fn with_state(mut self, state: AnimationState) -> Self {
        self.state = state;
        self
    }

    pub fn with_length_beats(mut self, beats: f64) -> Self {
        self.length_beats = Some(beats);
        self
    }

    pub fn with_scope_source(mut self, source: ComponentRef) -> Self {
        self.scope_source = Some(source);
        self.resolved_scope = None;
        self
    }

    /// Backward-compatible helper for older callers.
    pub fn with_playing(self, playing: bool) -> Self {
        self.with_state(if playing {
            AnimationState::Looping
        } else {
            AnimationState::Paused
        })
    }

    pub fn id(&self) -> Option<ComponentId> {
        self.component
    }
}

impl Default for AnimationComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for AnimationComponent {
    fn set_id(&mut self, component: ComponentId) {
        self.component = Some(component);
    }

    fn name(&self) -> &'static str {
        "animation"
    }

    fn init(&mut self, emit: &mut dyn crate::engine::ecs::SignalEmitter, component: ComponentId) {
        emit.push_intent_now(
            component,
            crate::engine::ecs::IntentValue::RegisterAnimation {
                component_id: component,
            },
        );
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
        use crate::scripting::ast::Expression;

        fn target_expr(t: &ComponentRef) -> Expression {
            match t {
                ComponentRef::Guid(u) => Expression::String(format!("@uuid:{u}")),
                ComponentRef::Query(s) => Expression::String(s.clone()),
            }
        }

        let ctor = match self.state {
            AnimationState::Playing => "playing",
            AnimationState::Looping => "looping",
            AnimationState::Paused => "paused",
        };
        let mut ce = ce_call("Animation", ctor, vec![]);
        if let Some(n) = self.length_beats {
            ce = ce.with_call("length", vec![num(n)]);
        }
        if let Some(source) = &self.scope_source {
            ce = ce.with_call("scope", vec![target_expr(source)]);
        }
        ce
    }
}

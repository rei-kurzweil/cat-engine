use super::Component;
use crate::engine::ecs::ComponentId;

/// Authored selection of a live capture device. Numbered devices are only
/// meaningful for the current host session and are deliberately not resolved
/// by the component itself.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum AudioInputDeviceSelector {
    Default,
    DeviceNumber(usize),
}

impl Default for AudioInputDeviceSelector {
    fn default() -> Self {
        Self::Default
    }
}

/// Capture intent and observable state. CPAL streams and queue storage belong
/// to the audio-input runtime, never to this serializable ECS component.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AudioInputComponent {
    pub device: AudioInputDeviceSelector,
    pub enabled: bool,
    #[serde(skip)]
    component: Option<ComponentId>,
}

impl Default for AudioInputComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioInputComponent {
    pub fn new() -> Self {
        Self {
            device: AudioInputDeviceSelector::Default,
            enabled: true,
            component: None,
        }
    }

    pub fn device_number(index: usize) -> Self {
        Self {
            device: AudioInputDeviceSelector::DeviceNumber(index),
            ..Self::new()
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn id(&self) -> Option<ComponentId> {
        self.component
    }
}

impl Component for AudioInputComponent {
    fn set_id(&mut self, component: ComponentId) {
        self.component = Some(component);
    }
    fn name(&self) -> &'static str {
        "audio_input"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn init(&mut self, emit: &mut dyn crate::engine::ecs::SignalEmitter, component: ComponentId) {
        emit.push_intent_now(
            component,
            crate::engine::ecs::IntentValue::AudioGraphDirtyImmediate {
                component_id: component,
            },
        );
    }
    fn to_mms_ast(
        &self,
        _: &crate::engine::ecs::World,
    ) -> crate::scripting::ast::ComponentExpression {
        use crate::engine::ecs::component::ce_helpers::*;
        let mut out = match self.device {
            AudioInputDeviceSelector::Default => ce("AudioInput"),
            AudioInputDeviceSelector::DeviceNumber(index) => {
                ce_call("AudioInput", "device_number", vec![num(index as f64)])
            }
        };
        if !self.enabled {
            out = out.with_call("enabled", vec![b(false)]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_numbered_devices_are_authored_distinctly() {
        assert_eq!(
            AudioInputComponent::new().device,
            AudioInputDeviceSelector::Default
        );
        assert_eq!(
            AudioInputComponent::device_number(2).device,
            AudioInputDeviceSelector::DeviceNumber(2)
        );
    }
}

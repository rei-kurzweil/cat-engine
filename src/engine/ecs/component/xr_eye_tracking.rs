use super::Component;
use crate::engine::ecs::ComponentId;

#[derive(Debug, Clone)]
pub struct XREyeTrackingComponent {
    pub host: String,
    pub port: u16,
}
impl XREyeTrackingComponent {
    pub fn on() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 9000,
        }
    }
    pub fn listen(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }
}
impl Default for XREyeTrackingComponent {
    fn default() -> Self {
        Self::on()
    }
}
impl Component for XREyeTrackingComponent {
    fn name(&self) -> &'static str {
        "xr_eye_tracking"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn set_id(&mut self, _: ComponentId) {}
}

#[derive(Debug, Clone)]
pub struct XREyeTrackingHtcComponent {
    pub host: String,
    pub port: u16,
}
impl XREyeTrackingHtcComponent {
    pub fn on() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 9002,
        }
    }
    pub fn listen(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }
}
impl Default for XREyeTrackingHtcComponent {
    fn default() -> Self {
        Self::on()
    }
}
impl Component for XREyeTrackingHtcComponent {
    fn name(&self) -> &'static str {
        "xr_eye_tracking_htc"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn set_id(&mut self, _: ComponentId) {}
}

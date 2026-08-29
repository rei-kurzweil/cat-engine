use super::Component;
use crate::engine::ecs::ComponentId;

/// Declares the coordinate convention used by an eye-tracking transport.
///
/// `CancelHeadRotation` treats reported directions as world-relative and
/// converts them into the target eye bone's parent-local basis in AVC.  The
/// transport system intentionally retains the raw direction: only AVC knows
/// the avatar hierarchy that supplies that basis.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HeadRotationCompensation {
    #[default]
    Off,
    CancelHeadRotation,
}

impl HeadRotationCompensation {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "cancel" => Some(Self::CancelHeadRotation),
            _ => None,
        }
    }
}

/// Latest usable gaze directions received from an eye-tracking source.
///
/// This is runtime state, intentionally not exposed through MMS.  `sequence`
/// is assigned by `XREyeTrackingSystem` from one counter shared by both wire
/// protocols, so consumers can deterministically choose the newest source.
#[derive(Debug, Clone, Copy, Default)]
pub struct EyeGazeSample {
    pub left: Option<[f32; 3]>,
    pub right: Option<[f32; 3]>,
    pub sequence: u64,
}

/// Generic OSC reports one closure value for both eyes. It has its own
/// sequence because closure packets are independent from gaze packets.
#[derive(Debug, Clone, Copy, Default)]
pub struct EyeClosureSample {
    pub closure: Option<f32>,
    pub sequence: u64,
}

#[derive(Debug, Clone)]
pub struct XREyeTrackingComponent {
    pub host: String,
    pub port: u16,
    pub head_rotation_compensation: HeadRotationCompensation,
    pub(crate) gaze_sample: EyeGazeSample,
    pub(crate) closure_sample: EyeClosureSample,
}
impl XREyeTrackingComponent {
    pub fn on() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 9000,
            head_rotation_compensation: HeadRotationCompensation::Off,
            gaze_sample: EyeGazeSample::default(),
            closure_sample: EyeClosureSample::default(),
        }
    }
    pub fn listen(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            head_rotation_compensation: HeadRotationCompensation::Off,
            gaze_sample: EyeGazeSample::default(),
            closure_sample: EyeClosureSample::default(),
        }
    }
    pub fn with_head_rotation_compensation(mut self, value: HeadRotationCompensation) -> Self {
        self.head_rotation_compensation = value;
        self
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
    pub head_rotation_compensation: HeadRotationCompensation,
    pub(crate) gaze_sample: EyeGazeSample,
}
impl XREyeTrackingHtcComponent {
    pub fn on() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 9002,
            head_rotation_compensation: HeadRotationCompensation::Off,
            gaze_sample: EyeGazeSample::default(),
        }
    }
    pub fn listen(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            head_rotation_compensation: HeadRotationCompensation::Off,
            gaze_sample: EyeGazeSample::default(),
        }
    }
    pub fn with_head_rotation_compensation(mut self, value: HeadRotationCompensation) -> Self {
        self.head_rotation_compensation = value;
        self
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

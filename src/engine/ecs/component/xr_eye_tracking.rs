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

/// Latest normalized per-eye closure values (`0 = open`, `1 = closed`).
///
/// It has its own sequence because closure packets are independent from gaze
/// packets. Transports with one combined value duplicate it into both eyes;
/// transports with independent openness preserve each eye after conversion.
#[derive(Debug, Clone, Copy, Default)]
pub struct EyeClosureSample {
    pub left: Option<f32>,
    pub right: Option<f32>,
    pub sequence: u64,
}

/// Independent angular caps around the head-local forward axis (`-Z`).
/// Values are radians and are validated by the MMS builder before storage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EyeRotationLimits {
    pub left: f32,
    pub right: f32,
    pub up: f32,
    pub down: f32,
}

impl EyeRotationLimits {
    pub const fn from_array(values: [f32; 4]) -> Self {
        Self {
            left: values[0],
            right: values[1],
            up: values[2],
            down: values[3],
        }
    }

    /// Apply both configured policies, taking the stricter cap per direction.
    pub fn tighter(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            right: self.right.min(other.right),
            up: self.up.min(other.up),
            down: self.down.min(other.down),
        }
    }
}

pub fn combined_eye_rotation_limits(
    shared: Option<EyeRotationLimits>,
    per_eye: Option<EyeRotationLimits>,
) -> Option<EyeRotationLimits> {
    match (shared, per_eye) {
        (Some(shared), Some(per_eye)) => Some(shared.tighter(per_eye)),
        (Some(limits), None) | (None, Some(limits)) => Some(limits),
        (None, None) => None,
    }
}

#[derive(Debug, Clone)]
pub struct XREyeTrackingComponent {
    pub host: String,
    pub port: u16,
    pub head_rotation_compensation: HeadRotationCompensation,
    pub rotation_limits: Option<EyeRotationLimits>,
    pub rotation_limits_per_eye: [Option<EyeRotationLimits>; 2],
    pub(crate) gaze_sample: EyeGazeSample,
    pub(crate) closure_sample: EyeClosureSample,
}
impl XREyeTrackingComponent {
    pub fn on() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 9000,
            head_rotation_compensation: HeadRotationCompensation::Off,
            rotation_limits: None,
            rotation_limits_per_eye: [None; 2],
            gaze_sample: EyeGazeSample::default(),
            closure_sample: EyeClosureSample::default(),
        }
    }
    pub fn listen(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            head_rotation_compensation: HeadRotationCompensation::Off,
            rotation_limits: None,
            rotation_limits_per_eye: [None; 2],
            gaze_sample: EyeGazeSample::default(),
            closure_sample: EyeClosureSample::default(),
        }
    }
    pub fn with_head_rotation_compensation(mut self, value: HeadRotationCompensation) -> Self {
        self.head_rotation_compensation = value;
        self
    }
    pub fn with_rotation_limits(mut self, values: [f32; 4]) -> Self {
        self.rotation_limits = Some(EyeRotationLimits::from_array(values));
        self
    }
    pub fn with_rotation_limits_per_eye(mut self, left: [f32; 4], right: [f32; 4]) -> Self {
        self.rotation_limits_per_eye = [
            Some(EyeRotationLimits::from_array(left)),
            Some(EyeRotationLimits::from_array(right)),
        ];
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
    pub rotation_limits: Option<EyeRotationLimits>,
    pub rotation_limits_per_eye: [Option<EyeRotationLimits>; 2],
    pub(crate) gaze_sample: EyeGazeSample,
    pub(crate) closure_sample: EyeClosureSample,
}
impl XREyeTrackingHtcComponent {
    pub fn on() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 9002,
            head_rotation_compensation: HeadRotationCompensation::Off,
            rotation_limits: None,
            rotation_limits_per_eye: [None; 2],
            gaze_sample: EyeGazeSample::default(),
            closure_sample: EyeClosureSample::default(),
        }
    }
    pub fn listen(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            head_rotation_compensation: HeadRotationCompensation::Off,
            rotation_limits: None,
            rotation_limits_per_eye: [None; 2],
            gaze_sample: EyeGazeSample::default(),
            closure_sample: EyeClosureSample::default(),
        }
    }
    pub fn with_head_rotation_compensation(mut self, value: HeadRotationCompensation) -> Self {
        self.head_rotation_compensation = value;
        self
    }
    pub fn with_rotation_limits(mut self, values: [f32; 4]) -> Self {
        self.rotation_limits = Some(EyeRotationLimits::from_array(values));
        self
    }
    pub fn with_rotation_limits_per_eye(mut self, left: [f32; 4], right: [f32; 4]) -> Self {
        self.rotation_limits_per_eye = [
            Some(EyeRotationLimits::from_array(left)),
            Some(EyeRotationLimits::from_array(right)),
        ];
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

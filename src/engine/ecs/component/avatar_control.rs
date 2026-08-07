use crate::engine::ecs::ComponentId;
use crate::engine::ecs::component::Component;

/// Coordinates all pose drivers for a humanoid avatar.
///
/// **Design rule**: every transform driver that moves this avatar's bones must be a
/// child of (or otherwise routed through) this component.  This includes the primary
/// body/head driver (`Input` / `InputXR`) and any hand controllers (`ControllerXR`).
/// Uncoordinated drivers that bypass this component and write directly to armature bones
/// are the root cause of the torso-rotation bug in the old two-input design.
///
/// Multiple drivers are fine; what matters is that they all appear in this node's
/// subtree so `AvatarControlSystem` can discover and route them during init.
///
/// ## Controller discovery
///
/// Hand controllers are discovered automatically by topology: any `ControllerXRComponent`
/// that is a **direct child** of this component is registered as a hand driver.
/// Its `hand` field (`Left` / `Right`) determines which hand bone it drives.
/// The bone is displaced under the controller's first `TransformComponent` child
/// (the driven transform written by `OpenXRSystem`).
///
/// If no controller is present for a configured hand bone, a plain
/// `TransformComponent` splice is inserted instead (for IK-only or static setups).
///
/// ## Topology (after init)
///
/// ```text
/// Input  (or  InputXR)                    ← primary driver
///   └── driven_t
///         ├── AvatarControlComponent
///               ├── model_root  (TransformComponent, Y offset)
///               │     └── GLTFComponent
///               │           └── [armature]
///               │                 left_lower_arm
///               │                   └── ControllerXR (Left, Grip)  ← moved here by system
///               │                         └── controller_driven_t
///               │                               └── J_Bip_L_Hand (displaced)
///               │                 right_lower_arm
///               │                   └── ControllerXR (Right, Grip)
///               │                         └── controller_driven_t
///               │                               └── J_Bip_R_Hand (displaced)
///               ├── ControllerXR (Left,  Grip) { T }  ← declared here; re-parented on init
///               └── ControllerXR (Right, Grip) { T }
///         └── head_mount  ← injected by AVC; fixed offset from driven_t
///               └── J_Bip_C_Head (displaced from the armature)
/// ```
#[derive(Debug, Clone)]
pub struct AvatarControlComponent {
    /// Whether AVC should generate an upright collision capsule. Enabled by default.
    pub collision_enabled: bool,

    /// Authored character-controller radius, capped to half the measured height.
    pub capsule_radius: f32,

    /// Body-local pole hint for the left elbow in the 2-bone arm IK solve.
    /// Transformed to world-space each tick by the solver using the model root
    /// rotation, so the elbow stays anatomically correct when the body turns.
    /// Default `[-1, 0, -1]` (elbow out + slightly back).
    pub left_arm_pole_direction: [f32; 3],

    /// Body-local pole hint for the right elbow in the 2-bone arm IK solve.
    /// Transformed to world-space each tick by the solver using the model root
    /// rotation, so the elbow stays anatomically correct when the body turns.
    /// Default `[1, 0, -1]`.
    pub right_arm_pole_direction: [f32; 3],

    /// Yaw delta (radians) that triggers body rotation. Default: π/4 (45°).
    pub body_yaw_threshold: f32,

    /// Body rotation rate (radians/sec). Default: 3.0.
    pub body_yaw_rate: f32,

    /// Use +Z as the authored forward axis override.
    ///
    /// When not explicitly overridden, AVC keeps the shared XR-style default
    /// (`false`) for both desktop and XR. This override remains available for
    /// assets that were authored with a different convention.
    pub forward_plus_z: bool,

    /// Whether `forward_plus_z` was explicitly authored as an override.
    pub forward_plus_z_overridden: bool,

    /// Initial body yaw (radians) seeded into the `YawFollow` pipeline op.
    ///
    /// When not explicitly overridden, AVC uses the shared default `π`.
    pub initial_body_yaw: f32,

    /// Whether `initial_body_yaw` was explicitly authored as an override.
    pub initial_body_yaw_overridden: bool,

    /// Optional rotation smoothing for hand pose drivers (ControllerXR etc.).
    /// Applied to the rotation channel of each discovered hand driver's pipeline.
    /// Equivalent to `QuatTemporalFilter` smoothing_factor. `None` = no smoothing pipeline.
    pub hand_rotation_smoothing: Option<f32>,

    /// Explicit avatar height (metres) used to set model_root.y = -avatar_height.
    /// Overrides the camera_bone auto-calibration if both are set.
    /// Use this when the camera bone lookup fails or the mesh height is known in advance.
    pub avatar_height: Option<f32>,

    /// Vertical distance (metres) from the head bone pivot to the eyes.
    ///
    /// VRM `J_Bip_C_Head` pivot sits at the skull base; the eye line is typically
    /// ~0.08 m above that.  When this is set, AVC shifts `model_root.y` down by
    /// this amount so the EYES (not the bone pivot) land at `driven_t`'s world Y
    /// — i.e. at HMD height in VR, or at the desktop input height.
    ///
    /// Without this, the avatar's eyes sit above the HMD eye position and the
    /// face/hair mesh swings into the XR camera frustum when pitching down.
    ///
    /// Applies on top of either `camera_bone` auto-calibration or
    /// `avatar_height` override.  Default: `None` (no adjustment).
    pub eye_height_from_head_bone: Option<f32>,

    /// Vertical offset (metres) used exclusively for the head IK target calculation.
    ///
    /// This is decoupled from the camera position transform (`T { CXR }` wrapper)
    /// so the camera can be positioned freely without affecting how the FABRIK solver
    /// bends the spine.  Typically set to a small value like 0.04–0.08 to account for
    /// the gap between the head bone pivot and the eye position, causing the spine to
    /// bend so the head lands at the right height relative to the HMD.
    ///
    /// When set, the FABRIK target_position_offset uses this value (Y-only) instead of
    /// reading the camera transform's translation.  If `None`, no offset is applied to
    /// the IK target (the head bone pivot chases the HMD position directly).
    /// Default: `None`.
    pub head_ik_eye_height: Option<f32>,

    // Runtime IDs set by AvatarControlSystem on first tick:
    pub(crate) head_mount: Option<ComponentId>,
    pub(crate) displaced_head: Option<ComponentId>,
    /// Cached left hand bone id (end effector of left-arm TwoBoneIK).
    pub(crate) left_hand_bone_id: Option<ComponentId>,
    /// Cached right hand bone id (end effector of right-arm TwoBoneIK).
    pub(crate) right_hand_bone_id: Option<ComponentId>,
    /// Raw left controller/grip transform that feeds the optional hand offset node.
    pub(crate) left_hand_raw_target_id: Option<ComponentId>,
    /// Raw right controller/grip transform that feeds the optional hand offset node.
    pub(crate) right_hand_raw_target_id: Option<ComponentId>,
    /// Final left visual hand target transform used by IK.
    pub(crate) left_hand_visual_target_id: Option<ComponentId>,
    /// Final right visual hand target transform used by IK.
    pub(crate) right_hand_visual_target_id: Option<ComponentId>,
    /// Immutable rest-pose correction that maps controller aim onto the finger mount.
    pub(crate) left_hand_aim_correction: Option<[f32; 4]>,
    pub(crate) right_hand_aim_correction: Option<[f32; 4]>,

    /// ComponentId of the body pipeline root (`TransformForkTRSComponent`).
    /// Set by `try_init_splices`.
    pub(crate) body_pipeline_id: Option<ComponentId>,

    /// The mapped/generated transform that owns the camera path after initialization.
    pub(crate) splice_camera_bone: Option<ComponentId>,
    pub(crate) humanoid_map_gltf: Option<ComponentId>,
    pub(crate) humanoid_map_generation: u64,

    /// Debug/diagnostic flag: skip creation of the body-rotation pipeline entirely.
    /// When `true`, model_root stays directly under AVC and only head rotation is applied.
    /// Use this to isolate whether torso-twist bugs originate in the body pipeline.
    pub skip_body_pipeline: bool,

    /// Debug/diagnostic flag: when enabled, arm TwoBoneIK chains spawn overlay
    /// visualizations for the actual target vector, transformed pole vector,
    /// bend-plane normal, and solved elbow direction used by the solver.
    pub ik_debug: bool,

    // ---------------------------------------------------------------------
    // Head-pose-sensitive body XZ translate follow (see
    // `docs/task/avatar-control-simple-humanoid-body-follow.md`, Phase 1).
    // ---------------------------------------------------------------------
    /// Enables the optional neck rest-pin when the map resolves a neck slot.
    pub neck_pin_enabled: bool,

    // Runtime state set by AvatarControlSystem / HeadPoseBodyXzFollowSystem:
    /// `model_root` component id, stashed at init so the body-follow system
    /// doesn't have to re-walk topology each tick.
    pub(crate) model_root_id: Option<ComponentId>,

    /// `model_root.local.translation.y` at rest (body height offset).  Set
    /// Set once at init from resolved camera-anchor calibration or `avatar_height`.
    pub(crate) model_root_local_y: f32,

    /// Resolved neck bone id (under `model_root`).  `None` if not found.
    pub(crate) neck_bone_id: Option<ComponentId>,

    /// Neck rest local translation cached at init for the rest-pin.
    pub(crate) neck_rest_translation: Option<[f32; 3]>,

    /// Runtime-only generated upright capsule transform.
    pub(crate) capsule_transform_id: Option<ComponentId>,

    /// Runtime-only generated response, used to refresh XR movement routing.
    pub(crate) capsule_response_id: Option<ComponentId>,

    component: Option<ComponentId>,
}

impl AvatarControlComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_collision_disabled(mut self) -> Self {
        self.collision_enabled = false;
        self
    }

    pub fn with_capsule_radius(mut self, radius: f32) -> Self {
        self.capsule_radius = radius.max(0.0);
        self
    }

    /// Override the left elbow pole direction (body-local).
    pub fn with_left_arm_pole_direction(mut self, dir: [f32; 3]) -> Self {
        self.left_arm_pole_direction = dir;
        self
    }

    /// Override the right elbow pole direction (body-local).
    pub fn with_right_arm_pole_direction(mut self, dir: [f32; 3]) -> Self {
        self.right_arm_pole_direction = dir;
        self
    }

    pub fn with_body_yaw_threshold(mut self, t: f32) -> Self {
        self.body_yaw_threshold = t;
        self
    }

    pub fn with_body_yaw_rate(mut self, r: f32) -> Self {
        self.body_yaw_rate = r;
        self
    }

    /// Override the initial body yaw (radians) seeded into the `YawFollow` pipeline op.
    /// Use `std::f32::consts::PI` for rigs that face -Z at rest.
    pub fn with_initial_yaw(mut self, yaw: f32) -> Self {
        self.initial_body_yaw = yaw;
        self.initial_body_yaw_overridden = true;
        self
    }

    /// Use +Z as the authored forward axis override.
    pub fn with_forward_plus_z(mut self) -> Self {
        self.forward_plus_z = true;
        self.forward_plus_z_overridden = true;
        self
    }

    /// Enable rotation smoothing for hand pose drivers.
    /// Set to e.g. `220.0` for smooth VR controller rotation.
    pub fn with_hand_rotation_smoothing(mut self, factor: f32) -> Self {
        self.hand_rotation_smoothing = Some(factor);
        self
    }

    /// Skip creation of the body-rotation pipeline. Only head rotation will be applied.
    /// Use to isolate whether torso-twist bugs originate in the body pipeline.
    pub fn with_body_pipeline_disabled(mut self) -> Self {
        self.skip_body_pipeline = true;
        self
    }

    /// Enable TwoBoneIK debug visualizations for chains owned by this AVC.
    pub fn with_ik_debug(mut self) -> Self {
        self.ik_debug = true;
        self
    }

    /// Explicitly set `model_root.y = -height` during init, bypassing mapped
    /// camera-anchor calibration. Use when the mesh height is known in advance.
    /// Camera re-parenting still uses the resolved humanoid camera anchor.
    pub fn with_avatar_height(mut self, height: f32) -> Self {
        self.avatar_height = Some(height);
        self
    }

    /// Shift `model_root.y` down so the avatar's EYES (not the head bone pivot)
    /// land at `driven_t`'s world Y.  Default eye offset for VRM is ~0.08.
    pub fn with_eye_height_from_head_bone(mut self, dy: f32) -> Self {
        self.eye_height_from_head_bone = Some(dy);
        self
    }

    /// Disable the neck rest-pin.
    pub fn without_neck_pin(mut self) -> Self {
        self.neck_pin_enabled = false;
        self
    }

    pub fn with_neck_pin_enabled(mut self, enabled: bool) -> Self {
        self.neck_pin_enabled = enabled;
        self
    }

    /// Set the vertical offset for the head IK target calculation (metres).
    /// Decoupled from the camera position so spine bending and camera positioning
    /// can be controlled independently. Default: `None`.
    pub fn with_head_ik_eye_height(mut self, dy: f32) -> Self {
        self.head_ik_eye_height = Some(dy);
        self
    }
}

impl Default for AvatarControlComponent {
    fn default() -> Self {
        Self {
            collision_enabled: true,
            capsule_radius: 0.28,
            left_arm_pole_direction: [-1.0, 0.0, -1.0],
            right_arm_pole_direction: [1.0, 0.0, -1.0],
            body_yaw_threshold: std::f32::consts::FRAC_PI_4,
            body_yaw_rate: 3.0,
            forward_plus_z: false,
            forward_plus_z_overridden: false,
            initial_body_yaw: 0.0,
            initial_body_yaw_overridden: false,
            hand_rotation_smoothing: None,
            avatar_height: None,
            eye_height_from_head_bone: None,
            head_mount: None,
            displaced_head: None,
            left_hand_bone_id: None,
            right_hand_bone_id: None,
            left_hand_raw_target_id: None,
            right_hand_raw_target_id: None,
            left_hand_visual_target_id: None,
            right_hand_visual_target_id: None,
            left_hand_aim_correction: None,
            right_hand_aim_correction: None,
            body_pipeline_id: None,
            splice_camera_bone: None,
            humanoid_map_gltf: None,
            humanoid_map_generation: 0,
            skip_body_pipeline: false,
            ik_debug: false,
            head_ik_eye_height: None,
            neck_pin_enabled: true,
            model_root_id: None,
            model_root_local_y: 0.0,
            neck_bone_id: None,
            neck_rest_translation: None,
            capsule_transform_id: None,
            capsule_response_id: None,
            component: None,
        }
    }
}

impl Component for AvatarControlComponent {
    fn name(&self) -> &'static str {
        "avatar_control"
    }

    fn set_id(&mut self, id: ComponentId) {
        self.component = Some(id);
    }

    fn init(&mut self, emit: &mut dyn crate::engine::ecs::SignalEmitter, component: ComponentId) {
        emit.push_intent_now(
            component,
            crate::engine::ecs::IntentValue::RegisterAvatarControl {
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
        let mut c = ce("AvatarControl")
            .with_call(
                "body_yaw_threshold",
                vec![num(self.body_yaw_threshold as f64)],
            )
            .with_call("body_yaw_rate", vec![num(self.body_yaw_rate as f64)]);
        if !self.collision_enabled {
            c = c.with_call("collision_disabled", vec![]);
        }
        if (self.capsule_radius - 0.28).abs() > f32::EPSILON {
            c = c.with_call("capsule_radius", vec![num(self.capsule_radius as f64)]);
        }
        if self.left_arm_pole_direction != [-1.0, 0.0, -1.0] {
            let d = self.left_arm_pole_direction;
            c = c.with_call(
                "left_arm_pole_direction",
                vec![array(vec![
                    num(d[0] as f64),
                    num(d[1] as f64),
                    num(d[2] as f64),
                ])],
            );
        }
        if self.right_arm_pole_direction != [1.0, 0.0, -1.0] {
            let d = self.right_arm_pole_direction;
            c = c.with_call(
                "right_arm_pole_direction",
                vec![array(vec![
                    num(d[0] as f64),
                    num(d[1] as f64),
                    num(d[2] as f64),
                ])],
            );
        }
        if self.forward_plus_z_overridden && self.forward_plus_z {
            c = c.with_call("forward_plus_z", vec![]);
        }
        if self.initial_body_yaw_overridden {
            c = c.with_call("initial_yaw", vec![num(self.initial_body_yaw as f64)]);
        }
        if self.ik_debug {
            c = c.with_call("ik_debug", vec![]);
        }
        if let Some(factor) = self.hand_rotation_smoothing {
            c = c.with_call("hand_rotation_smoothing", vec![num(factor as f64)]);
        }
        if let Some(h) = self.avatar_height {
            c = c.with_call("avatar_height", vec![num(h as f64)]);
        }
        if let Some(dy) = self.eye_height_from_head_bone {
            c = c.with_call("eye_height_from_head_bone", vec![num(dy as f64)]);
        }
        if let Some(dy) = self.head_ik_eye_height {
            c = c.with_call("head_ik_eye_height", vec![num(dy as f64)]);
        }
        if !self.neck_pin_enabled {
            c = c.with_call("neck_pin_disabled", vec![]);
        }
        c
    }
}

use crate::engine::ecs::component::PointerEvents;
use crate::engine::ecs::system::draggable_system::draggable_owner_for_hit;
use crate::engine::ecs::system::grabbable_system::grabbable_owner_for_hit;
use crate::engine::ecs::system::pointer_system::{
    PointerActivations, PointerSystem, pointer_topology_context,
};
use crate::engine::ecs::system::{BvhSystem, TransformSystem};
use crate::engine::ecs::{
    ComponentId, EventSignal, PointerActivationSource, RxWorld, SignalKind, World,
};
use crate::engine::graphics::VisualWorld;
use crate::engine::user_input::InputState;
use crate::utils::math;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DragUpdatePolicy {
    /// Only emit drag moves while the pointer still intersects the original target.
    RequireTargetContact,

    /// After `DragStart`, continue producing deltas by projecting the current pointer ray onto a
    /// stable plane captured at drag start.
    ///
    /// Used for editor gizmos where losing intersection with thin handle geometry is common.
    StartPlaneProjection,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GesturePointerClass {
    Desktop,
    Spatial,
}

#[derive(Debug, Default, Clone)]
pub struct GestureState {
    pub dragging: bool,
    pub drag_raycaster: Option<ComponentId>,
    pub drag_renderable: Option<ComponentId>,
    /// First click-capable hit at DragStart. Click is dispatched here, not to `drag_renderable`,
    /// so a DragOnly plane in front of rows doesn't swallow clicks.
    pub click_renderable: Option<ComponentId>,
    /// Hit point on `click_renderable` at press time (which may differ from the drag hit point).
    pub click_hit_point: Option<[f32; 3]>,
    pub last_hit_point: Option<[f32; 3]>,

    // Start-plane projection drag mode state.
    pub last_cursor_pos: Option<(f32, f32)>,
    pub drag_plane_point_world: Option<[f32; 3]>,
    pub drag_plane_normal_world: Option<[f32; 3]>,

    // Click detection: position at DragStart.
    pub drag_start_screen_pos: Option<(f32, f32)>,
    pub drag_start_hit_point: Option<[f32; 3]>,
    pub press_pointer_class: Option<GesturePointerClass>,
    pub press_ray_origin: Option<[f32; 3]>,
    pub press_ray_direction: Option<[f32; 3]>,
    pub xr_draggable: bool,
    pub last_controller_world: Option<[f32; 3]>,
}

#[derive(Debug)]
pub struct GestureSystem {
    /// Per-pointer gesture state, keyed by PointerComponent id.
    states: HashMap<ComponentId, GestureState>,
    grip_states: HashMap<ComponentId, GripGestureState>,
    grabbed_owners: HashSet<ComponentId>,
    pub drag_update_policy: DragUpdatePolicy,

    /// All ray hits this frame, sorted by interaction priority first, then front-to-back by t.
    /// Each entry: (priority, t, raycaster, renderable, origin, dir, pointer_events).
    ray_hits_sorted: Arc<
        Mutex<
            Vec<(
                u8,
                f32,
                ComponentId,
                ComponentId,
                [f32; 3],
                [f32; 3],
                PointerEvents,
            )>,
        >,
    >,
    immediate_handlers_installed: bool,
}

#[derive(Debug, Clone, Copy)]
struct GripGestureState {
    renderable: ComponentId,
    owner: ComponentId,
    desktop: bool,
}

impl GestureSystem {
    fn debug_gesture_enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| {
            let v = std::env::var("CAT_DEBUG_GESTURE").unwrap_or_default();
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    }

    pub fn begin_frame(&mut self) {
        if let Ok(mut hits) = self.ray_hits_sorted.lock() {
            hits.clear();
        }
    }

    /// Install drain-point handlers into `RxWorld`.
    pub fn install_handlers(&mut self, rx: &mut RxWorld) {
        if self.immediate_handlers_installed {
            return;
        }

        let hits_ref = self.ray_hits_sorted.clone();
        rx.add_global_handler_closure(SignalKind::RayIntersected, move |world, _emit, env| {
            let Some(EventSignal::RayIntersected {
                raycaster,
                renderable,
                t,
                origin,
                dir,
            }) = env.event.as_ref()
            else {
                return;
            };

            if !t.is_finite() || *t < 0.0 {
                return;
            }

            let (priority, pe) = BvhSystem::find_raycastable_for_renderable(world, *renderable)
                .map(|rc| (rc.interaction_priority, rc.pointer_events))
                .unwrap_or((0, PointerEvents::All));

            let Ok(mut hits) = hits_ref.lock() else {
                return;
            };
            let entry = (priority, *t, *raycaster, *renderable, *origin, *dir, pe);
            let pos = hits.partition_point(|h| h.0 > priority || (h.0 == priority && h.1 < *t));
            hits.insert(pos, entry);
        });

        self.immediate_handlers_installed = true;
    }

    /// Returns the gesture state for the first active pointer, for callers that only care about
    /// a single pointer (e.g. editor gizmos, cursor 3D).
    pub fn state(&self) -> &GestureState {
        // Return the first dragging state if any, otherwise any state, otherwise a default.
        self.states
            .values()
            .find(|s| s.dragging)
            .or_else(|| self.states.values().next())
            .unwrap_or(&EMPTY_GESTURE_STATE)
    }

    pub fn set_drag_update_policy(&mut self, policy: DragUpdatePolicy) {
        self.drag_update_policy = policy;
    }

    fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
        let mut out = [[0.0f32; 4]; 4];
        for c in 0..4 {
            for r in 0..4 {
                out[c][r] =
                    a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
            }
        }
        out
    }

    fn mat4_mul_vec4(m: [[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
        [
            m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0] * v[3],
            m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1] * v[3],
            m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2] * v[3],
            m[0][3] * v[0] + m[1][3] * v[1] + m[2][3] * v[2] + m[3][3] * v[3],
        ]
    }

    fn vec3_dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn finite_normalized(v: [f32; 3]) -> Option<[f32; 3]> {
        if !v.iter().all(|x| x.is_finite()) {
            return None;
        }
        let length_squared = Self::vec3_dot(v, v);
        if !length_squared.is_finite() || length_squared <= 0.0 {
            return None;
        }
        let inverse_length = length_squared.sqrt().recip();
        Some([
            v[0] * inverse_length,
            v[1] * inverse_length,
            v[2] * inverse_length,
        ])
    }

    fn distance(a: [f32; 3], b: [f32; 3]) -> Option<f32> {
        if !a.iter().chain(b.iter()).all(|x| x.is_finite()) {
            return None;
        }
        let delta = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let squared = Self::vec3_dot(delta, delta);
        squared.is_finite().then(|| squared.sqrt())
    }

    fn normalized_cosine_similarity(a: [f32; 3], b: [f32; 3]) -> Option<f32> {
        Self::finite_normalized(a)
            .zip(Self::finite_normalized(b))
            .map(|(a, b)| Self::vec3_dot(a, b).clamp(-1.0, 1.0))
    }

    fn ray_from_cursor(visuals: &VisualWorld, input: &InputState) -> Option<([f32; 3], [f32; 3])> {
        let vp = visuals.viewport();
        let w = vp[0];
        let h = vp[1];
        if w <= 0.0 || h <= 0.0 {
            return None;
        }

        let (cx, cy) = input.cursor_pos.unwrap_or((w * 0.5, h * 0.5));

        let x_ndc = (2.0 * (cx / w)) - 1.0;
        let y_ndc = 1.0 - (2.0 * (cy / h));

        let view = visuals.camera_view();
        let proj = visuals.camera_proj();
        let vp_mat = Self::mat4_mul(proj, view);
        let inv_vp = math::mat4_inverse(vp_mat)?;

        let near_clip = [x_ndc, y_ndc, 0.0, 1.0];
        let far_clip = [x_ndc, y_ndc, 1.0, 1.0];

        let near_world4 = Self::mat4_mul_vec4(inv_vp, near_clip);
        let far_world4 = Self::mat4_mul_vec4(inv_vp, far_clip);

        let near_w = near_world4[3];
        let far_w = far_world4[3];
        if near_w == 0.0 || far_w == 0.0 {
            return None;
        }

        let near = [
            near_world4[0] / near_w,
            near_world4[1] / near_w,
            near_world4[2] / near_w,
        ];
        let far = [
            far_world4[0] / far_w,
            far_world4[1] / far_w,
            far_world4[2] / far_w,
        ];

        let dir = math::vec3_normalize([far[0] - near[0], far[1] - near[1], far[2] - near[2]]);
        Some((near, dir))
    }

    fn ray_plane_intersect(
        origin: [f32; 3],
        dir: [f32; 3],
        plane_point: [f32; 3],
        plane_normal: [f32; 3],
    ) -> Option<[f32; 3]> {
        let denom = Self::vec3_dot(plane_normal, dir);
        if denom.abs() < 1e-6 {
            return None;
        }
        let op = [
            plane_point[0] - origin[0],
            plane_point[1] - origin[1],
            plane_point[2] - origin[2],
        ];
        let t = Self::vec3_dot(plane_normal, op) / denom;
        if !t.is_finite() {
            return None;
        }
        Some([
            origin[0] + dir[0] * t,
            origin[1] + dir[1] * t,
            origin[2] + dir[2] * t,
        ])
    }

    /// Consume RayIntersected signals and PointerActivations to emit DragStart/DragMove/DragEnd/Click.
    ///
    /// `input` is still passed for `cursor_pos` (screen-space fields on desktop pointer events).
    /// `activations` drives press/down/release for each pointer regardless of input source.
    pub fn tick_with_rx(
        &mut self,
        world: &World,
        visuals: &VisualWorld,
        input: &InputState,
        activations: &PointerActivations,
        pointer_system: &PointerSystem,
        rx: &mut RxWorld,
    ) {
        let hits: Vec<(
            u8,
            f32,
            ComponentId,
            ComponentId,
            [f32; 3],
            [f32; 3],
            PointerEvents,
        )> = self
            .ray_hits_sorted
            .lock()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_default();

        // --- Grip (XR) and left mouse (desktop): capture Grabbable targets. ---
        let mut grab_presses: Vec<(ComponentId, bool)> = activations
            .grip_pressed
            .iter()
            .copied()
            .map(|p| (p, false))
            .collect();
        grab_presses.extend(
            activations
                .pressed
                .iter()
                .copied()
                .filter(|p| {
                    let topology = pointer_topology_context(world, *p);
                    !topology.has_controller_driver
                        && !topology.has_xr_camera_anchor
                        && !topology.has_xr_input_driver
                })
                .map(|p| (p, true)),
        );
        let mut desktop_grab_consumed = HashSet::new();
        for (pointer_cid, desktop) in grab_presses {
            if self.grip_states.contains_key(&pointer_cid) {
                continue;
            }
            let Some(raycaster) = pointer_system.raycast_for_pointer(pointer_cid) else {
                continue;
            };
            let Some(hit) = hits.iter().find(|h| {
                h.2 == raycaster
                    && h.6.captures_drag()
                    && grabbable_owner_for_hit(world, h.3)
                        .is_some_and(|owner| !self.grabbed_owners.contains(&owner))
            }) else {
                continue;
            };
            let Some(owner) = grabbable_owner_for_hit(world, hit.3) else {
                continue;
            };
            self.grabbed_owners.insert(owner);
            self.grip_states.insert(
                pointer_cid,
                GripGestureState {
                    renderable: hit.3,
                    owner,
                    desktop,
                },
            );
            if desktop {
                desktop_grab_consumed.insert(pointer_cid);
            }
            rx.push_event(
                hit.3,
                EventSignal::GrabStart {
                    pointer: pointer_cid,
                    raycaster,
                    renderable: hit.3,
                    target: owner,
                    ray_origin_world: hit.4,
                    ray_dir_world: hit.5,
                },
            );
        }

        let mut grab_releases = activations.grip_released.clone();
        grab_releases.extend(
            activations
                .released
                .iter()
                .copied()
                .filter(|p| self.grip_states.get(p).is_some_and(|s| s.desktop)),
        );
        for pointer_cid in grab_releases {
            if let Some(state) = self.grip_states.remove(&pointer_cid) {
                self.grabbed_owners.remove(&state.owner);
                rx.push_event(
                    state.renderable,
                    EventSignal::GrabEnd {
                        pointer: pointer_cid,
                        target: state.owner,
                    },
                );
            }
        }

        // --- Trigger press: start a new drag per activated pointer ---
        for &pointer_cid in &activations.pressed {
            if desktop_grab_consumed.contains(&pointer_cid) {
                continue;
            }
            // Only start a gesture if this pointer isn't already dragging.
            if self
                .states
                .get(&pointer_cid)
                .map(|s| s.dragging)
                .unwrap_or(false)
            {
                continue;
            }

            let Some(raycaster_cid) = pointer_system.raycast_for_pointer(pointer_cid) else {
                continue;
            };

            // Hits from this pointer's raycaster only.
            let pointer_hits: Vec<_> = hits.iter().filter(|h| h.2 == raycaster_cid).collect();

            let topology = pointer_topology_context(world, pointer_cid);
            let controller_pointer = topology.has_controller_driver;
            let drag_hit = pointer_hits.iter().find(|h| {
                if !h.6.captures_drag() {
                    return false;
                }
                // XR trigger never activates a Grabbable-only target. Draggable (including a
                // target carrying both markers) and ordinary unmarked targets retain trigger UI.
                !controller_pointer
                    || draggable_owner_for_hit(world, h.3).is_some()
                    || grabbable_owner_for_hit(world, h.3).is_none()
            });
            let click_hit = pointer_hits.iter().find(|h| h.6.captures_click());
            if Self::debug_gesture_enabled() {
                let summary: Vec<String> = pointer_hits
                    .iter()
                    .take(8)
                    .map(|h| format!("{:?} t={:.3} pri={} pe={:?}", h.3, h.1, h.0, h.6))
                    .collect();
                eprintln!(
                    "[gesture] press pointer={:?} raycaster={:?} drag_hit={:?} click_hit={:?} hits={}",
                    pointer_cid,
                    raycaster_cid,
                    drag_hit.map(|h| h.3),
                    click_hit.map(|h| h.3),
                    if summary.is_empty() {
                        "<none>".to_string()
                    } else {
                        summary.join(" | ")
                    }
                );
            }

            let Some(&&(_priority, t, raycaster, renderable, origin, dir, _pe)) = drag_hit else {
                continue;
            };

            let drag_hit_point = Some([
                origin[0] + dir[0] * t,
                origin[1] + dir[1] * t,
                origin[2] + dir[2] * t,
            ]);

            // Determine if this is a screen-space pointer (has cursor_pos).
            let is_screen_pointer = !controller_pointer
                && !topology.has_xr_camera_anchor
                && !topology.has_xr_input_driver;
            let screen_pos = is_screen_pointer.then_some(input.cursor_pos).flatten();

            let state = self.states.entry(pointer_cid).or_default();
            state.dragging = true;
            state.drag_raycaster = Some(raycaster);
            state.drag_renderable = Some(renderable);
            state.click_renderable = click_hit.map(|h| h.3);
            state.click_hit_point = click_hit.map(|h| {
                [
                    h.4[0] + h.5[0] * h.1,
                    h.4[1] + h.5[1] * h.1,
                    h.4[2] + h.5[2] * h.1,
                ]
            });
            state.last_hit_point = drag_hit_point;
            state.last_cursor_pos = if is_screen_pointer { screen_pos } else { None };
            state.drag_start_screen_pos = if is_screen_pointer { screen_pos } else { None };
            state.drag_start_hit_point = drag_hit_point;
            state.press_pointer_class = Some(if is_screen_pointer {
                GesturePointerClass::Desktop
            } else {
                GesturePointerClass::Spatial
            });
            state.press_ray_origin = Some(origin);
            state.press_ray_direction = Some(dir);
            state.xr_draggable =
                controller_pointer && draggable_owner_for_hit(world, renderable).is_some();
            state.last_controller_world = state
                .xr_draggable
                .then(|| TransformSystem::world_position(world, pointer_cid))
                .flatten();

            // StartPlaneProjection only makes sense for screen-space pointers; XR uses RequireTargetContact.
            if self.drag_update_policy == DragUpdatePolicy::StartPlaneProjection
                && is_screen_pointer
            {
                let n = math::vec3_normalize(dir);
                state.drag_plane_point_world = drag_hit_point;
                state.drag_plane_normal_world = Some(n);
                if let Some(p0) = drag_hit_point {
                    state.last_hit_point = Some(p0);
                }
            }

            if let Some(p) = drag_hit_point {
                rx.push_event(
                    renderable,
                    EventSignal::DragStart {
                        activation_source: PointerActivationSource::Trigger,
                        raycaster,
                        renderable,
                        hit_point: p,
                        ray_dir_world: dir,
                        screen_pos_px: if is_screen_pointer { screen_pos } else { None },
                    },
                );
            }
        }

        // --- Down: continue active drags ---
        let active_pointers: Vec<ComponentId> = self.states.keys().copied().collect();
        for pointer_cid in active_pointers {
            let is_down = activations.down.contains(&pointer_cid);
            let is_released = activations.released.contains(&pointer_cid);

            // Move drag.
            if is_down {
                let (Some(active_rc), Some(active_renderable)) = ({
                    let s = self.states.get(&pointer_cid).unwrap();
                    (s.drag_raycaster, s.drag_renderable)
                }) else {
                    self.states.remove(&pointer_cid);
                    continue;
                };

                if !self
                    .states
                    .get(&pointer_cid)
                    .map(|s| s.dragging)
                    .unwrap_or(false)
                {
                    continue;
                }

                let pointer_hits: Vec<_> = hits.iter().filter(|h| h.2 == active_rc).collect();
                let is_screen_pointer = self
                    .states
                    .get(&pointer_cid)
                    .and_then(|s| s.drag_start_screen_pos)
                    .is_some();

                // Controller Draggable capture follows controller translation after press and
                // deliberately does not require the ray to remain over the target.
                if self
                    .states
                    .get(&pointer_cid)
                    .is_some_and(|s| s.xr_draggable)
                {
                    if let Some(current_controller) =
                        TransformSystem::world_position(world, pointer_cid)
                    {
                        let state = self.states.get_mut(&pointer_cid).unwrap();
                        if let Some(previous_controller) = state.last_controller_world {
                            let delta = [
                                current_controller[0] - previous_controller[0],
                                current_controller[1] - previous_controller[1],
                                current_controller[2] - previous_controller[2],
                            ];
                            if delta != [0.0; 3] {
                                let previous_hit =
                                    state.last_hit_point.unwrap_or(current_controller);
                                let hit_point = [
                                    previous_hit[0] + delta[0],
                                    previous_hit[1] + delta[1],
                                    previous_hit[2] + delta[2],
                                ];
                                rx.push_event(
                                    active_renderable,
                                    EventSignal::DragMove {
                                        activation_source: PointerActivationSource::Trigger,
                                        raycaster: active_rc,
                                        renderable: active_renderable,
                                        hit_point,
                                        delta_world: delta,
                                        screen_pos_px: None,
                                        screen_delta_px: None,
                                    },
                                );
                                state.last_hit_point = Some(hit_point);
                            }
                        }
                        state.last_controller_world = Some(current_controller);
                    }
                    continue;
                }

                let effective_policy = if is_screen_pointer {
                    self.drag_update_policy
                } else {
                    DragUpdatePolicy::RequireTargetContact
                };

                match effective_policy {
                    DragUpdatePolicy::RequireTargetContact => {
                        let target_hit = pointer_hits
                            .iter()
                            .find(|h| h.2 == active_rc && h.3 == active_renderable);
                        if let Some(&(_priority, t, _rc, _r, origin, dir, _pe)) =
                            target_hit.copied()
                        {
                            let cur = [
                                origin[0] + dir[0] * t,
                                origin[1] + dir[1] * t,
                                origin[2] + dir[2] * t,
                            ];
                            let state = self.states.get_mut(&pointer_cid).unwrap();
                            if let Some(prev) = state.last_hit_point {
                                let delta = [cur[0] - prev[0], cur[1] - prev[1], cur[2] - prev[2]];
                                if delta[0] != 0.0 || delta[1] != 0.0 || delta[2] != 0.0 {
                                    let screen_pos_px = if is_screen_pointer {
                                        input.cursor_pos
                                    } else {
                                        None
                                    };
                                    let screen_delta_px = if is_screen_pointer {
                                        match (state.last_cursor_pos, screen_pos_px) {
                                            (Some((px, py)), Some((cx, cy))) => {
                                                Some((cx - px, cy - py))
                                            }
                                            _ => None,
                                        }
                                    } else {
                                        None
                                    };
                                    rx.push_event(
                                        active_renderable,
                                        EventSignal::DragMove {
                                            activation_source: PointerActivationSource::Trigger,
                                            raycaster: active_rc,
                                            renderable: active_renderable,
                                            hit_point: cur,
                                            delta_world: delta,
                                            screen_pos_px,
                                            screen_delta_px,
                                        },
                                    );
                                }
                            }
                            let state = self.states.get_mut(&pointer_cid).unwrap();
                            state.last_hit_point = Some(cur);
                            state.last_cursor_pos = input.cursor_pos;
                        }
                    }

                    DragUpdatePolicy::StartPlaneProjection => {
                        let Some((o, d)) = Self::ray_from_cursor(visuals, input) else {
                            if let Some(s) = self.states.get_mut(&pointer_cid) {
                                s.last_cursor_pos = input.cursor_pos;
                            }
                            continue;
                        };

                        let (pp, pn) = {
                            let s = self.states.get(&pointer_cid).unwrap();
                            (s.drag_plane_point_world, s.drag_plane_normal_world)
                        };
                        let (Some(pp), Some(pn)) = (pp, pn) else {
                            if let Some(s) = self.states.get_mut(&pointer_cid) {
                                s.last_cursor_pos = input.cursor_pos;
                            }
                            continue;
                        };

                        let Some(cur) = Self::ray_plane_intersect(o, d, pp, pn) else {
                            if let Some(s) = self.states.get_mut(&pointer_cid) {
                                s.last_cursor_pos = input.cursor_pos;
                            }
                            continue;
                        };

                        let state = self.states.get_mut(&pointer_cid).unwrap();
                        if let Some(prev) = state.last_hit_point {
                            let delta = [cur[0] - prev[0], cur[1] - prev[1], cur[2] - prev[2]];
                            if delta[0] != 0.0 || delta[1] != 0.0 || delta[2] != 0.0 {
                                let screen_pos_px = input.cursor_pos;
                                let screen_delta_px = match (state.last_cursor_pos, screen_pos_px) {
                                    (Some((px, py)), Some((cx, cy))) => Some((cx - px, cy - py)),
                                    _ => None,
                                };
                                rx.push_event(
                                    active_renderable,
                                    EventSignal::DragMove {
                                        activation_source: PointerActivationSource::Trigger,
                                        raycaster: active_rc,
                                        renderable: active_renderable,
                                        hit_point: cur,
                                        delta_world: delta,
                                        screen_pos_px,
                                        screen_delta_px,
                                    },
                                );
                            }
                        }
                        state.last_hit_point = Some(cur);
                        state.last_cursor_pos = input.cursor_pos;
                    }
                }
            }

            // End drag.
            if is_released {
                if let Some(state) = self.states.get(&pointer_cid) {
                    if state.dragging {
                        if let (Some(active_rc), Some(active_renderable)) =
                            (state.drag_raycaster, state.drag_renderable)
                        {
                            rx.push_event(
                                active_renderable,
                                EventSignal::DragEnd {
                                    activation_source: PointerActivationSource::Trigger,
                                    raycaster: active_rc,
                                    renderable: active_renderable,
                                    hit_point: state.last_hit_point,
                                },
                            );

                            let release_click_hit = hits
                                .iter()
                                .find(|h| h.2 == active_rc && h.6.captures_click());
                            let pointer = world
                                .get_component_by_id_as::<
                                    crate::engine::ecs::component::PointerComponent,
                                >(pointer_cid);

                            let rejection = match (
                                state.click_renderable,
                                state.click_hit_point,
                                release_click_hit,
                                pointer,
                            ) {
                                (None, _, _, _) => Some("missing press click target".to_string()),
                                (_, None, _, _) => {
                                    Some("missing press click hit point".to_string())
                                }
                                (_, _, None, _) => Some("missing release hit".to_string()),
                                (_, _, _, None) => {
                                    Some("missing pointer configuration".to_string())
                                }
                                (
                                    Some(press_target),
                                    Some(_),
                                    Some(release_hit),
                                    Some(_pointer),
                                ) if release_hit.3 != press_target => Some(format!(
                                    "target changed: press={press_target:?} release={:?}",
                                    release_hit.3
                                )),
                                (Some(_), Some(_), Some(release_hit), Some(pointer)) => {
                                    match state.press_pointer_class {
                                        Some(GesturePointerClass::Desktop) => {
                                            match (state.drag_start_screen_pos, input.cursor_pos) {
                                                (Some((sx, sy)), Some((ex, ey))) => {
                                                    let distance = ((ex - sx).powi(2)
                                                        + (ey - sy).powi(2))
                                                    .sqrt();
                                                    (!distance.is_finite()
                                                        || distance
                                                            > pointer
                                                                .click_max_screen_distance_px)
                                                        .then(|| {
                                                            format!(
                                                                "pixel movement: {distance:.3}px exceeds {:.3}px",
                                                                pointer.click_max_screen_distance_px
                                                            )
                                                        })
                                                }
                                                _ => Some(
                                                    "pixel movement: missing cursor position"
                                                        .to_string(),
                                                ),
                                            }
                                        }
                                        Some(GesturePointerClass::Spatial) => {
                                            let similarity =
                                                state.press_ray_direction.and_then(|start| {
                                                    Self::normalized_cosine_similarity(
                                                        start,
                                                        release_hit.5,
                                                    )
                                                });
                                            let minimum_similarity =
                                                pointer.click_max_ray_angle_deg.to_radians().cos();
                                            match similarity {
                                                None => Some(
                                                    "angular similarity: invalid ray direction"
                                                        .to_string(),
                                                ),
                                                Some(dot)
                                                    if !dot.is_finite()
                                                        || dot < minimum_similarity =>
                                                {
                                                    Some(format!(
                                                        "angular similarity: {dot:.6} < {minimum_similarity:.6}"
                                                    ))
                                                }
                                                Some(_) => match state.press_ray_origin.and_then(
                                                    |start| Self::distance(start, release_hit.4),
                                                ) {
                                                    None => Some(
                                                        "origin displacement: invalid ray origin"
                                                            .to_string(),
                                                    ),
                                                    Some(distance)
                                                        if distance
                                                            > pointer.click_max_origin_distance =>
                                                    {
                                                        Some(format!(
                                                            "origin displacement: {distance:.6}m > {:.6}m",
                                                            pointer.click_max_origin_distance
                                                        ))
                                                    }
                                                    Some(_) => None,
                                                },
                                            }
                                        }
                                        None => Some("missing press pointer class".to_string()),
                                    }
                                }
                            };

                            if rejection.is_none() {
                                let click_target = state.click_renderable.expect("checked above");
                                if Self::debug_gesture_enabled() {
                                    eprintln!(
                                        "[gesture] click pointer={:?} raycaster={:?} drag_renderable={:?} click_target={:?}",
                                        pointer_cid, active_rc, active_renderable, click_target,
                                    );
                                }
                                if let Some(start_hit) = state.click_hit_point {
                                    rx.push_event(
                                        click_target,
                                        EventSignal::Click {
                                            raycaster: active_rc,
                                            renderable: click_target,
                                            hit_point: start_hit,
                                            screen_pos_px: state.drag_start_screen_pos,
                                        },
                                    );
                                }
                            } else if Self::debug_gesture_enabled() {
                                eprintln!(
                                    "[gesture] click rejected pointer={pointer_cid:?} raycaster={active_rc:?}: {}",
                                    rejection.expect("rejection checked above")
                                );
                            }
                        }
                    }
                }
                self.states.remove(&pointer_cid);
            }
        }
    }
}

static EMPTY_GESTURE_STATE: GestureState = GestureState {
    dragging: false,
    drag_raycaster: None,
    drag_renderable: None,
    click_renderable: None,
    click_hit_point: None,
    last_hit_point: None,
    last_cursor_pos: None,
    drag_plane_point_world: None,
    drag_plane_normal_world: None,
    drag_start_screen_pos: None,
    drag_start_hit_point: None,
    press_pointer_class: None,
    press_ray_origin: None,
    press_ray_direction: None,
    xr_draggable: false,
    last_controller_world: None,
};

impl Default for GestureSystem {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            grip_states: HashMap::new(),
            grabbed_owners: HashSet::new(),
            drag_update_policy: DragUpdatePolicy::StartPlaneProjection,
            ray_hits_sorted: Arc::new(Mutex::new(Vec::new())),
            immediate_handlers_installed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GestureSystem;
    use crate::engine::ecs::component::{
        CameraXRComponent, PointerComponent, PointerEvents, TransformComponent,
    };
    use crate::engine::ecs::system::pointer_system::{PointerActivations, PointerSystem};
    use crate::engine::ecs::{EventSignal, RxWorld, World};
    use crate::engine::graphics::VisualWorld;
    use crate::engine::user_input::InputState;

    fn set_hit(
        gestures: &mut GestureSystem,
        raycaster: crate::engine::ecs::ComponentId,
        target: crate::engine::ecs::ComponentId,
        origin: [f32; 3],
        direction: [f32; 3],
        t: f32,
    ) {
        gestures.begin_frame();
        gestures.ray_hits_sorted.lock().unwrap().push((
            0,
            t,
            raycaster,
            target,
            origin,
            direction,
            PointerEvents::All,
        ));
    }

    fn event_count(rx: &mut RxWorld, predicate: impl Fn(&EventSignal) -> bool) -> usize {
        rx.drain_ready_events()
            .into_iter()
            .filter_map(|signal| signal.event)
            .filter(predicate)
            .count()
    }

    #[test]
    fn normalized_cosine_similarity_is_scale_independent() {
        let dot = GestureSystem::normalized_cosine_similarity([0.0, 0.0, -20.0], [0.0, 0.0, -0.25]);
        assert_eq!(dot, Some(1.0));

        let ninety_degrees =
            GestureSystem::normalized_cosine_similarity([10.0, 0.0, 0.0], [0.0, 0.001, 0.0]);
        assert_eq!(ninety_degrees, Some(0.0));
    }

    #[test]
    fn normalized_cosine_similarity_rejects_invalid_directions() {
        assert_eq!(
            GestureSystem::normalized_cosine_similarity([0.0; 3], [0.0, 0.0, -1.0]),
            None
        );
        assert_eq!(
            GestureSystem::normalized_cosine_similarity([f32::NAN, 0.0, -1.0], [0.0, 0.0, -1.0]),
            None
        );
    }

    #[test]
    fn desktop_click_uses_pointer_pixel_threshold_and_release_target() {
        let mut world = World::default();
        let pointer =
            world.add_component(PointerComponent::new().click_max_screen_distance_px(5.0));
        let target = world.add_component(TransformComponent::new());
        let other_target = world.add_component(TransformComponent::new());
        let mut pointers = PointerSystem::default();
        let mut rx = RxWorld::default();
        pointers.register_pointer(&mut world, pointer, &mut rx);
        let raycaster = pointers.raycast_for_pointer(pointer).unwrap();
        let visuals = VisualWorld::default();
        let mut gestures = GestureSystem::default();

        let mut input = InputState::default();
        input.cursor_pos = Some((10.0, 10.0));
        set_hit(
            &mut gestures,
            raycaster,
            target,
            [0.0; 3],
            [0.0, 0.0, -1.0],
            1.0,
        );
        gestures.tick_with_rx(
            &world,
            &visuals,
            &input,
            &PointerActivations {
                pressed: vec![pointer],
                ..Default::default()
            },
            &pointers,
            &mut rx,
        );
        assert_eq!(
            event_count(&mut rx, |event| matches!(
                event,
                EventSignal::DragStart { .. }
            )),
            1
        );

        input.cursor_pos = Some((13.0, 14.0));
        set_hit(
            &mut gestures,
            raycaster,
            other_target,
            [0.0; 3],
            [0.0, 0.0, -1.0],
            1.0,
        );
        gestures.tick_with_rx(
            &world,
            &visuals,
            &input,
            &PointerActivations {
                released: vec![pointer],
                ..Default::default()
            },
            &pointers,
            &mut rx,
        );
        assert_eq!(
            event_count(&mut rx, |event| matches!(event, EventSignal::Click { .. })),
            0,
            "a release over a different click target must cancel the click"
        );
    }

    #[test]
    fn spatial_click_uses_ray_angle_and_origin_instead_of_surface_displacement() {
        let mut world = World::default();
        let camera = world.add_component(CameraXRComponent::on());
        let pointer = world.add_component(
            PointerComponent::new()
                .click_max_ray_angle_deg(2.0)
                .click_max_origin_distance(0.03),
        );
        world.add_child(camera, pointer).unwrap();
        let target = world.add_component(TransformComponent::new());
        let mut pointers = PointerSystem::default();
        let mut rx = RxWorld::default();
        pointers.register_pointer(&mut world, pointer, &mut rx);
        let raycaster = pointers.raycast_for_pointer(pointer).unwrap();
        let visuals = VisualWorld::default();
        let input = InputState::default();
        let mut gestures = GestureSystem::default();

        set_hit(
            &mut gestures,
            raycaster,
            target,
            [0.0; 3],
            [0.0, 0.0, -2.0],
            0.5,
        );
        gestures.tick_with_rx(
            &world,
            &visuals,
            &input,
            &PointerActivations {
                pressed: vec![pointer],
                ..Default::default()
            },
            &pointers,
            &mut rx,
        );
        let _ = rx.drain_ready_events();

        // The release surface point is metres away from the press point, but the normalized ray
        // direction is unchanged and the origin moved only 2 cm.
        set_hit(
            &mut gestures,
            raycaster,
            target,
            [0.02, 0.0, 0.0],
            [0.0, 0.0, -0.25],
            20.0,
        );
        gestures.tick_with_rx(
            &world,
            &visuals,
            &input,
            &PointerActivations {
                released: vec![pointer],
                ..Default::default()
            },
            &pointers,
            &mut rx,
        );
        assert_eq!(
            event_count(&mut rx, |event| matches!(event, EventSignal::Click { .. })),
            1
        );
    }
}

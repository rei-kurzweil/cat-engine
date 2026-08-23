use crate::engine::ecs::component::xr_eye_tracking::EyeGazeSample;
use crate::engine::ecs::component::{XREyeTrackingComponent, XREyeTrackingHtcComponent};
use crate::engine::ecs::{ComponentId, EventSignal, SignalEmitter, World};
use std::collections::{HashMap, HashSet};
use std::net::UdpSocket;

#[derive(Debug, Default)]
pub struct XREyeTrackingSystem {
    sockets: HashMap<ComponentId, UdpSocket>,
    htc_sockets: HashMap<ComponentId, UdpSocket>,
    /// A failed bind is retried so a port becoming available recovers without
    /// restarting the scene, but each active component reports that failure
    /// only once.
    failed_standard_binds: HashSet<ComponentId>,
    failed_htc_binds: HashSet<ComponentId>,
    /// OSC sends gaze vectors and openness as independent packets. Keep the
    /// latest value for every field so one packet cannot erase the others
    /// before the script callback sees them.
    standard_samples: HashMap<ComponentId, StandardEyeSample>,
    receive_sequence: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct StandardEyeSample {
    combined_look: Option<Look>,
    left_look: Option<Look>,
    right_look: Option<Look>,
    combined_openness: Option<f32>,
}
impl XREyeTrackingSystem {
    pub fn tick(&mut self, world: &mut World, emit: &mut dyn SignalEmitter) {
        self.tick_standard(world, emit);
        self.tick_htc(world, emit);
    }
    fn tick_standard(&mut self, world: &mut World, emit: &mut dyn SignalEmitter) {
        let ids: Vec<_> = world
            .all_components()
            .filter(|&id| {
                world
                    .get_component_by_id_as::<XREyeTrackingComponent>(id)
                    .is_some()
            })
            .collect();
        self.sockets.retain(|id, _| ids.contains(id));
        self.standard_samples.retain(|id, _| ids.contains(id));
        self.failed_standard_binds.retain(|id| ids.contains(id));
        for id in ids {
            let c = world
                .get_component_by_id_as::<XREyeTrackingComponent>(id)
                .unwrap();
            if !self.sockets.contains_key(&id) {
                match UdpSocket::bind(format!("{}:{}", c.host, c.port)) {
                    Ok(s) => {
                        let _ = s.set_nonblocking(true);
                        self.failed_standard_binds.remove(&id);
                        eprintln!("[XREyeTracking] listening on {}:{}", c.host, c.port);
                        self.sockets.insert(id, s);
                    }
                    Err(error) => {
                        if self.failed_standard_binds.insert(id) {
                            eprintln!("[XREyeTracking] cannot bind {}:{}: {error}", c.host, c.port);
                        }
                    }
                }
            }
            if let Some(s) = self.sockets.get(&id) {
                let mut bytes = [0u8; 1024];
                let mut received_update = false;
                let mut received_gaze = false;
                while let Ok(len) = s.recv(&mut bytes) {
                    let packet = &bytes[..len];
                    match decode_osc(packet) {
                        Some((combined_look, left_look, right_look, combined_openness)) => {
                            received_gaze |= combined_look.is_some()
                                || left_look.is_some()
                                || right_look.is_some();
                            let sample = self.standard_samples.entry(id).or_default();
                            if combined_look.is_some() {
                                sample.combined_look = combined_look;
                            }
                            if left_look.is_some() {
                                sample.left_look = left_look;
                            }
                            if right_look.is_some() {
                                sample.right_look = right_look;
                            }
                            if combined_openness.is_some() {
                                sample.combined_openness = combined_openness;
                            }
                            received_update = true;
                        }
                        None => {}
                    }
                }
                // A face tracker can submit many UDP packets between rendered
                // frames. Keep only the newest sample so script callbacks and
                // renderer mutations remain bounded to one per frame/source.
                if received_update {
                    let sample = self.standard_samples.get(&id).copied().unwrap_or_default();
                    if received_gaze {
                        let gaze_sample = standard_gaze_sample(sample, self.next_sequence())
                            .unwrap_or(EyeGazeSample {
                                sequence: self.receive_sequence,
                                ..EyeGazeSample::default()
                            });
                        if let Some(component) =
                            world.get_component_by_id_as_mut::<XREyeTrackingComponent>(id)
                        {
                            component.gaze_sample = gaze_sample;
                        }
                    }
                    emit.push_event(
                        id,
                        EventSignal::XrEyeTrackingUpdated {
                            combined_look: sample.combined_look,
                            left_look: sample.left_look,
                            right_look: sample.right_look,
                            combined_openness: sample.combined_openness,
                        },
                    );
                }
            }
        }
    }
    fn tick_htc(&mut self, world: &mut World, emit: &mut dyn SignalEmitter) {
        let ids: Vec<_> = world
            .all_components()
            .filter(|&id| {
                world
                    .get_component_by_id_as::<XREyeTrackingHtcComponent>(id)
                    .is_some()
            })
            .collect();
        self.htc_sockets.retain(|id, _| ids.contains(id));
        self.failed_htc_binds.retain(|id| ids.contains(id));
        for id in ids {
            let c = world
                .get_component_by_id_as::<XREyeTrackingHtcComponent>(id)
                .unwrap();
            if !self.htc_sockets.contains_key(&id) {
                match UdpSocket::bind(format!("{}:{}", c.host, c.port)) {
                    Ok(s) => {
                        let _ = s.set_nonblocking(true);
                        self.failed_htc_binds.remove(&id);
                        eprintln!("[XREyeTrackingHTC] listening on {}:{}", c.host, c.port);
                        self.htc_sockets.insert(id, s);
                    }
                    Err(error) => {
                        if self.failed_htc_binds.insert(id) {
                            eprintln!(
                                "[XREyeTrackingHTC] cannot bind {}:{}: {error}",
                                c.host, c.port
                            );
                        }
                    }
                }
            }
            if let Some(s) = self.htc_sockets.get(&id) {
                let mut b = [0u8; 1024];
                let mut packets = Vec::new();
                while let Ok(n) = s.recv(&mut b) {
                    if let Some((left, right)) = decode_htc(&b[..n]) {
                        packets.push((left, right));
                    }
                }
                for (left, right) in packets {
                    let sequence = self.next_sequence();
                    let gaze_sample =
                        htc_gaze_sample(&left, &right, sequence).unwrap_or(EyeGazeSample {
                            sequence,
                            ..EyeGazeSample::default()
                        });
                    if let Some(component) =
                        world.get_component_by_id_as_mut::<XREyeTrackingHtcComponent>(id)
                    {
                        component.gaze_sample = gaze_sample;
                    }
                    emit.push_event(id, EventSignal::XrEyeTrackingHtcUpdated { left, right });
                }
            }
        }
    }
    fn next_sequence(&mut self) -> u64 {
        self.receive_sequence = self.receive_sequence.wrapping_add(1);
        self.receive_sequence
    }
}
pub type Look = [f32; 3];

fn normalized_look(look: Look) -> Option<Look> {
    if !look.iter().all(|value| value.is_finite()) {
        return None;
    }
    let length_sq = look.iter().map(|value| value * value).sum::<f32>();
    (length_sq > 1e-12).then(|| {
        let inv = length_sq.sqrt().recip();
        [look[0] * inv, look[1] * inv, look[2] * inv]
    })
}

fn standard_gaze_sample(sample: StandardEyeSample, sequence: u64) -> Option<EyeGazeSample> {
    let left = sample
        .left_look
        .and_then(normalized_look)
        .or_else(|| sample.combined_look.and_then(normalized_look));
    let right = sample
        .right_look
        .and_then(normalized_look)
        .or_else(|| sample.combined_look.and_then(normalized_look));
    (left.is_some() || right.is_some()).then_some(EyeGazeSample {
        left,
        right,
        sequence,
    })
}

fn htc_gaze_sample(left: &HtcEye, right: &HtcEye, sequence: u64) -> Option<EyeGazeSample> {
    let left = left.look.and_then(normalized_look);
    let right = right.look.and_then(normalized_look);
    (left.is_some() || right.is_some()).then_some(EyeGazeSample {
        left,
        right,
        sequence,
    })
}
/// Decodes ALVR's VRChat Eye OSC messages. Unknown OSC arguments/messages are ignored.
pub fn decode_osc(
    packet: &[u8],
) -> Option<(Option<Look>, Option<Look>, Option<Look>, Option<f32>)> {
    let mut i = 0;
    let addr = osc_str(packet, &mut i)?;
    let tags = osc_str(packet, &mut i)?;
    let scalar = |i: &mut usize| {
        let v = osc_f32(packet, i)?;
        v.is_finite().then_some(v)
    };
    match (addr, tags) {
        ("/avatar/parameters/EyesClosedAmount" | "/tracking/eye/EyesClosedAmount", ",f") => Some((
            None,
            None,
            None,
            Some((1.0 - scalar(&mut i)?).clamp(0.0, 1.0)),
        )),
        ("/tracking/eye/CenterVec", ",fff") => Some((
            Some([scalar(&mut i)?, scalar(&mut i)?, scalar(&mut i)?]),
            None,
            None,
            None,
        )),
        ("/tracking/eye/LeftEyeVec", ",fff") => Some((
            None,
            Some([scalar(&mut i)?, scalar(&mut i)?, scalar(&mut i)?]),
            None,
            None,
        )),
        ("/tracking/eye/RightEyeVec", ",fff") => Some((
            None,
            None,
            Some([scalar(&mut i)?, scalar(&mut i)?, scalar(&mut i)?]),
            None,
        )),
        ("/tracking/eye/LeftRightVec", ",ffffff") => Some((
            None,
            Some([scalar(&mut i)?, scalar(&mut i)?, scalar(&mut i)?]),
            Some([scalar(&mut i)?, scalar(&mut i)?, scalar(&mut i)?]),
            None,
        )),
        ("/tracking/eye/CenterPitchYaw", ",ff") => Some((
            Some(look_from_pitch_yaw(scalar(&mut i)?, scalar(&mut i)?)),
            None,
            None,
            None,
        )),
        ("/tracking/eye/LeftRightPitchYaw", ",ffff") => Some((
            None,
            Some(look_from_pitch_yaw(scalar(&mut i)?, scalar(&mut i)?)),
            Some(look_from_pitch_yaw(scalar(&mut i)?, scalar(&mut i)?)),
            None,
        )),
        _ => None,
    }
}
fn look_from_pitch_yaw(pitch_deg: f32, yaw_deg: f32) -> Look {
    let pitch = pitch_deg.to_radians();
    let yaw = yaw_deg.to_radians();
    [
        yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    ]
}
fn osc_str<'a>(b: &'a [u8], i: &mut usize) -> Option<&'a str> {
    let end = b.get(*i..)?.iter().position(|&x| x == 0)? + *i;
    let s = std::str::from_utf8(&b[*i..end]).ok()?;
    *i = (end + 4) & !3;
    Some(s)
}
fn osc_f32(b: &[u8], i: &mut usize) -> Option<f32> {
    let x = f32::from_bits(u32::from_be_bytes(b.get(*i..*i + 4)?.try_into().ok()?));
    *i += 4;
    Some(x)
}
#[derive(Debug, Clone, PartialEq)]
pub struct HtcEye {
    pub look: Option<Look>,
    pub position: Option<[f32; 2]>,
    pub openness: Option<f32>,
    pub wide: Option<f32>,
    pub squeeze: Option<f32>,
    pub pupil_diameter: Option<f32>,
}
/// ALVR Mittens packet: 84 byte, `M`, protocol v1, little-endian floats. Layout follows the ALVR face tracking wire format.
pub fn decode_htc(b: &[u8]) -> Option<(HtcEye, HtcEye)> {
    if b.len() != 84 || b[0] != b'M' || b[1] != 1 {
        return None;
    }
    let mut p = 4;
    let eye = |p: &mut usize| {
        let flags = u32::from_le_bytes(b.get(*p..*p + 4)?.try_into().ok()?);
        *p += 4;
        let f = |p: &mut usize| {
            let x = f32::from_le_bytes(b.get(*p..*p + 4)?.try_into().ok()?);
            *p += 4;
            Some(x)
        };
        let vals = [
            f(p)?,
            f(p)?,
            f(p)?,
            f(p)?,
            f(p)?,
            f(p)?,
            f(p)?,
            f(p)?,
            f(p)?,
        ];
        if vals.iter().any(|x| !x.is_finite()) {
            return None;
        };
        Some(HtcEye {
            look: if flags & 1 != 0 {
                Some([vals[0], vals[1], vals[2]])
            } else {
                None
            },
            position: if flags & 2 != 0 {
                Some([vals[3], vals[4]])
            } else {
                None
            },
            openness: if flags & 4 != 0 { Some(vals[5]) } else { None },
            wide: if flags & 8 != 0 { Some(vals[6]) } else { None },
            squeeze: if flags & 16 != 0 { Some(vals[7]) } else { None },
            pupil_diameter: if flags & 32 != 0 { Some(vals[8]) } else { None },
        })
    };
    Some((eye(&mut p)?, eye(&mut p)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_sample_prefers_individual_and_falls_back_to_combined() {
        let sample = standard_gaze_sample(
            StandardEyeSample {
                combined_look: Some([0.0, 0.0, -2.0]),
                left_look: Some([3.0, 0.0, 0.0]),
                ..StandardEyeSample::default()
            },
            9,
        )
        .expect("usable gaze");
        assert_eq!(sample.sequence, 9);
        assert_eq!(sample.left, Some([1.0, 0.0, 0.0]));
        assert_eq!(sample.right, Some([0.0, 0.0, -1.0]));

        let invalid_individual = standard_gaze_sample(
            StandardEyeSample {
                combined_look: Some([0.0, 0.0, -1.0]),
                left_look: Some([0.0, 0.0, 0.0]),
                ..StandardEyeSample::default()
            },
            10,
        )
        .expect("combined fallback");
        assert_eq!(invalid_individual.left, Some([0.0, 0.0, -1.0]));
    }

    #[test]
    fn invalid_or_zero_gaze_is_not_a_pose_sample() {
        assert_eq!(normalized_look([0.0, 0.0, 0.0]), None);
        assert_eq!(normalized_look([f32::NAN, 0.0, -1.0]), None);
        assert!(
            htc_gaze_sample(
                &HtcEye {
                    look: Some([0.0, 0.0, 0.0]),
                    position: None,
                    openness: None,
                    wide: None,
                    squeeze: None,
                    pupil_diameter: None
                },
                &HtcEye {
                    look: None,
                    position: None,
                    openness: None,
                    wide: None,
                    squeeze: None,
                    pupil_diameter: None
                },
                1,
            )
            .is_none()
        );
    }
}

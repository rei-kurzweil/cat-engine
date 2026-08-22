use crate::engine::ecs::component::{XREyeTrackingComponent, XREyeTrackingHtcComponent};
use crate::engine::ecs::{ComponentId, EventSignal, SignalEmitter, World};
use std::collections::HashMap;
use std::net::UdpSocket;

#[derive(Debug, Default)]
pub struct XREyeTrackingSystem {
    sockets: HashMap<ComponentId, UdpSocket>,
    htc_sockets: HashMap<ComponentId, UdpSocket>,
}
impl XREyeTrackingSystem {
    pub fn tick(&mut self, world: &World, emit: &mut dyn SignalEmitter) {
        self.tick_standard(world, emit);
        self.tick_htc(world, emit);
    }
    fn tick_standard(&mut self, world: &World, emit: &mut dyn SignalEmitter) {
        let ids: Vec<_> = world
            .all_components()
            .filter(|&id| {
                world
                    .get_component_by_id_as::<XREyeTrackingComponent>(id)
                    .is_some()
            })
            .collect();
        self.sockets.retain(|id, _| ids.contains(id));
        for id in ids {
            let c = world
                .get_component_by_id_as::<XREyeTrackingComponent>(id)
                .unwrap();
            if !self.sockets.contains_key(&id) {
                if let Ok(s) = UdpSocket::bind(format!("{}:{}", c.host, c.port)) {
                    let _ = s.set_nonblocking(true);
                    self.sockets.insert(id, s);
                }
            }
            if let Some(s) = self.sockets.get(&id) {
                drain(
                    s,
                    |b| {
                        decode_osc(b).map(|v| EventSignal::XrEyeTrackingUpdated {
                            combined_look: v.0,
                            left_look: v.1,
                            right_look: v.2,
                            combined_openness: v.3,
                        })
                    },
                    id,
                    emit,
                );
            }
        }
    }
    fn tick_htc(&mut self, world: &World, emit: &mut dyn SignalEmitter) {
        let ids: Vec<_> = world
            .all_components()
            .filter(|&id| {
                world
                    .get_component_by_id_as::<XREyeTrackingHtcComponent>(id)
                    .is_some()
            })
            .collect();
        self.htc_sockets.retain(|id, _| ids.contains(id));
        for id in ids {
            let c = world
                .get_component_by_id_as::<XREyeTrackingHtcComponent>(id)
                .unwrap();
            if !self.htc_sockets.contains_key(&id) {
                if let Ok(s) = UdpSocket::bind(format!("{}:{}", c.host, c.port)) {
                    let _ = s.set_nonblocking(true);
                    self.htc_sockets.insert(id, s);
                }
            }
            if let Some(s) = self.htc_sockets.get(&id) {
                drain(
                    s,
                    |b| {
                        decode_htc(b).map(|(l, r)| EventSignal::XrEyeTrackingHtcUpdated {
                            left: l,
                            right: r,
                        })
                    },
                    id,
                    emit,
                );
            }
        }
    }
}
fn drain<F: Fn(&[u8]) -> Option<EventSignal>>(
    s: &UdpSocket,
    f: F,
    id: ComponentId,
    emit: &mut dyn SignalEmitter,
) {
    let mut b = [0u8; 1024];
    while let Ok(n) = s.recv(&mut b) {
        if let Some(e) = f(&b[..n]) {
            emit.push_event(id, e);
        }
    }
}
pub type Look = [f32; 3];
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
        _ => None,
    }
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

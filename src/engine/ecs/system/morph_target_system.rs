//! Runtime helpers for GLTF-owned morph target state.
//!
//! The renderer consumes `active_palette` to avoid dispatching targets whose
//! effective factor is within the system epsilon. Keeping this small policy
//! module independent of Vulkano also makes the threshold contract testable.
use crate::engine::ecs::component::{MorphFactorState, MorphTargetKey};
use std::collections::BTreeMap;

pub(crate) fn active_palette(
    factors: &BTreeMap<MorphTargetKey, MorphFactorState>,
) -> Vec<(MorphTargetKey, f32)> {
    crate::engine::ecs::component::morph_target::active_factors(factors.iter())
}

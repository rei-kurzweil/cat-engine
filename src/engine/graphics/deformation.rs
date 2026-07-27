//! Data contracts and CPU-side scheduling for compute-cached mesh deformation.
//!
//! The types in this module are the Rust half of the `std430` declarations in
//! `assets/shaders/mesh-deformation.comp`.

use std::collections::BTreeMap;
use std::mem::offset_of;

use vulkano::buffer::BufferContents;

pub const DEFORMATION_LOCAL_SIZE_X: u32 = 64;
pub const ZERO_NORMAL_SENTINEL: u32 = 0x8000_8000;
pub const OCT_NORMAL_MAX_ANGULAR_ERROR_RADIANS: f32 = 0.0001;

#[derive(BufferContents, Clone, Copy, Debug, Default, PartialEq)]
#[repr(C, align(16))]
pub struct GpuBaseDeformationVertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
}

#[derive(BufferContents, Clone, Copy, Debug, Default, PartialEq)]
#[repr(C, align(16))]
pub struct GpuDeformationSkinVertex {
    pub joints: [u32; 4],
    pub weights: [f32; 4],
}

#[derive(BufferContents, Clone, Copy, Debug, Default, PartialEq)]
#[repr(C, align(16))]
pub struct GpuMorphDelta {
    pub position_delta: [f32; 4],
    pub normal_delta: [f32; 4],
}

#[derive(BufferContents, Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct GpuActiveMorph {
    pub delta_base: u32,
    pub weight: f32,
}

#[derive(BufferContents, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct GpuDeformationJob {
    pub base_vertex: u32,
    pub skin_vertex: u32,
    pub output_vertex: u32,
    pub vertex_count: u32,
    pub bones_base: u32,
    pub bones_count: u32,
    pub active_morph_base: u32,
    pub active_morph_count: u32,
}

#[derive(BufferContents, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct GpuDeformationWorkgroup {
    pub job_index: u32,
    pub first_vertex: u32,
}

#[derive(BufferContents, Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct GpuDeformedVertex {
    pub position: [f32; 3],
    pub packed_normal: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct DeformationRange {
    pub base: u32,
    pub vertex_count: u32,
}

/// Builds the indirection records consumed by a single one-dimensional compute dispatch.
pub fn build_workgroups(jobs: &[GpuDeformationJob]) -> Vec<GpuDeformationWorkgroup> {
    let mut workgroups = Vec::new();
    for (job_index, job) in jobs.iter().enumerate() {
        let mut first_vertex = 0;
        while first_vertex < job.vertex_count {
            workgroups.push(GpuDeformationWorkgroup {
                job_index: job_index as u32,
                first_vertex,
            });
            first_vertex += DEFORMATION_LOCAL_SIZE_X;
        }
    }
    workgroups
}

/// Stable first-fit range allocator. Freed neighbours are always coalesced.
#[derive(Clone, Debug, Default)]
pub struct DeformationRangeAllocator {
    capacity: u32,
    live_vertices: u32,
    free: BTreeMap<u32, u32>,
}

impl DeformationRangeAllocator {
    pub fn with_capacity(capacity: u32) -> Self {
        let mut allocator = Self {
            capacity,
            ..Self::default()
        };
        if capacity != 0 {
            allocator.free.insert(0, capacity);
        }
        allocator
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn live_vertices(&self) -> u32 {
        self.live_vertices
    }

    pub fn allocate(&mut self, vertex_count: u32) -> Option<DeformationRange> {
        if vertex_count == 0 {
            return Some(DeformationRange::default());
        }
        let (&base, &length) = self
            .free
            .iter()
            .find(|(_, length)| **length >= vertex_count)?;
        self.free.remove(&base);
        if length > vertex_count {
            self.free.insert(base + vertex_count, length - vertex_count);
        }
        self.live_vertices += vertex_count;
        Some(DeformationRange { base, vertex_count })
    }

    pub fn free(&mut self, range: DeformationRange) {
        if range.vertex_count == 0 {
            return;
        }
        debug_assert!(range.base.saturating_add(range.vertex_count) <= self.capacity);
        self.live_vertices = self.live_vertices.saturating_sub(range.vertex_count);

        let mut base = range.base;
        let mut length = range.vertex_count;
        if let Some((&previous_base, &previous_length)) = self.free.range(..base).next_back() {
            if previous_base + previous_length == base {
                base = previous_base;
                length += previous_length;
                self.free.remove(&previous_base);
            }
        }
        if let Some((&next_base, &next_length)) = self.free.range(base..).next() {
            if base + length == next_base {
                length += next_length;
                self.free.remove(&next_base);
            }
        }
        self.free.insert(base, length);
    }

    /// Extends the address space without moving any live range.
    pub fn grow(&mut self, new_capacity: u32) {
        assert!(new_capacity >= self.capacity);
        if new_capacity == self.capacity {
            return;
        }
        let extension = DeformationRange {
            base: self.capacity,
            vertex_count: new_capacity - self.capacity,
        };
        self.capacity = new_capacity;
        // `free` adjusts live count, so merge the extension directly instead.
        let mut base = extension.base;
        let mut length = extension.vertex_count;
        if let Some((&previous_base, &previous_length)) = self.free.range(..base).next_back() {
            if previous_base + previous_length == base {
                base = previous_base;
                length += previous_length;
                self.free.remove(&previous_base);
            }
        }
        self.free.insert(base, length);
    }

    pub fn allocate_growing(&mut self, vertex_count: u32) -> (DeformationRange, bool) {
        if let Some(range) = self.allocate(vertex_count) {
            return (range, false);
        }
        let required = self
            .capacity
            .checked_add(vertex_count)
            .expect("deformation cache address space exhausted");
        let new_capacity = required.next_power_of_two().max(1);
        self.grow(new_capacity);
        (
            self.allocate(vertex_count)
                .expect("grown deformation cache must contain requested range"),
            true,
        )
    }
}

fn sign_not_zero(value: f32) -> f32 {
    if value < 0.0 { -1.0 } else { 1.0 }
}

fn snorm16(value: f32) -> i16 {
    (value.clamp(-1.0, 1.0) * 32767.0).round() as i16
}

/// Octahedral SNORM16x2 encoding used by the permanent 16-byte cache format.
pub fn oct_encode_normal(normal: [f32; 3]) -> Option<u32> {
    if !normal.iter().all(|value| value.is_finite()) {
        return None;
    }
    let length_squared = normal.iter().map(|value| value * value).sum::<f32>();
    if length_squared == 0.0 {
        return Some(ZERO_NORMAL_SENTINEL);
    }
    if !length_squared.is_finite() {
        return None;
    }
    let inverse_length = length_squared.sqrt().recip();
    let mut n = [
        normal[0] * inverse_length,
        normal[1] * inverse_length,
        normal[2] * inverse_length,
    ];
    let inverse_l1 = (n[0].abs() + n[1].abs() + n[2].abs()).recip();
    n[0] *= inverse_l1;
    n[1] *= inverse_l1;
    n[2] *= inverse_l1;
    let encoded = if n[2] >= 0.0 {
        [n[0], n[1]]
    } else {
        [
            (1.0 - n[1].abs()) * sign_not_zero(n[0]),
            (1.0 - n[0].abs()) * sign_not_zero(n[1]),
        ]
    };
    let x = snorm16(encoded[0]) as u16 as u32;
    let y = snorm16(encoded[1]) as u16 as u32;
    let packed = x | (y << 16);
    // The sentinel is not a valid encoder result with the 32767 scale, but keep
    // this explicit so future quantizer changes cannot make zero ambiguous.
    debug_assert_ne!(packed, ZERO_NORMAL_SENTINEL);
    Some(packed)
}

pub fn oct_decode_normal(packed: u32) -> [f32; 3] {
    if packed == ZERO_NORMAL_SENTINEL {
        return [0.0; 3];
    }
    let x = (packed as u16 as i16) as f32 / 32767.0;
    let y = ((packed >> 16) as u16 as i16) as f32 / 32767.0;
    let mut n = [x, y, 1.0 - x.abs() - y.abs()];
    if n[2] < 0.0 {
        let old_x = n[0];
        n[0] = (1.0 - n[1].abs()) * sign_not_zero(old_x);
        n[1] = (1.0 - old_x.abs()) * sign_not_zero(n[1]);
    }
    let inverse_length = n
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt()
        .recip();
    [
        n[0] * inverse_length,
        n[1] * inverse_length,
        n[2] * inverse_length,
    ]
}

const _: () = {
    assert!(size_of::<GpuBaseDeformationVertex>() == 32);
    assert!(align_of::<GpuBaseDeformationVertex>() == 16);
    assert!(offset_of!(GpuBaseDeformationVertex, position) == 0);
    assert!(offset_of!(GpuBaseDeformationVertex, normal) == 16);
    assert!(size_of::<GpuDeformationSkinVertex>() == 32);
    assert!(align_of::<GpuDeformationSkinVertex>() == 16);
    assert!(offset_of!(GpuDeformationSkinVertex, weights) == 16);
    assert!(size_of::<GpuMorphDelta>() == 32);
    assert!(align_of::<GpuMorphDelta>() == 16);
    assert!(offset_of!(GpuMorphDelta, normal_delta) == 16);
    assert!(size_of::<GpuActiveMorph>() == 8);
    assert!(align_of::<GpuActiveMorph>() == 4);
    assert!(offset_of!(GpuActiveMorph, weight) == 4);
    assert!(size_of::<GpuDeformationJob>() == 32);
    assert!(align_of::<GpuDeformationJob>() == 4);
    assert!(offset_of!(GpuDeformationJob, active_morph_count) == 28);
    assert!(size_of::<GpuDeformationWorkgroup>() == 8);
    assert!(align_of::<GpuDeformationWorkgroup>() == 4);
    assert!(offset_of!(GpuDeformationWorkgroup, first_vertex) == 4);
    assert!(size_of::<GpuDeformedVertex>() == 16);
    assert!(align_of::<GpuDeformedVertex>() == 4);
    assert!(offset_of!(GpuDeformedVertex, packed_normal) == 12);
    assert!(size_of::<DeformationRange>() == 8);
    assert!(align_of::<DeformationRange>() == 4);
    assert!(offset_of!(DeformationRange, vertex_count) == 4);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workgroup_boundaries_and_multiple_jobs() {
        for (vertices, expected) in [(0, 0), (1, 1), (63, 1), (64, 1), (65, 2)] {
            let groups = build_workgroups(&[GpuDeformationJob {
                vertex_count: vertices,
                ..Default::default()
            }]);
            assert_eq!(groups.len(), expected);
        }
        let groups = build_workgroups(&[
            GpuDeformationJob {
                vertex_count: 65,
                ..Default::default()
            },
            GpuDeformationJob {
                vertex_count: 128,
                ..Default::default()
            },
        ]);
        assert_eq!(
            groups,
            [
                GpuDeformationWorkgroup {
                    job_index: 0,
                    first_vertex: 0
                },
                GpuDeformationWorkgroup {
                    job_index: 0,
                    first_vertex: 64
                },
                GpuDeformationWorkgroup {
                    job_index: 1,
                    first_vertex: 0
                },
                GpuDeformationWorkgroup {
                    job_index: 1,
                    first_vertex: 64
                },
            ]
        );
    }

    #[test]
    fn allocator_is_stable_coalescing_and_reuses_ranges() {
        let mut allocator = DeformationRangeAllocator::with_capacity(16);
        let a = allocator.allocate(4).unwrap();
        let b = allocator.allocate(6).unwrap();
        let c = allocator.allocate(6).unwrap();
        assert_eq!((a.base, b.base, c.base), (0, 4, 10));
        allocator.free(b);
        let reused = allocator.allocate(5).unwrap();
        assert_eq!(reused.base, 4);
        allocator.free(a);
        allocator.free(reused);
        allocator.free(c);
        assert_eq!(allocator.allocate(10).unwrap().base, 0);
        assert_eq!(allocator.live_vertices(), 10);
    }

    #[test]
    fn allocator_grows_without_moving_live_ranges() {
        let mut allocator = DeformationRangeAllocator::with_capacity(4);
        let a = allocator.allocate(4).unwrap();
        let (b, grew) = allocator.allocate_growing(5);
        assert!(grew);
        assert_eq!(a.base, 0);
        assert_eq!(b.base, 4);
        assert_eq!(allocator.capacity(), 16);
    }

    #[test]
    fn octahedral_axes_zero_and_invalid_values() {
        for axis in [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ] {
            let decoded = oct_decode_normal(oct_encode_normal(axis).unwrap());
            assert!(dot(axis, decoded) > 0.999999);
        }
        assert_eq!(oct_encode_normal([0.0; 3]), Some(ZERO_NORMAL_SENTINEL));
        assert_eq!(oct_decode_normal(ZERO_NORMAL_SENTINEL), [0.0; 3]);
        assert_eq!(oct_encode_normal([f32::NAN, 0.0, 0.0]), None);
        assert_eq!(oct_encode_normal([f32::INFINITY, 0.0, 0.0]), None);
    }

    #[test]
    fn randomized_octahedral_angular_error_is_bounded() {
        let mut state = 0x1234_5678_u32;
        for _ in 0..20_000 {
            let mut next = || {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state as f32 / u32::MAX as f32) * 2.0 - 1.0
            };
            let value = [next(), next(), next()];
            let length = dot(value, value).sqrt();
            if length < 1.0e-5 {
                continue;
            }
            let normal = [value[0] / length, value[1] / length, value[2] / length];
            let decoded = oct_decode_normal(oct_encode_normal(normal).unwrap());
            let cross = [
                normal[1] as f64 * decoded[2] as f64 - normal[2] as f64 * decoded[1] as f64,
                normal[2] as f64 * decoded[0] as f64 - normal[0] as f64 * decoded[2] as f64,
                normal[0] as f64 * decoded[1] as f64 - normal[1] as f64 * decoded[0] as f64,
            ];
            let cross_length = cross.iter().map(|value| value * value).sum::<f64>().sqrt();
            let dot64 = normal
                .iter()
                .zip(decoded)
                .map(|(a, b)| *a as f64 * b as f64)
                .sum::<f64>();
            let angle = cross_length.atan2(dot64) as f32;
            assert!(
                angle <= OCT_NORMAL_MAX_ANGULAR_ERROR_RADIANS,
                "{normal:?} decoded as {decoded:?}, error {angle}"
            );
        }
    }

    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }
}

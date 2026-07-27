//! CPU reference for validating compute-cached mesh deformation.
//!
//! Keep this independent from `assets/shaders/mesh-deformation.comp` so GPU readback tests can
//! detect changes in shader behavior.

use crate::engine::graphics::primitives::TransformMatrix;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DeformedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeformationReferenceError {
    AttributeCountMismatch {
        positions: usize,
        normals: usize,
        joints: usize,
        weights: usize,
    },
    JointOutOfRange {
        vertex: usize,
        joint: u16,
        bones: usize,
    },
    MorphDeltaOutOfRange {
        vertex: usize,
        delta: usize,
        deltas: usize,
    },
    NonFiniteInput,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ActiveMorph {
    pub delta_base: usize,
    pub weight: f32,
}

/// Deforms mesh-local positions and normals using compute-cache semantics.
///
/// In particular:
///
/// - matrices and vectors use the engine's column-major `TransformMatrix` representation;
/// - all four matrices are blended directly, without normalizing their weights;
/// - an empty bones palette or non-positive weight sum selects the identity matrix;
/// - the blended matrix's upper-left 3x3 transforms the normal, which is then normalized.
pub(crate) fn deform_vertices(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    bones: &[TransformMatrix],
) -> Result<Vec<DeformedVertex>, DeformationReferenceError> {
    deform_vertices_with_morphs(positions, normals, joints, weights, bones, &[], &[])
}

pub(crate) fn deform_vertices_with_morphs(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    bones: &[TransformMatrix],
    morph_deltas: &[([f32; 3], [f32; 3])],
    active_morphs: &[ActiveMorph],
) -> Result<Vec<DeformedVertex>, DeformationReferenceError> {
    let vertex_count = positions.len();
    if normals.len() != vertex_count
        || joints.len() != vertex_count
        || weights.len() != vertex_count
    {
        return Err(DeformationReferenceError::AttributeCountMismatch {
            positions: vertex_count,
            normals: normals.len(),
            joints: joints.len(),
            weights: weights.len(),
        });
    }
    if positions
        .iter()
        .flatten()
        .chain(normals.iter().flatten())
        .chain(weights.iter().flatten())
        .chain(bones.iter().flatten().flatten())
        .chain(
            morph_deltas
                .iter()
                .flat_map(|(position, normal)| position.iter().chain(normal.iter())),
        )
        .chain(active_morphs.iter().map(|morph| &morph.weight))
        .any(|value| !value.is_finite())
    {
        return Err(DeformationReferenceError::NonFiniteInput);
    }

    positions
        .iter()
        .copied()
        .zip(normals.iter().copied())
        .zip(joints.iter().copied())
        .zip(weights.iter().copied())
        .enumerate()
        .map(
            |(vertex, (((position, normal), vertex_joints), vertex_weights))| {
                let mut position = position;
                let mut normal = normal;
                for active in active_morphs {
                    let delta_index = active.delta_base + vertex;
                    let Some((position_delta, normal_delta)) =
                        morph_deltas.get(delta_index).copied()
                    else {
                        return Err(DeformationReferenceError::MorphDeltaOutOfRange {
                            vertex,
                            delta: delta_index,
                            deltas: morph_deltas.len(),
                        });
                    };
                    for axis in 0..3 {
                        position[axis] += position_delta[axis] * active.weight;
                        normal[axis] += normal_delta[axis] * active.weight;
                    }
                }
                let weight_sum = vertex_weights.iter().sum::<f32>();
                let skin = if bones.is_empty() || weight_sum <= 0.0 {
                    identity()
                } else {
                    blend_skin_matrix(vertex, vertex_joints, vertex_weights, bones)?
                };

                let position = transform_position(skin, position);
                let normal = normalize(transform_direction(skin, normal));

                Ok(DeformedVertex { position, normal })
            },
        )
        .collect()
}

fn blend_skin_matrix(
    vertex: usize,
    joints: [u16; 4],
    weights: [f32; 4],
    bones: &[TransformMatrix],
) -> Result<TransformMatrix, DeformationReferenceError> {
    let mut skin = [[0.0; 4]; 4];
    for (joint, weight) in joints.into_iter().zip(weights) {
        let Some(bone) = bones.get(joint as usize) else {
            return Err(DeformationReferenceError::JointOutOfRange {
                vertex,
                joint,
                bones: bones.len(),
            });
        };
        for column in 0..4 {
            for row in 0..4 {
                skin[column][row] += bone[column][row] * weight;
            }
        }
    }
    Ok(skin)
}

fn identity() -> TransformMatrix {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn transform_position(matrix: TransformMatrix, position: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * position[0]
            + matrix[1][0] * position[1]
            + matrix[2][0] * position[2]
            + matrix[3][0],
        matrix[0][1] * position[0]
            + matrix[1][1] * position[1]
            + matrix[2][1] * position[2]
            + matrix[3][1],
        matrix[0][2] * position[0]
            + matrix[1][2] * position[1]
            + matrix[2][2] * position[2]
            + matrix[3][2],
    ]
}

fn transform_direction(matrix: TransformMatrix, direction: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * direction[0] + matrix[1][0] * direction[1] + matrix[2][0] * direction[2],
        matrix[0][1] * direction[0] + matrix[1][1] * direction[1] + matrix[2][1] * direction[2],
        matrix[0][2] * direction[0] + matrix[1][2] * direction[1] + matrix[2][2] * direction[2],
    ]
}

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if length > 0.0 {
        [vector[0] / length, vector[1] / length, vector[2] / length]
    } else {
        vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1.0e-6;

    fn translation(x: f32, y: f32, z: f32) -> TransformMatrix {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [x, y, z, 1.0],
        ]
    }

    fn rotation_z_90() -> TransformMatrix {
        [
            [0.0, 1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    fn assert_vec3_close(actual: [f32; 3], expected: [f32; 3]) {
        for axis in 0..3 {
            assert!(
                (actual[axis] - expected[axis]).abs() <= EPSILON,
                "axis {axis}: expected {}, got {}",
                expected[axis],
                actual[axis]
            );
        }
    }

    #[test]
    fn empty_palette_uses_identity_deformation() {
        let output = deform_vertices(
            &[[1.0, 2.0, 3.0]],
            &[[0.0, 3.0, 0.0]],
            &[[99, 98, 97, 96]],
            &[[1.0, 0.0, 0.0, 0.0]],
            &[],
        )
        .unwrap();

        assert_vec3_close(output[0].position, [1.0, 2.0, 3.0]);
        assert_vec3_close(output[0].normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn single_joint_deforms_position_in_mesh_local_space() {
        let output = deform_vertices(
            &[[1.0, 2.0, 3.0]],
            &[[0.0, 0.0, 1.0]],
            &[[0, 0, 0, 0]],
            &[[1.0, 0.0, 0.0, 0.0]],
            &[translation(4.0, -2.0, 0.5)],
        )
        .unwrap();

        assert_vec3_close(output[0].position, [5.0, 0.0, 3.5]);
        assert_vec3_close(output[0].normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn four_joint_weights_blend_without_renormalization() {
        let bones = [
            translation(4.0, 0.0, 0.0),
            translation(0.0, 8.0, 0.0),
            translation(0.0, 0.0, 12.0),
            translation(-4.0, 0.0, 0.0),
        ];
        let output = deform_vertices(
            &[[1.0, 1.0, 1.0]],
            &[[0.0, 1.0, 0.0]],
            &[[0, 1, 2, 3]],
            &[[0.25, 0.25, 0.25, 0.25]],
            &bones,
        )
        .unwrap();

        assert_vec3_close(output[0].position, [1.0, 3.0, 4.0]);
        assert_vec3_close(output[0].normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn non_unit_weight_sum_is_not_renormalized() {
        let output = deform_vertices(
            &[[1.0, 0.0, 0.0]],
            &[[0.0, 1.0, 0.0]],
            &[[0, 0, 0, 0]],
            &[[2.0, 0.0, 0.0, 0.0]],
            &[translation(3.0, 0.0, 0.0)],
        )
        .unwrap();
        assert_vec3_close(output[0].position, [8.0, 0.0, 0.0]);
    }

    #[test]
    fn non_uniform_normal_transform_is_normalized_once() {
        let scale = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let output = deform_vertices(
            &[[0.0; 3]],
            &[[1.0, 1.0, 0.0]],
            &[[0; 4]],
            &[[1.0, 0.0, 0.0, 0.0]],
            &[scale],
        )
        .unwrap();
        let inverse_length = 13.0_f32.sqrt().recip();
        assert_vec3_close(
            output[0].normal,
            [2.0 * inverse_length, 3.0 * inverse_length, 0.0],
        );
    }

    #[test]
    fn morph_targets_accumulate_before_skinning() {
        let deltas = [
            ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([0.0, 0.0, 2.0], [1.0, 0.0, 0.0]),
        ];
        let output = deform_vertices_with_morphs(
            &[[1.0, 0.0, 0.0]],
            &[[0.0, 0.0, 1.0]],
            &[[0; 4]],
            &[[1.0, 0.0, 0.0, 0.0]],
            &[rotation_z_90()],
            &deltas,
            &[
                ActiveMorph {
                    delta_base: 0,
                    weight: 2.0,
                },
                ActiveMorph {
                    delta_base: 1,
                    weight: 0.5,
                },
            ],
        )
        .unwrap();
        assert_vec3_close(output[0].position, [0.0, 3.0, 1.0]);
        let inverse_length = 5.25_f32.sqrt().recip();
        assert_vec3_close(
            output[0].normal,
            [-2.0 * inverse_length, 0.5 * inverse_length, inverse_length],
        );
    }

    #[test]
    fn non_finite_values_fail_before_deformation() {
        assert_eq!(
            deform_vertices(
                &[[0.0; 3]],
                &[[0.0, 1.0, 0.0]],
                &[[0; 4]],
                &[[f32::NAN, 0.0, 0.0, 0.0]],
                &[identity()],
            ),
            Err(DeformationReferenceError::NonFiniteInput)
        );
    }

    #[test]
    fn normal_uses_blended_matrix_and_is_normalized() {
        let output = deform_vertices(
            &[[2.0, 0.0, 0.0]],
            &[[2.0, 0.0, 0.0]],
            &[[0, 0, 0, 0]],
            &[[1.0, 0.0, 0.0, 0.0]],
            &[rotation_z_90()],
        )
        .unwrap();

        assert_vec3_close(output[0].position, [0.0, 2.0, 0.0]);
        assert_vec3_close(output[0].normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn shared_mesh_can_be_deformed_by_independent_instances() {
        let positions = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0]; 2];
        let joints = [[0, 0, 0, 0]; 2];
        let weights = [[1.0, 0.0, 0.0, 0.0]; 2];

        let instance_a = deform_vertices(
            &positions,
            &normals,
            &joints,
            &weights,
            &[translation(1.0, 0.0, 0.0)],
        )
        .unwrap();
        let instance_b = deform_vertices(
            &positions,
            &normals,
            &joints,
            &weights,
            &[translation(0.0, 2.0, 0.0)],
        )
        .unwrap();

        assert_vec3_close(instance_a[0].position, [2.0, 0.0, 0.0]);
        assert_vec3_close(instance_a[1].position, [1.0, 1.0, 0.0]);
        assert_vec3_close(instance_b[0].position, [1.0, 2.0, 0.0]);
        assert_vec3_close(instance_b[1].position, [0.0, 3.0, 0.0]);
    }

    #[test]
    fn malformed_attributes_and_joint_indices_fail_clearly() {
        assert_eq!(
            deform_vertices(
                &[[0.0; 3]],
                &[],
                &[[0; 4]],
                &[[1.0, 0.0, 0.0, 0.0]],
                &[identity()],
            ),
            Err(DeformationReferenceError::AttributeCountMismatch {
                positions: 1,
                normals: 0,
                joints: 1,
                weights: 1,
            })
        );

        assert_eq!(
            deform_vertices(
                &[[0.0; 3]],
                &[[0.0, 1.0, 0.0]],
                &[[1, 0, 0, 0]],
                &[[1.0, 0.0, 0.0, 0.0]],
                &[identity()],
            ),
            Err(DeformationReferenceError::JointOutOfRange {
                vertex: 0,
                joint: 1,
                bones: 1,
            })
        );
    }
}

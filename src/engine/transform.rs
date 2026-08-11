use std::fmt;

/// Column-major 4x4 transform matrix shared by ECS, rendering, XR, and scripting.
pub type TransformMatrix = [[f32; 4]; 4];

/// Coordinate space used when transferring copied transform values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformSpace {
    Local,
    World,
}

/// Plain copied transform channels with no ECS identity or cached matrices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformTrs {
    pub translation: [f32; 3],
    pub rotation_quat_xyzw: [f32; 4],
    pub scale: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformTrsError {
    NonFiniteValue,
    DegenerateQuaternion,
    NonAffineMatrix,
    SingularScale,
    ReflectedMatrix,
    ShearNotRepresentable,
}

impl fmt::Display for TransformTrsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => f.write_str("transform TRS contains a non-finite value"),
            Self::DegenerateQuaternion => {
                f.write_str("transform TRS contains a degenerate rotation quaternion")
            }
            Self::NonAffineMatrix => f.write_str("transform matrix is not affine"),
            Self::SingularScale => f.write_str("transform matrix has a singular scale axis"),
            Self::ReflectedMatrix => {
                f.write_str("transform matrix contains a reflection or negative determinant")
            }
            Self::ShearNotRepresentable => {
                f.write_str("transform matrix contains shear that TRS cannot represent")
            }
        }
    }
}

impl std::error::Error for TransformTrsError {}

impl TransformTrs {
    pub const IDENTITY: Self = Self {
        translation: [0.0, 0.0, 0.0],
        rotation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    };

    pub const fn new(translation: [f32; 3], rotation_quat_xyzw: [f32; 4], scale: [f32; 3]) -> Self {
        Self {
            translation,
            rotation_quat_xyzw,
            scale,
        }
    }

    pub fn is_finite(self) -> bool {
        self.translation
            .into_iter()
            .chain(self.rotation_quat_xyzw)
            .chain(self.scale)
            .all(f32::is_finite)
    }

    /// Validate all channels and return a copy with a unit quaternion.
    pub fn normalized(self) -> Result<Self, TransformTrsError> {
        if !self.is_finite() {
            return Err(TransformTrsError::NonFiniteValue);
        }

        let [x, y, z, w] = self.rotation_quat_xyzw;
        let len_squared = x * x + y * y + z * z + w * w;
        if len_squared < 1e-12 {
            return Err(TransformTrsError::DegenerateQuaternion);
        }
        let inverse_len = len_squared.sqrt().recip();

        Ok(Self {
            rotation_quat_xyzw: [
                x * inverse_len,
                y * inverse_len,
                z * inverse_len,
                w * inverse_len,
            ],
            ..self
        })
    }

    /// Compose a validated TRS value into a column-major matrix.
    pub fn to_matrix(self) -> Result<TransformMatrix, TransformTrsError> {
        let normalized = self.normalized()?;
        let [tx, ty, tz] = normalized.translation;
        let [sx, sy, sz] = normalized.scale;
        let [x, y, z, w] = normalized.rotation_quat_xyzw;

        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;
        let wx = w * x;
        let wy = w * y;
        let wz = w * z;

        let r00 = 1.0 - 2.0 * (yy + zz);
        let r01 = 2.0 * (xy + wz);
        let r02 = 2.0 * (xz - wy);
        let r10 = 2.0 * (xy - wz);
        let r11 = 1.0 - 2.0 * (xx + zz);
        let r12 = 2.0 * (yz + wx);
        let r20 = 2.0 * (xz + wy);
        let r21 = 2.0 * (yz - wx);
        let r22 = 1.0 - 2.0 * (xx + yy);

        Ok([
            [r00 * sx, r01 * sx, r02 * sx, 0.0],
            [r10 * sy, r11 * sy, r12 * sy, 0.0],
            [r20 * sz, r21 * sz, r22 * sz, 0.0],
            [tx, ty, tz, 1.0],
        ])
    }

    /// Decompose a finite affine matrix into canonical TRS channels.
    ///
    /// This strict conversion rejects singular axes, shear, and matrices with a
    /// negative determinant. An even number of negative input scale axes is
    /// indistinguishable from a rotation and is therefore canonicalized to
    /// positive scale with the equivalent rotation.
    pub fn from_matrix(matrix: TransformMatrix) -> Result<Self, TransformTrsError> {
        const AFFINE_EPSILON: f32 = 1e-5;
        const SCALE_EPSILON: f32 = 1e-7;
        const ORTHOGONAL_EPSILON: f32 = 1e-4;

        if !matrix.into_iter().flatten().all(f32::is_finite) {
            return Err(TransformTrsError::NonFiniteValue);
        }
        if matrix[0][3].abs() > AFFINE_EPSILON
            || matrix[1][3].abs() > AFFINE_EPSILON
            || matrix[2][3].abs() > AFFINE_EPSILON
            || (matrix[3][3] - 1.0).abs() > AFFINE_EPSILON
        {
            return Err(TransformTrsError::NonAffineMatrix);
        }

        fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
            a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
        }
        fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        }
        fn length(value: [f32; 3]) -> f32 {
            dot(value, value).sqrt()
        }
        fn divided(value: [f32; 3], divisor: f32) -> [f32; 3] {
            [value[0] / divisor, value[1] / divisor, value[2] / divisor]
        }

        let columns = [
            [matrix[0][0], matrix[0][1], matrix[0][2]],
            [matrix[1][0], matrix[1][1], matrix[1][2]],
            [matrix[2][0], matrix[2][1], matrix[2][2]],
        ];
        let scale = columns.map(length);
        if scale.into_iter().any(|axis| axis <= SCALE_EPSILON) {
            return Err(TransformTrsError::SingularScale);
        }

        let basis = [
            divided(columns[0], scale[0]),
            divided(columns[1], scale[1]),
            divided(columns[2], scale[2]),
        ];
        if dot(basis[0], basis[1]).abs() > ORTHOGONAL_EPSILON
            || dot(basis[0], basis[2]).abs() > ORTHOGONAL_EPSILON
            || dot(basis[1], basis[2]).abs() > ORTHOGONAL_EPSILON
        {
            return Err(TransformTrsError::ShearNotRepresentable);
        }
        if dot(cross(basis[0], basis[1]), basis[2]) <= 0.0 {
            return Err(TransformTrsError::ReflectedMatrix);
        }

        let rotation_matrix = [
            [basis[0][0], basis[0][1], basis[0][2], 0.0],
            [basis[1][0], basis[1][1], basis[1][2], 0.0],
            [basis[2][0], basis[2][1], basis[2][2], 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        Ok(Self {
            translation: [matrix[3][0], matrix[3][1], matrix[3][2]],
            rotation_quat_xyzw: crate::utils::math::mat_to_quat(rotation_matrix),
            scale,
        })
    }
}

impl Default for TransformTrs {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::{TransformTrs, TransformTrsError};

    #[test]
    fn identity_composes_to_identity_matrix() {
        assert_eq!(
            TransformTrs::IDENTITY.to_matrix().unwrap(),
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]
        );
    }

    #[test]
    fn matrix_composition_normalizes_rotation_and_preserves_trs() {
        let trs = TransformTrs::new([2.0, 3.0, 4.0], [0.0, 0.0, 0.0, 2.0], [5.0, 6.0, 7.0]);
        let normalized = trs.normalized().unwrap();
        assert_eq!(normalized.rotation_quat_xyzw, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(
            trs.to_matrix().unwrap(),
            [
                [5.0, 0.0, 0.0, 0.0],
                [0.0, 6.0, 0.0, 0.0],
                [0.0, 0.0, 7.0, 0.0],
                [2.0, 3.0, 4.0, 1.0],
            ]
        );
    }

    #[test]
    fn validation_rejects_non_finite_values_and_degenerate_rotation() {
        let non_finite =
            TransformTrs::new([f32::NAN, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]);
        assert_eq!(
            non_finite.normalized(),
            Err(TransformTrsError::NonFiniteValue)
        );

        let degenerate = TransformTrs::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        assert_eq!(
            degenerate.normalized(),
            Err(TransformTrsError::DegenerateQuaternion)
        );
    }

    fn assert_matrix_close(actual: [[f32; 4]; 4], expected: [[f32; 4]; 4]) {
        for column in 0..4 {
            for row in 0..4 {
                assert!(
                    (actual[column][row] - expected[column][row]).abs() < 1e-5,
                    "matrix mismatch at [{column}][{row}]: {} != {}",
                    actual[column][row],
                    expected[column][row]
                );
            }
        }
    }

    #[test]
    fn matrix_decomposition_round_trips_canonical_trs() {
        let source = TransformTrs::new(
            [2.0, -3.0, 4.0],
            [
                0.0,
                std::f32::consts::FRAC_1_SQRT_2,
                0.0,
                std::f32::consts::FRAC_1_SQRT_2,
            ],
            [2.0, 3.0, 4.0],
        );
        let matrix = source.to_matrix().unwrap();
        let decomposed = TransformTrs::from_matrix(matrix).unwrap();

        assert_eq!(decomposed.translation, source.translation);
        for axis in 0..3 {
            assert!((decomposed.scale[axis] - source.scale[axis]).abs() < 1e-5);
        }
        assert_matrix_close(decomposed.to_matrix().unwrap(), matrix);
    }

    #[test]
    fn matrix_decomposition_rejects_singular_reflected_and_sheared_matrices() {
        let singular = TransformTrs::new([0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0, 0.0, 1.0])
            .to_matrix()
            .unwrap();
        assert_eq!(
            TransformTrs::from_matrix(singular),
            Err(TransformTrsError::SingularScale)
        );

        let reflected = TransformTrs::new([0.0; 3], [0.0, 0.0, 0.0, 1.0], [-1.0, 1.0, 1.0])
            .to_matrix()
            .unwrap();
        assert_eq!(
            TransformTrs::from_matrix(reflected),
            Err(TransformTrsError::ReflectedMatrix)
        );

        let mut sheared = TransformTrs::IDENTITY.to_matrix().unwrap();
        sheared[1][0] = 0.25;
        assert_eq!(
            TransformTrs::from_matrix(sheared),
            Err(TransformTrsError::ShearNotRepresentable)
        );
    }
}

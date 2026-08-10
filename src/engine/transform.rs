use std::fmt;

/// Column-major 4x4 transform matrix shared by ECS, rendering, XR, and scripting.
pub type TransformMatrix = [[f32; 4]; 4];

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
}

impl fmt::Display for TransformTrsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => f.write_str("transform TRS contains a non-finite value"),
            Self::DegenerateQuaternion => {
                f.write_str("transform TRS contains a degenerate rotation quaternion")
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
}

//! Matriz 3×3 guardada por columnas. Se usa para rotaciones y marcos tangentes.

use super::Vec3;
use std::ops::Mul;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3 {
    pub cols: [Vec3; 3],
}

impl Mat3 {
    pub const IDENTITY: Mat3 = Mat3 {
        cols: [Vec3::X, Vec3::Y, Vec3::Z],
    };

    #[inline]
    pub fn from_cols(c0: Vec3, c1: Vec3, c2: Vec3) -> Mat3 {
        Mat3 { cols: [c0, c1, c2] }
    }

    pub fn rotation_x(angle: f32) -> Mat3 {
        let (s, c) = angle.sin_cos();
        Mat3::from_cols(Vec3::X, Vec3::new(0.0, c, s), Vec3::new(0.0, -s, c))
    }

    pub fn rotation_y(angle: f32) -> Mat3 {
        let (s, c) = angle.sin_cos();
        Mat3::from_cols(Vec3::new(c, 0.0, -s), Vec3::Y, Vec3::new(s, 0.0, c))
    }

    pub fn rotation_z(angle: f32) -> Mat3 {
        let (s, c) = angle.sin_cos();
        Mat3::from_cols(Vec3::new(c, s, 0.0), Vec3::new(-s, c, 0.0), Vec3::Z)
    }

    /// Rotación de `angle` radianes alrededor de un eje arbitrario (fórmula de Rodrigues).
    pub fn rotation_axis(axis: Vec3, angle: f32) -> Mat3 {
        let a = axis.normalized();
        let (s, c) = angle.sin_cos();
        let t = 1.0 - c;
        Mat3::from_cols(
            Vec3::new(
                t * a.x * a.x + c,
                t * a.x * a.y + s * a.z,
                t * a.x * a.z - s * a.y,
            ),
            Vec3::new(
                t * a.x * a.y - s * a.z,
                t * a.y * a.y + c,
                t * a.y * a.z + s * a.x,
            ),
            Vec3::new(
                t * a.x * a.z + s * a.y,
                t * a.y * a.z - s * a.x,
                t * a.z * a.z + c,
            ),
        )
    }

    #[inline]
    pub fn transform(&self, v: Vec3) -> Vec3 {
        self.cols[0] * v.x + self.cols[1] * v.y + self.cols[2] * v.z
    }

    pub fn transpose(&self) -> Mat3 {
        let [a, b, c] = self.cols;
        Mat3::from_cols(
            Vec3::new(a.x, b.x, c.x),
            Vec3::new(a.y, b.y, c.y),
            Vec3::new(a.z, b.z, c.z),
        )
    }
}

impl Mul<Vec3> for Mat3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, v: Vec3) -> Vec3 {
        self.transform(v)
    }
}

impl Mul<Mat3> for Mat3 {
    type Output = Mat3;
    fn mul(self, o: Mat3) -> Mat3 {
        Mat3::from_cols(
            self.transform(o.cols[0]),
            self.transform(o.cols[1]),
            self.transform(o.cols[2]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, TAU};

    fn approx(a: Vec3, b: Vec3, eps: f32) -> bool {
        (a - b).length() < eps
    }

    #[test]
    fn rotation_preserves_length() {
        let v = Vec3::new(1.5, -2.0, 0.7);
        for m in [
            Mat3::rotation_x(0.3),
            Mat3::rotation_y(1.7),
            Mat3::rotation_z(-2.2),
            Mat3::rotation_axis(Vec3::new(1.0, 1.0, 0.5), 0.9),
        ] {
            assert!(((m * v).length() - v.length()).abs() < 1e-5);
        }
    }

    #[test]
    fn full_turn_returns_original() {
        let v = Vec3::new(0.2, 3.0, -1.0);
        for m in [
            Mat3::rotation_x(TAU),
            Mat3::rotation_y(TAU),
            Mat3::rotation_z(TAU),
            Mat3::rotation_axis(Vec3::new(-1.0, 2.0, 0.3), TAU),
        ] {
            assert!(approx(m * v, v, 1e-4));
        }
        // 360° como 36 pasos de 10°.
        let step = Mat3::rotation_y(TAU / 36.0);
        let mut w = v;
        for _ in 0..36 {
            w = step * w;
        }
        assert!(approx(w, v, 1e-4));
    }

    #[test]
    fn quarter_turns_match_axes() {
        assert!(approx(Mat3::rotation_z(FRAC_PI_2) * Vec3::X, Vec3::Y, 1e-6));
        assert!(approx(Mat3::rotation_x(FRAC_PI_2) * Vec3::Y, Vec3::Z, 1e-6));
        assert!(approx(Mat3::rotation_y(FRAC_PI_2) * Vec3::Z, Vec3::X, 1e-6));
        let r = Mat3::rotation_axis(Vec3::Y, 0.8);
        assert!(approx(r * Vec3::Z, Mat3::rotation_y(0.8) * Vec3::Z, 1e-6));
    }

    #[test]
    fn transpose_is_inverse_for_rotations() {
        let r = Mat3::rotation_axis(Vec3::new(0.3, -1.0, 2.0), 1.1);
        let v = Vec3::new(4.0, 5.0, 6.0);
        assert!(approx(r.transpose() * (r * v), v, 1e-4));
        assert!(approx((Mat3::IDENTITY * r) * v, r * v, 1e-6));
    }
}

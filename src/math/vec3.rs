//! Vector de 3 componentes en `f32`, usado para posiciones, direcciones y colores.

use std::ops::{Add, AddAssign, Div, DivAssign, Index, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);
    pub const ONE: Vec3 = Vec3::new(1.0, 1.0, 1.0);
    pub const X: Vec3 = Vec3::new(1.0, 0.0, 0.0);
    pub const Y: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    pub const Z: Vec3 = Vec3::new(0.0, 0.0, 1.0);

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    #[inline]
    pub const fn splat(v: f32) -> Vec3 {
        Vec3 { x: v, y: v, z: v }
    }

    /// Vector unitario sobre el eje `axis` (0 = X, 1 = Y, 2 = Z) con el signo dado.
    #[inline]
    pub fn axis(axis: usize, sign: f32) -> Vec3 {
        match axis {
            0 => Vec3::new(sign, 0.0, 0.0),
            1 => Vec3::new(0.0, sign, 0.0),
            _ => Vec3::new(0.0, 0.0, sign),
        }
    }

    #[inline]
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    #[inline]
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Devuelve el vector con longitud 1. El vector cero se devuelve tal cual.
    #[inline]
    pub fn normalized(self) -> Vec3 {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            self
        }
    }

    #[inline]
    pub fn min(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x.min(o.x), self.y.min(o.y), self.z.min(o.z))
    }

    #[inline]
    pub fn max(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x.max(o.x), self.y.max(o.y), self.z.max(o.z))
    }

    #[inline]
    pub fn abs(self) -> Vec3 {
        Vec3::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    #[inline]
    pub fn max_component(self) -> f32 {
        self.x.max(self.y).max(self.z)
    }

    #[inline]
    pub fn min_component(self) -> f32 {
        self.x.min(self.y).min(self.z)
    }

    /// Interpolación lineal: `self` cuando `t = 0`, `o` cuando `t = 1`.
    #[inline]
    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self + (o - self) * t
    }

    /// Aplica `f` a cada componente.
    #[inline]
    pub fn map(self, f: impl Fn(f32) -> f32) -> Vec3 {
        Vec3::new(f(self.x), f(self.y), f(self.z))
    }

    #[inline]
    pub fn clamp01(self) -> Vec3 {
        self.map(|c| c.clamp(0.0, 1.0))
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Luminancia relativa (Rec. 709) de un color lineal.
    #[inline]
    pub fn luminance(self) -> f32 {
        0.2126 * self.x + 0.7152 * self.y + 0.0722 * self.z
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl SubAssign for Vec3 {
    #[inline]
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;
    #[inline]
    fn mul(self, v: Vec3) -> Vec3 {
        v * self
    }
}

/// Producto componente a componente (útil para colores).
impl Mul<Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x * o.x, self.y * o.y, self.z * o.z)
    }
}

impl MulAssign<f32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, s: f32) {
        *self = *self * s;
    }
}

impl MulAssign<Vec3> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, o: Vec3) {
        *self = *self * o;
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, s: f32) -> Vec3 {
        let inv = 1.0 / s;
        Vec3::new(self.x * inv, self.y * inv, self.z * inv)
    }
}

impl DivAssign<f32> for Vec3 {
    #[inline]
    fn div_assign(&mut self, s: f32) {
        *self = *self / s;
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    #[inline]
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

impl Index<usize> for Vec3 {
    type Output = f32;
    #[inline]
    fn index(&self, i: usize) -> &f32 {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("índice de Vec3 fuera de rango: {i}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-6;

    fn approx(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn dot_product() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, -5.0, 6.0);
        assert!((a.dot(b) - 12.0).abs() < EPS);
        assert!((Vec3::X.dot(Vec3::Y)).abs() < EPS);
    }

    #[test]
    fn cross_product() {
        assert!(approx(Vec3::X.cross(Vec3::Y), Vec3::Z));
        assert!(approx(Vec3::Y.cross(Vec3::Z), Vec3::X));
        assert!(approx(Vec3::Z.cross(Vec3::X), Vec3::Y));
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(-2.0, 0.5, 4.0);
        let c = a.cross(b);
        // El producto cruz es perpendicular a ambos operandos.
        assert!(c.dot(a).abs() < 1e-4);
        assert!(c.dot(b).abs() < 1e-4);
        // Anticonmutativo.
        assert!(approx(c, -b.cross(a)));
    }

    #[test]
    fn normalization_has_unit_length() {
        for v in [
            Vec3::new(3.0, 4.0, 0.0),
            Vec3::new(-0.001, 0.002, 0.0005),
            Vec3::new(100.0, -250.0, 42.0),
        ] {
            assert!((v.normalized().length() - 1.0).abs() < 1e-5);
        }
        // El vector cero no produce NaN.
        assert_eq!(Vec3::ZERO.normalized(), Vec3::ZERO);
    }

    #[test]
    fn operators() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(0.5, -1.0, 2.0);
        assert_eq!(a + b, Vec3::new(1.5, 1.0, 5.0));
        assert_eq!(a - b, Vec3::new(0.5, 3.0, 1.0));
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(2.0 * a, a * 2.0);
        assert_eq!(a * b, Vec3::new(0.5, -2.0, 6.0));
        assert_eq!(a / 2.0, Vec3::new(0.5, 1.0, 1.5));
        assert_eq!(-a, Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(a[0], 1.0);
        assert_eq!(a[1], 2.0);
        assert_eq!(a[2], 3.0);
        let mut c = a;
        c += b;
        c -= b;
        c *= 3.0;
        c /= 3.0;
        assert!(approx(c, a));
        assert!(approx(a.lerp(b, 0.5), (a + b) * 0.5));
        assert_eq!(a.max_component(), 3.0);
        assert_eq!(b.min_component(), -1.0);
    }
}

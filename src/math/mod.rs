//! Álgebra lineal básica: vectores, rayos y matrices 3×3.

pub mod mat3;
pub mod ray;
pub mod vec3;

pub use mat3::Mat3;
pub use ray::Ray;
pub use vec3::Vec3;

/// Interpolación suave de Hermite entre `e0` y `e1`.
#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_limits_and_monotonic() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
        let mut prev = 0.0;
        for i in 0..=100 {
            let v = smoothstep(2.0, 5.0, 2.0 + 3.0 * i as f32 / 100.0);
            assert!(v >= prev);
            prev = v;
        }
    }

    #[test]
    fn lerp_endpoints() {
        assert_eq!(lerp(2.0, 6.0, 0.0), 2.0);
        assert_eq!(lerp(2.0, 6.0, 1.0), 6.0);
        assert_eq!(lerp(2.0, 6.0, 0.25), 3.0);
    }
}

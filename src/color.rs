//! Conversión de color: tone mapping (ACES filmic), gamma 2.2 y cuantización a `u8`.

use crate::math::Vec3;

pub const GAMMA: f32 = 2.2;

/// Aproximación de Narkowicz a la curva ACES filmic. Comprime HDR a [0, 1].
#[inline]
pub fn aces_film(x: f32) -> f32 {
    let x = x.max(0.0);
    let (a, b, c, d, e) = (2.51, 0.03, 2.43, 0.59, 0.14);
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
}

/// Tone mapping por componente con exposición.
#[inline]
pub fn tonemap(c: Vec3, exposure: f32) -> Vec3 {
    (c * exposure).map(aces_film)
}

/// Lineal → espacio de pantalla (gamma 2.2).
#[inline]
pub fn gamma_encode(x: f32) -> f32 {
    x.max(0.0).powf(1.0 / GAMMA)
}

/// Espacio de pantalla → lineal.
#[inline]
pub fn gamma_decode(x: f32) -> f32 {
    x.max(0.0).powf(GAMMA)
}

/// Cuantiza un valor en [0, 1] a un byte, con redondeo.
#[inline]
pub fn to_u8(x: f32) -> u8 {
    if x.is_nan() {
        return 0;
    }
    (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Color lineal HDR → RGB8 listo para escribir en un archivo.
#[inline]
pub fn linear_to_rgb8(c: Vec3, exposure: f32) -> [u8; 3] {
    let t = tonemap(c, exposure);
    [
        to_u8(gamma_encode(t.x)),
        to_u8(gamma_encode(t.y)),
        to_u8(gamma_encode(t.z)),
    ]
}

/// RGB8 en espacio de pantalla → color lineal en [0, 1].
#[inline]
pub fn rgb8_to_linear(rgb: [u8; 3]) -> Vec3 {
    Vec3::new(
        gamma_decode(rgb[0] as f32 / 255.0),
        gamma_decode(rgb[1] as f32 / 255.0),
        gamma_decode(rgb[2] as f32 / 255.0),
    )
}

/// RGB8 sin conversión de gamma → [0, 1] (para datos como normal maps).
#[inline]
pub fn rgb8_to_unit(rgb: [u8; 3]) -> Vec3 {
    Vec3::new(rgb[0] as f32, rgb[1] as f32, rgb[2] as f32) / 255.0
}

/// Color lineal en [0, 1] → RGB8 en espacio de pantalla, sin tone mapping.
#[inline]
pub fn linear_to_srgb8(c: Vec3) -> [u8; 3] {
    [
        to_u8(gamma_encode(c.x)),
        to_u8(gamma_encode(c.y)),
        to_u8(gamma_encode(c.z)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tonemap_is_monotonic_and_bounded() {
        let mut prev = -1.0;
        for i in 0..2000 {
            let x = i as f32 * 0.01;
            let y = aces_film(x);
            assert!((0.0..=1.0).contains(&y));
            assert!(y >= prev, "ACES no es monótono en {x}");
            prev = y;
        }
        assert_eq!(aces_film(0.0), 0.0);
        assert!(aces_film(100.0) > 0.99);
        assert_eq!(aces_film(-3.0), 0.0);
    }

    #[test]
    fn gamma_is_monotonic_and_inverse() {
        let mut prev = -1.0;
        for i in 0..=100 {
            let x = i as f32 / 100.0;
            let g = gamma_encode(x);
            assert!(g >= prev);
            prev = g;
            assert!((gamma_decode(g) - x).abs() < 1e-4);
        }
        assert_eq!(gamma_encode(1.0), 1.0);
    }

    #[test]
    fn u8_conversion_in_range() {
        assert_eq!(to_u8(-1.0), 0);
        assert_eq!(to_u8(0.0), 0);
        assert_eq!(to_u8(1.0), 255);
        assert_eq!(to_u8(7.0), 255);
        assert_eq!(to_u8(f32::NAN), 0);
        let mut prev = 0u8;
        for i in 0..=1000 {
            let v = to_u8(i as f32 / 1000.0);
            assert!(v >= prev);
            prev = v;
        }
        // Colores HDR extremos siempre quedan en [0, 255] y en orden.
        let dark = linear_to_rgb8(Vec3::splat(0.05), 1.0);
        let bright = linear_to_rgb8(Vec3::splat(50.0), 1.0);
        assert!(dark[0] < bright[0]);
    }

    #[test]
    fn srgb_roundtrip() {
        for v in [0u8, 1, 17, 128, 200, 255] {
            let lin = rgb8_to_linear([v, v, v]);
            assert_eq!(linear_to_srgb8(lin), [v, v, v]);
        }
    }
}

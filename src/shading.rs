//! Funciones de sombreado: Blinn-Phong, reflexión, refracción (Snell) y Fresnel de Schlick.
//!
//! Convención: `n` es la normal orientada contra el rayo incidente (`dot(d, n) < 0`).

use crate::math::{Mat3, Vec3};

/// Reflexión especular: `r = d - 2 (d·n) n`.
#[inline]
pub fn reflect(d: Vec3, n: Vec3) -> Vec3 {
    d - n * (2.0 * d.dot(n))
}

/// Refracción por la ley de Snell con `eta = n1 / n2`.
/// Devuelve `None` si hay reflexión interna total.
#[inline]
pub fn refract(d: Vec3, n: Vec3, eta: f32) -> Option<Vec3> {
    let cos_i = (-d.dot(n)).clamp(-1.0, 1.0);
    let sin2_t = eta * eta * (1.0 - cos_i * cos_i).max(0.0);
    if sin2_t > 1.0 {
        return None;
    }
    let cos_t = (1.0 - sin2_t).sqrt();
    Some((d * eta + n * (eta * cos_i - cos_t)).normalized())
}

/// Reflectancia de Fresnel por la aproximación de Schlick, para un rayo que va del medio
/// `n1` al medio `n2` con `cos_i` = coseno del ángulo de incidencia.
/// Si el rayo sale a un medio menos denso se usa el ángulo transmitido, y la reflexión
/// interna total devuelve 1.
#[inline]
pub fn fresnel_schlick(cos_i: f32, n1: f32, n2: f32) -> f32 {
    let mut cos = cos_i.clamp(0.0, 1.0);
    if n1 > n2 {
        let eta = n1 / n2;
        let sin2_t = eta * eta * (1.0 - cos * cos);
        if sin2_t > 1.0 {
            return 1.0;
        }
        cos = (1.0 - sin2_t).sqrt();
    }
    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    let m = 1.0 - cos;
    let m2 = m * m;
    (r0 + (1.0 - r0) * m2 * m2 * m).clamp(0.0, 1.0)
}

/// Término difuso de Lambert y especular de Blinn-Phong: `(n·l, (n·h)^shininess)`.
#[inline]
pub fn blinn_phong(n: Vec3, l: Vec3, view: Vec3, shininess: f32) -> (f32, f32) {
    let ndl = n.dot(l);
    if ndl <= 0.0 {
        return (0.0, 0.0);
    }
    let h = (l + view).normalized();
    (ndl, n.dot(h).max(0.0).powf(shininess))
}

/// Marco tangente fijo (T, B, N) de la cara de un cubo con normal `n` (un eje ±X, ±Y, ±Z).
/// `T` es la dirección en que crece `u`; `B` es la dirección en que *decrece* `v` (arriba en
/// la imagen), igual que en `dda::face_uv`. Se devuelve como matriz de columnas (T, B, N).
#[inline]
pub fn tangent_frame(n: Vec3) -> Mat3 {
    let (t, b) = if n.y > 0.5 {
        (Vec3::X, -Vec3::Z)
    } else if n.y < -0.5 {
        (Vec3::X, Vec3::Z)
    } else if n.x > 0.5 {
        (-Vec3::Z, Vec3::Y)
    } else if n.x < -0.5 {
        (Vec3::Z, Vec3::Y)
    } else if n.z > 0.5 {
        (Vec3::X, Vec3::Y)
    } else {
        (-Vec3::X, Vec3::Y)
    };
    Mat3::from_cols(t, b, n)
}

/// Normal del normal map (texel en [0, 1]) llevada a espacio de mundo sobre la cara `n`.
#[inline]
pub fn perturb_normal(texel: Vec3, n: Vec3) -> Vec3 {
    let ts = (texel * 2.0 - Vec3::ONE).normalized();
    (tangent_frame(n) * ts).normalized()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::dda::face_uv;

    fn sin_between(a: Vec3, b: Vec3) -> f32 {
        a.cross(b).length()
    }

    #[test]
    fn reflection_is_symmetric() {
        let n = Vec3::Y;
        let d = Vec3::new(1.0, -1.0, 0.5).normalized();
        let r = reflect(d, n);
        // Mismo ángulo con la normal, del otro lado, y conserva la longitud.
        assert!(((-d).dot(n) - r.dot(n)).abs() < 1e-6);
        assert!((r.length() - 1.0).abs() < 1e-6);
        assert!((r - Vec3::new(d.x, -d.y, d.z)).length() < 1e-6);
        // Reflejar dos veces devuelve el vector original.
        assert!((reflect(r, n) - d).length() < 1e-6);
    }

    #[test]
    fn refraction_obeys_snell() {
        let n = Vec3::Y;
        for (n1, n2) in [(1.0, 1.33), (1.0, 1.5), (1.33, 1.0), (1.5, 1.33)] {
            for deg in [0.0f32, 10.0, 25.0, 40.0] {
                let th = deg.to_radians();
                let d = Vec3::new(th.sin(), -th.cos(), 0.0);
                let t = refract(d, n, n1 / n2).unwrap();
                let lhs = n1 * sin_between(d, -n);
                let rhs = n2 * sin_between(t, -n);
                assert!((lhs - rhs).abs() < 1e-4, "{n1}->{n2} a {deg}°");
                assert!(t.dot(n) < 0.0, "el rayo refractado cruza la superficie");
            }
        }
        // Incidencia normal: no se desvía.
        let t = refract(-Vec3::Y, Vec3::Y, 1.0 / 1.5).unwrap();
        assert!((t + Vec3::Y).length() < 1e-6);
    }

    #[test]
    fn total_internal_reflection_is_detected() {
        let n = Vec3::Y;
        // Del agua al aire, el ángulo crítico es ~48.75°.
        let crit = (1.0f32 / 1.33).asin().to_degrees();
        let below = (crit - 2.0).to_radians();
        let above = (crit + 2.0).to_radians();
        let d_below = Vec3::new(below.sin(), -below.cos(), 0.0);
        let d_above = Vec3::new(above.sin(), -above.cos(), 0.0);
        assert!(refract(d_below, n, 1.33).is_some());
        assert!(refract(d_above, n, 1.33).is_none());
        assert_eq!(fresnel_schlick(above.cos(), 1.33, 1.0), 1.0);
        // Del aire al agua nunca hay reflexión interna total.
        assert!(refract(d_above, n, 1.0 / 1.33).is_some());
    }

    #[test]
    fn fresnel_in_range_and_one_at_grazing() {
        for (n1, n2) in [(1.0, 1.33), (1.0, 1.5), (1.33, 1.0), (1.5, 1.0)] {
            let mut prev = 0.0;
            for i in 0..=90 {
                // De incidencia normal (cos = 1) a rasante (cos = 0).
                let cos = (90 - i) as f32 / 90.0;
                let f = fresnel_schlick(cos, n1, n2);
                assert!((0.0..=1.0).contains(&f));
                assert!(f >= prev - 1e-6, "Fresnel no es monótono");
                prev = f;
            }
            // A 90° (cos = 0) refleja todo.
            assert!((fresnel_schlick(0.0, n1, n2) - 1.0).abs() < 1e-6);
        }
        // Incidencia normal aire→vidrio: ~4 %.
        assert!((fresnel_schlick(1.0, 1.0, 1.5) - 0.04).abs() < 1e-3);
        // Monótona: más rasante, más reflexión.
        assert!(fresnel_schlick(0.2, 1.0, 1.33) > fresnel_schlick(0.8, 1.0, 1.33));
    }

    const FACES: [Vec3; 6] = [
        Vec3::X,
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::Y,
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::Z,
        Vec3::new(0.0, 0.0, -1.0),
    ];

    #[test]
    fn flat_normal_map_on_top_face_is_up() {
        // Texel (0.5, 0.5, 1) = normal (0, 0, 1) en espacio tangente.
        let flat = Vec3::new(0.5, 0.5, 1.0);
        let n = perturb_normal(flat, Vec3::Y);
        assert!((n - Vec3::Y).length() < 1e-6);
        // En cualquier cara, la normal plana es la normal de la cara.
        for f in FACES {
            assert!((perturb_normal(flat, f) - f).length() < 1e-6);
        }
    }

    #[test]
    fn tangent_frames_are_orthonormal_and_match_uv() {
        for f in FACES {
            let m = tangent_frame(f);
            let [t, b, n] = m.cols;
            assert_eq!(n, f);
            assert!(t.dot(b).abs() < 1e-6 && t.dot(n).abs() < 1e-6);
            // Base derecha: T × B = N.
            assert!((t.cross(b) - n).length() < 1e-6);
            // Moverse sobre la cara a lo largo de T aumenta u; a lo largo de B disminuye v.
            let cell = [3, 3, 3];
            let center = Vec3::splat(3.5) + f * 0.5;
            let (u0, v0) = face_uv(center, cell, f);
            let (u1, _) = face_uv(center + t * 0.2, cell, f);
            let (_, v1) = face_uv(center + b * 0.2, cell, f);
            assert!(u1 > u0, "cara {f:?}");
            assert!(v1 < v0, "cara {f:?}");
        }
    }

    #[test]
    fn blinn_phong_terms() {
        let (d, s) = blinn_phong(Vec3::Y, Vec3::Y, Vec3::Y, 32.0);
        assert!((d - 1.0).abs() < 1e-6 && (s - 1.0).abs() < 1e-6);
        assert_eq!(blinn_phong(Vec3::Y, -Vec3::Y, Vec3::Y, 32.0), (0.0, 0.0));
        let (_, s_sharp) = blinn_phong(
            Vec3::Y,
            Vec3::new(0.3, 1.0, 0.0).normalized(),
            Vec3::Y,
            128.0,
        );
        let (_, s_soft) = blinn_phong(Vec3::Y, Vec3::new(0.3, 1.0, 0.0).normalized(), Vec3::Y, 8.0);
        assert!(s_sharp < s_soft);
    }
}

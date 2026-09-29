//! Skybox de atardecer en cubemap de 6 caras.
//!
//! Las caras se generan por código al iniciar (gradiente naranja-rosa-violeta, disco solar
//! y nubes hechas con fBm) y se exportan a `assets/skybox/<cara>.bmp`. Si esos BMP existen, se cargan en su lugar.

use crate::color;
use crate::image_io::Image;
use crate::math::{smoothstep, Vec3};
use crate::terrain::noise::Perlin;
use crate::texture::{ColorSpace, Texture};
use crate::texture_gen::{self, AssetSource};
use std::io;

pub const POS_X: usize = 0;
pub const NEG_X: usize = 1;
pub const POS_Y: usize = 2;
pub const NEG_Y: usize = 3;
pub const POS_Z: usize = 4;
pub const NEG_Z: usize = 5;

/// Nombre de archivo de cada cara (convención de cubemap de OpenGL).
pub const FACE_NAMES: [&str; 6] = ["px", "nx", "py", "ny", "pz", "nz"];

/// Dirección → (cara, u, v) con `u, v` en [0, 1]; `v = 0` es la fila de arriba de la cara.
#[inline]
pub fn direction_to_face_uv(d: Vec3) -> (usize, f32, f32) {
    let a = d.abs();
    let (face, ma, sc, tc) = if a.x >= a.y && a.x >= a.z {
        if d.x > 0.0 {
            (POS_X, a.x, -d.z, -d.y)
        } else {
            (NEG_X, a.x, d.z, -d.y)
        }
    } else if a.y >= a.z {
        if d.y > 0.0 {
            (POS_Y, a.y, d.x, d.z)
        } else {
            (NEG_Y, a.y, d.x, -d.z)
        }
    } else if d.z > 0.0 {
        (POS_Z, a.z, d.x, -d.y)
    } else {
        (NEG_Z, a.z, -d.x, -d.y)
    };
    let inv = if ma > 0.0 { 0.5 / ma } else { 0.0 };
    (
        face,
        (sc * inv + 0.5).clamp(0.0, 1.0),
        (tc * inv + 0.5).clamp(0.0, 1.0),
    )
}

/// Inversa de `direction_to_face_uv` (dirección normalizada).
pub fn face_uv_to_direction(face: usize, u: f32, v: f32) -> Vec3 {
    let sc = 2.0 * u - 1.0;
    let tc = 2.0 * v - 1.0;
    let d = match face {
        POS_X => Vec3::new(1.0, -tc, -sc),
        NEG_X => Vec3::new(-1.0, -tc, sc),
        POS_Y => Vec3::new(sc, 1.0, tc),
        NEG_Y => Vec3::new(sc, -1.0, -tc),
        POS_Z => Vec3::new(sc, -tc, 1.0),
        _ => Vec3::new(-sc, -tc, -1.0),
    };
    d.normalized()
}

/// Multiplicador HDR del cielo al muestrearlo.
pub const SKY_INTENSITY: f32 = 1.0;

/// Semilla del ruido de las nubes.
const CLOUD_SEED: u64 = 77;

/// Color lineal (en [0, 1]) del cielo de atardecer en la dirección `d`.
pub fn sunset_color(d: Vec3, sun_dir: Vec3, clouds: &Perlin) -> Vec3 {
    let horizon = Vec3::new(1.0, 0.46, 0.18);
    let low = Vec3::new(0.98, 0.36, 0.30);
    let mid = Vec3::new(0.60, 0.26, 0.48);
    let zenith = Vec3::new(0.13, 0.10, 0.33);
    let y = d.y;
    let mut c = if y >= 0.0 {
        let c = horizon.lerp(low, smoothstep(0.0, 0.14, y));
        let c = c.lerp(mid, smoothstep(0.08, 0.42, y));
        c.lerp(zenith, smoothstep(0.35, 1.0, y))
    } else {
        // Bajo el horizonte: un mar de nubes. Las crestas se iluminan de rosa-naranja y los
        // valles quedan violeta; más abajo todo se oscurece.
        let t = -y;
        let k = 1.0 / (t + 0.06);
        let n = clouds.fbm2(d.x * k * 0.55 + 40.0, d.z * k * 0.55 - 13.0, 5, 2.0, 0.5);
        let puff = smoothstep(-0.3, 0.35, n);
        let flat = Vec3::new(d.x, 0.0, d.z).normalized();
        let sun_flat = Vec3::new(sun_dir.x, 0.0, sun_dir.z).normalized();
        let lit = 0.5 + 0.5 * flat.dot(sun_flat);
        let crest = Vec3::new(0.95, 0.52, 0.42) * (0.55 + 0.45 * lit);
        let valley = Vec3::new(0.30, 0.17, 0.32);
        let sea = valley.lerp(crest, puff) * (1.0 - 0.55 * smoothstep(0.1, 0.9, t));
        let haze = Vec3::new(0.70, 0.40, 0.40);
        horizon
            .lerp(haze, smoothstep(0.0, 0.03, t))
            .lerp(sea, smoothstep(0.01, 0.14, t))
    };

    // El horizonte brilla más hacia el lado del sol.
    let flat = Vec3::new(d.x, 0.0, d.z).normalized();
    let sun_flat = Vec3::new(sun_dir.x, 0.0, sun_dir.z).normalized();
    let toward_sun = flat.dot(sun_flat).max(0.0).powi(3);
    let near_horizon = (1.0 - y.abs()).max(0.0).powi(6);
    c += Vec3::new(0.5, 0.22, 0.06) * (toward_sun * near_horizon);

    // Nubes: fBm proyectado sobre un plano alto, alargado en una dirección y más
    // denso a media altura. Se iluminan de naranja hacia el sol y de violeta al otro lado.
    if d.y > 0.0 {
        let k = 1.0 / (d.y + 0.1);
        let n = clouds.fbm2(d.x * k * 0.8, d.z * k * 2.2, 5, 2.0, 0.5);
        let density = smoothstep(0.0, 0.4, n)
            * smoothstep(0.02, 0.2, d.y)
            * (1.0 - smoothstep(0.55, 0.9, d.y));
        let lit = d.dot(sun_dir).max(0.0).powi(2);
        let shade = Vec3::new(0.46, 0.26, 0.42);
        let bright = Vec3::new(1.0, 0.58, 0.40);
        c = c.lerp(shade.lerp(bright, 0.25 + 0.75 * lit), density * 0.85);
    }

    // Halo y disco solar.
    let cos = d.dot(sun_dir);
    let glow = cos.max(0.0);
    c += Vec3::new(1.0, 0.55, 0.25) * (0.35 * glow.powi(12) + 0.5 * glow.powi(120));
    if cos > 0.9993 {
        c = Vec3::new(1.0, 0.95, 0.8);
    }
    c.clamp01()
}

/// Genera la imagen de una cara del cubemap.
pub fn generate_face(face: usize, size: usize, sun_dir: Vec3) -> Image {
    let clouds = Perlin::new(CLOUD_SEED);
    Image::from_fn(size, size, |x, y| {
        let u = (x as f32 + 0.5) / size as f32;
        let v = (y as f32 + 0.5) / size as f32;
        let d = face_uv_to_direction(face, u, v);
        color::linear_to_srgb8(sunset_color(d, sun_dir, &clouds))
    })
}

#[derive(Clone, Debug)]
pub struct Skybox {
    pub faces: Vec<Texture>,
    /// Multiplicador HDR aplicado al muestrear.
    pub intensity: f32,
    /// Color promedio (ya multiplicado por `intensity`), base de la luz ambiental.
    pub average: Vec3,
}

impl Skybox {
    pub fn from_images(images: &[Image], intensity: f32) -> Skybox {
        let faces: Vec<Texture> = images
            .iter()
            .map(|img| Texture::from_image(img, ColorSpace::Srgb))
            .collect();
        // Promedio de las cinco caras visibles (la de abajo casi no se ve).
        let mut sum = Vec3::ZERO;
        for (i, f) in faces.iter().enumerate() {
            if i != NEG_Y {
                sum += f.average();
            }
        }
        Skybox {
            faces,
            intensity,
            average: sum / 5.0 * intensity,
        }
    }

    /// Genera o carga las 6 caras según `source`.
    pub fn load(source: &AssetSource, size: usize, sun_dir: Vec3) -> io::Result<Skybox> {
        let mut images = Vec::with_capacity(6);
        for (face, name) in FACE_NAMES.iter().enumerate() {
            let img = match source {
                AssetSource::Generated => generate_face(face, size, sun_dir),
                AssetSource::Directory(root) => {
                    texture_gen::load_or_create(&root.join("skybox"), name, || {
                        generate_face(face, size, sun_dir)
                    })?
                    .0
                }
            };
            images.push(img);
        }
        Ok(Skybox::from_images(&images, SKY_INTENSITY))
    }

    /// Color HDR del cielo en la dirección `dir`.
    #[inline]
    pub fn sample(&self, dir: Vec3) -> Vec3 {
        let (face, u, v) = direction_to_face_uv(dir);
        self.faces[face].sample_bilinear_clamp(u, v) * self.intensity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_directions_map_to_the_right_face() {
        assert_eq!(direction_to_face_uv(Vec3::X).0, POS_X);
        assert_eq!(direction_to_face_uv(-Vec3::X).0, NEG_X);
        assert_eq!(direction_to_face_uv(Vec3::Y).0, POS_Y);
        assert_eq!(direction_to_face_uv(-Vec3::Y).0, NEG_Y);
        assert_eq!(direction_to_face_uv(Vec3::Z).0, POS_Z);
        assert_eq!(direction_to_face_uv(-Vec3::Z).0, NEG_Z);
        // El centro de cada cara es (0.5, 0.5).
        for d in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
            let (_, u, v) = direction_to_face_uv(d);
            assert!((u - 0.5).abs() < 1e-6 && (v - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn uv_in_unit_range_and_inverse() {
        let mut s: u32 = 12345;
        for _ in 0..2000 {
            let mut r = || {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                s as f32 / u32::MAX as f32 * 2.0 - 1.0
            };
            let d = Vec3::new(r(), r(), r()).normalized();
            if d.length() < 0.5 {
                continue;
            }
            let (face, u, v) = direction_to_face_uv(d);
            assert!((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v));
            let back = face_uv_to_direction(face, u, v);
            assert!((back - d).length() < 1e-4);
        }
    }

    #[test]
    fn side_faces_have_sky_on_top() {
        // En las caras laterales, v = 0 mira hacia arriba.
        for face in [POS_X, NEG_X, POS_Z, NEG_Z] {
            assert!(face_uv_to_direction(face, 0.5, 0.0).y > 0.5);
            assert!(face_uv_to_direction(face, 0.5, 1.0).y < -0.5);
        }
    }

    #[test]
    fn generated_sky_is_a_sunset() {
        let sun = Vec3::new(-0.6, 0.25, -0.75).normalized();
        let sky = Skybox::load(&AssetSource::Generated, 32, sun).unwrap();
        let horizon = sky.sample(Vec3::new(1.0, 0.02, 0.0).normalized());
        let zenith = sky.sample(Vec3::Y);
        // Horizonte cálido (más rojo que azul), cenit más frío y oscuro.
        assert!(horizon.x > horizon.z);
        assert!(zenith.z > zenith.x * 0.8);
        assert!(horizon.luminance() > zenith.luminance());
        // Hacia el sol es lo más brillante.
        assert!(sky.sample(sun).luminance() > horizon.luminance());
        assert!(sky.average.is_finite() && sky.average.max_component() > 0.0);
    }
}

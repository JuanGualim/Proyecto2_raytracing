//! Texturas en memoria (colores lineales en `f32`) con muestreo nearest y wrap.

use crate::color;
use crate::image_io::Image;
use crate::math::Vec3;

pub type TextureId = usize;

/// Cómo interpretar los bytes de una imagen al convertirla en textura.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    /// Colores en espacio de pantalla: se convierten a lineal (gamma 2.2).
    Srgb,
    /// Datos (normal maps, alturas): solo se dividen entre 255.
    Linear,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    pub width: usize,
    pub height: usize,
    /// Texels por filas; la fila 0 es la de arriba.
    pub texels: Vec<Vec3>,
}

/// Parte fraccionaria en [0, 1), también para valores negativos (wrap / repeat).
#[inline]
pub fn wrap01(x: f32) -> f32 {
    let f = x - x.floor();
    // `x - floor(x)` puede redondear a 1.0 con valores negativos muy pequeños.
    if f >= 1.0 {
        0.0
    } else {
        f
    }
}

impl Texture {
    pub fn from_image(img: &Image, space: ColorSpace) -> Texture {
        let texels = img
            .pixels
            .iter()
            .map(|&p| match space {
                ColorSpace::Srgb => color::rgb8_to_linear(p),
                ColorSpace::Linear => color::rgb8_to_unit(p),
            })
            .collect();
        Texture {
            width: img.width,
            height: img.height,
            texels,
        }
    }

    pub fn solid(c: Vec3) -> Texture {
        Texture {
            width: 1,
            height: 1,
            texels: vec![c],
        }
    }

    #[inline]
    pub fn texel(&self, x: usize, y: usize) -> Vec3 {
        self.texels[y * self.width + x]
    }

    /// Muestreo nearest con wrap. `(0, 0)` es la esquina superior izquierda.
    #[inline]
    pub fn sample_nearest(&self, u: f32, v: f32) -> Vec3 {
        let x = ((wrap01(u) * self.width as f32) as usize).min(self.width - 1);
        let y = ((wrap01(v) * self.height as f32) as usize).min(self.height - 1);
        self.texel(x, y)
    }

    /// Muestreo bilineal con los bordes fijados (clamp). Se usa en las caras del skybox.
    pub fn sample_bilinear_clamp(&self, u: f32, v: f32) -> Vec3 {
        let fx = (u.clamp(0.0, 1.0) * self.width as f32 - 0.5).max(0.0);
        let fy = (v.clamp(0.0, 1.0) * self.height as f32 - 0.5).max(0.0);
        let x0 = (fx as usize).min(self.width - 1);
        let y0 = (fy as usize).min(self.height - 1);
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let top = self.texel(x0, y0).lerp(self.texel(x1, y0), tx);
        let bottom = self.texel(x0, y1).lerp(self.texel(x1, y1), tx);
        top.lerp(bottom, ty)
    }

    /// Color promedio de la textura.
    pub fn average(&self) -> Vec3 {
        let sum = self.texels.iter().fold(Vec3::ZERO, |acc, &t| acc + t);
        sum / self.texels.len().max(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> Texture {
        // 2×2: rojo, verde / azul, blanco.
        Texture {
            width: 2,
            height: 2,
            texels: vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE],
        }
    }

    #[test]
    fn uv_wrap() {
        assert!((wrap01(-0.25) - 0.75).abs() < 1e-6);
        assert!((wrap01(1.25) - 0.25).abs() < 1e-6);
        assert!((wrap01(3.0)).abs() < 1e-6);
        assert!((wrap01(-2.5) - 0.5).abs() < 1e-6);
        let w = wrap01(-1e-9);
        assert!((0.0..1.0).contains(&w));
    }

    #[test]
    fn nearest_returns_expected_texel() {
        let t = checker();
        assert_eq!(t.sample_nearest(0.1, 0.1), Vec3::X);
        assert_eq!(t.sample_nearest(0.9, 0.1), Vec3::Y);
        assert_eq!(t.sample_nearest(0.1, 0.9), Vec3::Z);
        assert_eq!(t.sample_nearest(0.9, 0.9), Vec3::ONE);
        // Con wrap: valores negativos y mayores a 1.
        assert_eq!(t.sample_nearest(-0.9, 0.1), Vec3::X);
        assert_eq!(t.sample_nearest(1.6, 2.7), Vec3::ONE);
        assert_eq!(t.sample_nearest(-0.1, -0.1), Vec3::ONE);
        assert_eq!(t.sample_nearest(1.0, 0.0), Vec3::X);
    }

    #[test]
    fn from_image_color_spaces() {
        let img = Image::from_fn(1, 1, |_, _| [255, 128, 0]);
        let lin = Texture::from_image(&img, ColorSpace::Linear);
        assert!((lin.texels[0].y - 128.0 / 255.0).abs() < 1e-6);
        let srgb = Texture::from_image(&img, ColorSpace::Srgb);
        assert!(srgb.texels[0].y < lin.texels[0].y);
        assert_eq!(srgb.texels[0].x, 1.0);
    }

    #[test]
    fn bilinear_and_average() {
        let t = checker();
        let c = t.sample_bilinear_clamp(0.5, 0.5);
        assert!((c - t.average()).length() < 1e-5);
        assert_eq!(t.sample_bilinear_clamp(0.0, 0.0), Vec3::X);
        assert_eq!(Texture::solid(Vec3::Y).sample_nearest(0.3, 0.7), Vec3::Y);
    }
}

//! Imágenes RGB de 8 bits y sus formatos de archivo (BMP de 24 bits y PPM P6),
//! implementados a mano sin dependencias externas.

pub mod bmp;
pub mod ppm;

use std::io;
use std::path::Path;

/// Imagen RGB8. La fila 0 es la de arriba.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 3]>,
}

impl Image {
    pub fn new(width: usize, height: usize) -> Image {
        Image {
            width,
            height,
            pixels: vec![[0, 0, 0]; width * height],
        }
    }

    pub fn from_fn(width: usize, height: usize, f: impl Fn(usize, usize) -> [u8; 3]) -> Image {
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                pixels.push(f(x, y));
            }
        }
        Image {
            width,
            height,
            pixels,
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> [u8; 3] {
        self.pixels[y * self.width + x]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        self.pixels[y * self.width + x] = rgb;
    }
}

/// Guarda la imagen eligiendo el formato por la extensión (`.bmp` o `.ppm`).
pub fn save(path: &Path, img: &Image) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("ppm") => ppm::write_ppm(path, img),
        _ => bmp::write_bmp(path, img),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set_and_from_fn() {
        let mut img = Image::from_fn(3, 2, |x, y| [x as u8, y as u8, 7]);
        assert_eq!(img.get(2, 1), [2, 1, 7]);
        img.set(0, 1, [9, 9, 9]);
        assert_eq!(img.get(0, 1), [9, 9, 9]);
        assert_eq!(img.pixels.len(), 6);
    }
}

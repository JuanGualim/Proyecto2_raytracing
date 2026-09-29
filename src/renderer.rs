//! Renderizador. Fase 2: cubos de colores recorridos con DDA y vistos con la cámara orbital.

use crate::camera::Camera;
use crate::color;
use crate::image_io::Image;
use crate::math::Vec3;
use crate::world::{dda, VoxelGrid};

/// Color plano provisional por id de material.
fn palette(id: u8) -> Vec3 {
    match id {
        1 => Vec3::new(0.35, 0.7, 0.25),
        2 => Vec3::new(0.55, 0.55, 0.58),
        3 => Vec3::new(0.55, 0.35, 0.18),
        4 => Vec3::new(0.2, 0.4, 0.9),
        5 => Vec3::new(0.9, 0.9, 0.95),
        6 => Vec3::new(1.0, 0.8, 0.2),
        _ => Vec3::new(1.0, 0.3, 0.8),
    }
}

/// Unos cuantos cubos de colores sobre un piso, para probar el DDA.
pub fn demo_grid() -> VoxelGrid {
    let mut g = VoxelGrid::new(12, 8, 12);
    g.fill_box([0, 0, 0], [11, 0, 11], 1);
    g.fill_box([2, 1, 2], [3, 3, 3], 2);
    g.fill_box([7, 1, 2], [8, 1, 4], 3);
    g.fill_box([5, 1, 7], [9, 1, 9], 4);
    g.fill_box([2, 1, 8], [2, 4, 8], 6);
    g.set(6, 1, 5, 5);
    g.set(6, 2, 5, 7);
    g
}

/// Render por DDA con color plano y una luz direccional sin sombras.
pub fn render_grid(grid: &VoxelGrid, camera: &Camera, width: usize, height: usize) -> Image {
    let frame = camera.frame(width, height);
    let light = Vec3::new(0.5, 1.0, 0.3).normalized();
    Image::from_fn(width, height, |x, y| {
        let ray = frame.ray(x as f32 + 0.5, y as f32 + 0.5);
        let c = match dda::trace(grid, &ray, 0.0, f32::INFINITY) {
            Some(h) => palette(h.material) * (0.25 + 0.75 * h.normal.dot(light).max(0.0)),
            None => Vec3::new(0.55, 0.7, 0.95),
        };
        color::linear_to_srgb8(c)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_grid_is_visible() {
        let g = demo_grid();
        let cam = Camera::new(Vec3::new(6.0, 1.0, 6.0), 0.7, 0.6, 22.0);
        let img = render_grid(&g, &cam, 48, 27);
        let sky = color::linear_to_srgb8(Vec3::new(0.55, 0.7, 0.95));
        let covered = img.pixels.iter().filter(|&&p| p != sky).count();
        assert!(covered > img.pixels.len() / 8);
    }
}

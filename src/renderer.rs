//! Renderizador. Fase 1: un cubo de color plano intersectado con una AABB.

use crate::color;
use crate::image_io::Image;
use crate::math::{Ray, Vec3};
use crate::world::Aabb;

/// Renderiza un cubo con sombreado Lambert simple sobre un gradiente de cielo.
pub fn render_flat_cube(width: usize, height: usize) -> Image {
    let cube = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
    let eye = Vec3::new(3.0, 2.5, 4.0);
    let forward = (cube.center() - eye).normalized();
    let right = forward.cross(Vec3::Y).normalized();
    let up = right.cross(forward);
    let half_h = (40f32.to_radians() * 0.5).tan();
    let half_w = half_h * width as f32 / height as f32;
    let light = Vec3::new(0.6, 1.0, 0.3).normalized();
    let base = Vec3::new(0.9, 0.35, 0.2);

    Image::from_fn(width, height, |x, y| {
        let sx = (2.0 * (x as f32 + 0.5) / width as f32 - 1.0) * half_w;
        let sy = (1.0 - 2.0 * (y as f32 + 0.5) / height as f32) * half_h;
        let ray = Ray::new(eye, (forward + right * sx + up * sy).normalized());
        let c = match cube.hit_with_normal(&ray, 0.0, f32::INFINITY) {
            Some((_, n)) => base * (0.15 + 0.85 * n.dot(light).max(0.0)),
            None => Vec3::new(0.5, 0.7, 1.0).lerp(Vec3::ONE, 1.0 - ray.dir.y.max(0.0)),
        };
        color::linear_to_srgb8(c.clamp01())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_cube_is_visible_in_the_center() {
        let img = render_flat_cube(40, 30);
        let center = img.get(20, 15);
        let corner = img.get(0, 0);
        // En el centro se ve el cubo (rojizo); en la esquina, el cielo (azulado).
        assert!(center[0] > center[2]);
        assert!(corner[2] >= corner[0]);
        assert_ne!(center, corner);
    }
}

//! Cámara orbital: mira siempre a `target` desde una posición definida por
//! `yaw`, `pitch` y `distance` (limitada a `[min_distance, max_distance]`).

use crate::math::{Ray, Vec3};

const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub target: Vec3,
    /// Ángulo horizontal en radianes (0 = la cámara está en +Z respecto al objetivo).
    pub yaw: f32,
    /// Elevación en radianes; positivo = la cámara está arriba y mira hacia abajo.
    pub pitch: f32,
    distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    /// Campo de visión vertical en radianes.
    pub fov_y: f32,
}

impl Camera {
    pub fn new(target: Vec3, yaw: f32, pitch: f32, distance: f32) -> Camera {
        let mut cam = Camera {
            target,
            yaw,
            pitch: 0.0,
            distance,
            min_distance: 4.0,
            max_distance: 120.0,
            fov_y: 45f32.to_radians(),
        };
        cam.set_pitch(pitch);
        cam.set_distance(distance);
        cam
    }

    /// Cambia los límites del zoom y vuelve a aplicar el clamp.
    pub fn with_limits(mut self, min_distance: f32, max_distance: f32) -> Camera {
        self.min_distance = min_distance;
        self.max_distance = max_distance.max(min_distance);
        let d = self.distance;
        self.set_distance(d);
        self
    }

    pub fn with_fov_degrees(mut self, fov: f32) -> Camera {
        self.fov_y = fov.to_radians();
        self
    }

    #[inline]
    pub fn distance(&self) -> f32 {
        self.distance
    }

    pub fn set_distance(&mut self, d: f32) {
        self.distance = d.clamp(self.min_distance, self.max_distance);
    }

    /// Zoom multiplicativo: `factor < 1` acerca, `factor > 1` aleja.
    pub fn zoom(&mut self, factor: f32) {
        self.set_distance(self.distance * factor);
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch.clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Gira la cámara alrededor del objetivo.
    pub fn orbit(&mut self, d_yaw: f32, d_pitch: f32) {
        self.yaw += d_yaw;
        self.set_pitch(self.pitch + d_pitch);
    }

    pub fn position(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.target + Vec3::new(cp * sy, sp, cp * cy) * self.distance
    }

    /// Precalcula la base de la cámara para generar rayos en una imagen de `width × height`.
    pub fn frame(&self, width: usize, height: usize) -> CameraFrame {
        let origin = self.position();
        let forward = (self.target - origin).normalized();
        let right = forward.cross(Vec3::Y).normalized();
        let up = right.cross(forward);
        let half_h = (self.fov_y * 0.5).tan();
        let half_w = half_h * width as f32 / height as f32;
        CameraFrame {
            origin,
            forward,
            right: right * half_w,
            up: up * half_h,
            inv_w: 1.0 / width as f32,
            inv_h: 1.0 / height as f32,
        }
    }
}

/// Base precalculada de la cámara para un tamaño de imagen.
#[derive(Clone, Copy, Debug)]
pub struct CameraFrame {
    pub origin: Vec3,
    pub forward: Vec3,
    right: Vec3,
    up: Vec3,
    inv_w: f32,
    inv_h: f32,
}

impl CameraFrame {
    /// Rayo por la posición de imagen `(px, py)` en píxeles; `(0, 0)` es la esquina
    /// superior izquierda y el centro del píxel `(x, y)` es `(x + 0.5, y + 0.5)`.
    #[inline]
    pub fn ray(&self, px: f32, py: f32) -> Ray {
        let sx = 2.0 * px * self.inv_w - 1.0;
        let sy = 1.0 - 2.0 * py * self.inv_h;
        let dir = self.forward + self.right * sx + self.up * sy;
        Ray::new(self.origin, dir.normalized())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_is_clamped() {
        let mut c = Camera::new(Vec3::ZERO, 0.0, 0.3, 10.0).with_limits(5.0, 20.0);
        c.zoom(0.1);
        assert_eq!(c.distance(), 5.0);
        c.zoom(100.0);
        assert_eq!(c.distance(), 20.0);
        c.set_distance(12.0);
        assert_eq!(c.distance(), 12.0);
        let c = Camera::new(Vec3::ZERO, 0.0, 0.0, 500.0).with_limits(1.0, 50.0);
        assert_eq!(c.distance(), 50.0);
    }

    #[test]
    fn orbit_keeps_distance_to_target() {
        let target = Vec3::new(16.0, 13.0, 16.0);
        let mut c = Camera::new(target, 0.0, 0.4, 30.0);
        for _ in 0..50 {
            c.orbit(0.37, 0.05);
            assert!(((c.position() - target).length() - 30.0).abs() < 1e-3);
        }
        // El pitch se limita para no voltear la cámara.
        assert!(c.pitch < std::f32::consts::FRAC_PI_2);
    }

    #[test]
    fn center_ray_points_at_target() {
        let target = Vec3::new(3.0, -2.0, 7.0);
        for (yaw, pitch) in [(0.0, 0.0), (1.2, 0.5), (-2.5, -0.3), (3.0, 1.2)] {
            let c = Camera::new(target, yaw, pitch, 15.0);
            let f = c.frame(64, 36);
            let r = f.ray(32.0, 18.0);
            let expected = (target - c.position()).normalized();
            assert!((r.dir - expected).length() < 1e-5);
            assert!((r.dir.length() - 1.0).abs() < 1e-5);
            // El borde superior de la imagen apunta más arriba que el inferior.
            assert!(f.ray(32.0, 0.0).dir.y > f.ray(32.0, 36.0).dir.y);
        }
    }
}

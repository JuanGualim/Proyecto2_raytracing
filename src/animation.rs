//! Trayectoria de la cámara por frame.
//!
//! Con 288 frames a 24 fps (12 s):
//! - frames 0–191: órbita completa de 360° a distancia media alrededor de la isla;
//! - frames 192–287: la cámara sigue rotando (más despacio) mientras hace un zoom suave
//!   (ease in-out) que se **acerca** a la estatua y al lago y luego se **aleja**.
//!
//! Con otro número de frames las fases se escalan en la misma proporción (2/3 y 1/3).

use crate::camera::Camera;
use crate::math::{lerp, Vec3};
use crate::scene::Landmarks;
use std::f32::consts::TAU;

pub const DEFAULT_FRAMES: u32 = 288;
pub const FPS: u32 = 24;

/// Curva suave de 0 a 1 (ease in-out cúbica).
#[inline]
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub total_frames: u32,
    /// Frames de la órbita inicial de 360°.
    pub orbit_frames: u32,
    /// Ángulo inicial de la órbita (radianes).
    pub start_yaw: f32,
    /// Velocidad de giro al final del zoom, como fracción de la velocidad de la órbita.
    pub zoom_spin: f32,
    pub orbit_pitch: f32,
    pub close_pitch: f32,
    pub orbit_distance: f32,
    pub close_distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    /// Objetivo durante la órbita (centro de la isla).
    pub center: Vec3,
    /// Objetivo en el punto más cercano del zoom (entre la estatua y el lago).
    pub focus: Vec3,
}

impl Animation {
    pub fn new(total_frames: u32, landmarks: &Landmarks) -> Animation {
        let total_frames = total_frames.max(2);
        let orbit_frames =
            ((total_frames as f32 * 2.0 / 3.0).round() as u32).clamp(1, total_frames - 1);
        Animation {
            total_frames,
            orbit_frames,
            start_yaw: 10f32.to_radians(),
            zoom_spin: 0.3,
            orbit_pitch: 20f32.to_radians(),
            close_pitch: 17f32.to_radians(),
            orbit_distance: 33.0,
            close_distance: 13.0,
            min_distance: 10.0,
            max_distance: 60.0,
            center: landmarks.island_center,
            focus: landmarks.statue.lerp(landmarks.lake, 0.4),
        }
    }

    /// Frames de la fase de zoom.
    pub fn zoom_frames(&self) -> u32 {
        self.total_frames - self.orbit_frames
    }

    /// Progreso del zoom en [0, 1] (0 durante la órbita).
    pub fn zoom_progress(&self, frame: u32) -> f32 {
        if frame < self.orbit_frames {
            0.0
        } else {
            (frame - self.orbit_frames) as f32 / self.zoom_frames() as f32
        }
    }

    /// Cuánto se ha acercado la cámara: 0 = distancia media, 1 = lo más cerca.
    /// Sube con ease in-out hasta la mitad del zoom y baja igual.
    pub fn closeness(&self, frame: u32) -> f32 {
        let s = self.zoom_progress(frame);
        if s < 0.5 {
            ease_in_out(s * 2.0)
        } else {
            ease_in_out((1.0 - s) * 2.0)
        }
    }

    pub fn yaw(&self, frame: u32) -> f32 {
        let orbit = frame.min(self.orbit_frames) as f32 / self.orbit_frames as f32;
        // Durante el zoom sigue girando: la velocidad arranca igual a la de la órbita y
        // baja linealmente hasta `zoom_spin` veces esa velocidad (sin saltos).
        let v0 = TAU / self.orbit_frames as f32;
        let v1 = v0 * self.zoom_spin;
        let n = self.zoom_frames() as f32;
        let k = self.zoom_progress(frame) * n;
        let extra = v0 * k - (v0 - v1) * k * k / (2.0 * n);
        self.start_yaw + TAU * orbit + extra
    }

    /// Cámara del frame `frame`.
    pub fn camera(&self, frame: u32) -> Camera {
        let w = self.closeness(frame);
        let target = self.center.lerp(self.focus, w);
        let pitch = lerp(self.orbit_pitch, self.close_pitch, w);
        let distance = lerp(self.orbit_distance, self.close_distance, w);
        Camera::new(target, self.yaw(frame), pitch, distance)
            .with_limits(self.min_distance, self.max_distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn landmarks() -> Landmarks {
        Landmarks {
            island_center: Vec3::new(16.0, 13.0, 16.0),
            statue: Vec3::new(15.5, 17.0, 17.5),
            lake: Vec3::new(20.5, 14.0, 17.0),
            greenhouse: Vec3::new(11.5, 16.0, 11.5),
            ruins: Vec3::new(9.5, 15.0, 20.5),
        }
    }

    #[test]
    fn default_split_is_192_orbit_and_96_zoom() {
        let a = Animation::new(DEFAULT_FRAMES, &landmarks());
        assert_eq!(a.orbit_frames, 192);
        assert_eq!(a.zoom_frames(), 96);
        assert_eq!(DEFAULT_FRAMES / FPS, 12);
    }

    #[test]
    fn orbit_is_a_full_turn_at_constant_distance() {
        let a = Animation::new(DEFAULT_FRAMES, &landmarks());
        for f in 0..a.orbit_frames {
            let cam = a.camera(f);
            assert!((cam.distance() - a.orbit_distance).abs() < 1e-4);
            assert_eq!(cam.target, a.center);
        }
        assert!((a.yaw(192) - a.yaw(0) - TAU).abs() < 1e-4);
        // La rotación no se detiene durante el zoom ni cambia de velocidad de golpe.
        let v0 = a.yaw(192) - a.yaw(191);
        let v1 = a.yaw(193) - a.yaw(192);
        assert!((v0 - v1).abs() < v0 * 0.05);
        for f in 192..287 {
            assert!(a.yaw(f + 1) > a.yaw(f));
        }
    }

    #[test]
    fn zoom_goes_in_and_back_out_smoothly() {
        let a = Animation::new(DEFAULT_FRAMES, &landmarks());
        let d: Vec<f32> = (0..DEFAULT_FRAMES)
            .map(|f| a.camera(f).distance())
            .collect();
        let (closest, dmin) = d
            .iter()
            .enumerate()
            .min_by(|x, y| x.1.total_cmp(y.1))
            .map(|(i, &v)| (i as u32, v))
            .unwrap();
        assert!(
            (236..=244).contains(&closest),
            "lo más cerca en el frame {closest}"
        );
        assert!(dmin < 15.0);
        // Se acerca y luego se aleja casi hasta la distancia de la órbita.
        assert!(d[287] > 30.0);
        for f in 193..240 {
            assert!(d[f] <= d[f - 1] + 1e-4);
        }
        for f in 241..288 {
            assert!(d[f] >= d[f - 1] - 1e-4);
        }
        // Sin saltos entre frames consecutivos.
        for f in 1..DEFAULT_FRAMES {
            let jump = (a.camera(f).position() - a.camera(f - 1).position()).length();
            assert!(jump < 2.5, "salto de {jump} en el frame {f}");
        }
        // En el punto más cercano mira a la estatua y al lago.
        let near = a.camera(closest);
        assert!((near.target - a.focus).length() < 0.2);
    }

    #[test]
    fn other_frame_counts_scale() {
        let a = Animation::new(30, &landmarks());
        assert_eq!(a.orbit_frames, 20);
        let d = a.camera(25).distance();
        assert!(d < a.orbit_distance && d >= a.min_distance);
        assert!(ease_in_out(0.0) == 0.0 && ease_in_out(1.0) == 1.0);
    }
}

//! Caja alineada a los ejes e intersección rayo–caja por el método de slabs.

use crate::math::{Ray, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

/// Resultado de intersectar un rayo con una caja.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlabHit {
    /// Parámetro de entrada, ya recortado a `t_min`.
    pub t_enter: f32,
    /// Parámetro de salida, ya recortado a `t_max`.
    pub t_exit: f32,
    /// Eje por el que entra el rayo; `None` si el rayo empieza dentro de la caja.
    pub entry_axis: Option<usize>,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Aabb {
        Aabb { min, max }
    }

    /// Cubo unitario de la celda `(x, y, z)` del grid.
    pub fn cell(x: i32, y: i32, z: i32) -> Aabb {
        let min = Vec3::new(x as f32, y as f32, z as f32);
        Aabb::new(min, min + Vec3::ONE)
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn contains(&self, p: Vec3) -> bool {
        (0..3).all(|a| p[a] >= self.min[a] && p[a] <= self.max[a])
    }

    /// Método de slabs. Devuelve el intervalo `[t_enter, t_exit]` dentro de `[t_min, t_max]`
    /// en el que el rayo está dentro de la caja, o `None` si no la toca.
    pub fn intersect(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<SlabHit> {
        let mut t_enter = t_min;
        let mut t_exit = t_max;
        let mut entry_axis = None;
        for axis in 0..3 {
            let o = ray.origin[axis];
            let d = ray.dir[axis];
            let (lo, hi) = (self.min[axis], self.max[axis]);
            if d == 0.0 {
                // Rayo paralelo al slab: o está dentro del slab o no toca la caja.
                if o < lo || o > hi {
                    return None;
                }
                continue;
            }
            let inv = 1.0 / d;
            let mut t0 = (lo - o) * inv;
            let mut t1 = (hi - o) * inv;
            if inv < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }
            if t0 > t_enter {
                t_enter = t0;
                entry_axis = Some(axis);
            }
            if t1 < t_exit {
                t_exit = t1;
            }
            if t_enter > t_exit {
                return None;
            }
        }
        Some(SlabHit {
            t_enter,
            t_exit,
            entry_axis,
        })
    }

    /// Intersección con la normal de la cara de entrada (hacia afuera de la caja).
    /// Si el rayo nace dentro, se usa la cara de salida con la normal invertida.
    pub fn hit_with_normal(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<(f32, Vec3)> {
        let hit = self.intersect(ray, t_min, t_max)?;
        match hit.entry_axis {
            Some(axis) => {
                let sign = if ray.dir[axis] > 0.0 { -1.0 } else { 1.0 };
                Some((hit.t_enter, Vec3::axis(axis, sign)))
            }
            None => {
                let p = ray.at(hit.t_exit);
                let rel = p - self.center();
                let half = (self.max - self.min) * 0.5;
                let mut best = 0;
                let mut best_v = -1.0;
                for a in 0..3 {
                    let v = (rel[a] / half[a]).abs();
                    if v > best_v {
                        best_v = v;
                        best = a;
                    }
                }
                let sign = if rel[best] > 0.0 { -1.0 } else { 1.0 };
                Some((hit.t_exit, Vec3::axis(best, sign)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit() -> Aabb {
        Aabb::new(Vec3::ZERO, Vec3::ONE)
    }

    #[test]
    fn ray_that_hits() {
        let r = Ray::new(Vec3::new(0.5, 0.5, -3.0), Vec3::Z);
        let h = unit().intersect(&r, 0.0, f32::INFINITY).unwrap();
        assert!((h.t_enter - 3.0).abs() < 1e-6);
        assert!((h.t_exit - 4.0).abs() < 1e-6);
        assert_eq!(h.entry_axis, Some(2));
        let (t, n) = unit().hit_with_normal(&r, 0.0, f32::INFINITY).unwrap();
        assert!((t - 3.0).abs() < 1e-6);
        assert_eq!(n, Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn ray_that_misses() {
        let r = Ray::new(Vec3::new(2.0, 0.5, -3.0), Vec3::Z);
        assert!(unit().intersect(&r, 0.0, f32::INFINITY).is_none());
        // Apunta en sentido contrario a la caja.
        let r = Ray::new(Vec3::new(0.5, 0.5, -3.0), -Vec3::Z);
        assert!(unit().intersect(&r, 0.0, f32::INFINITY).is_none());
        // Diagonal que pasa por fuera.
        let r = Ray::new(
            Vec3::new(-1.0, 3.0, 0.5),
            Vec3::new(1.0, 0.1, 0.0).normalized(),
        );
        assert!(unit().intersect(&r, 0.0, f32::INFINITY).is_none());
        // Choca, pero más allá de t_max.
        let r = Ray::new(Vec3::new(0.5, 0.5, -3.0), Vec3::Z);
        assert!(unit().intersect(&r, 0.0, 2.0).is_none());
    }

    #[test]
    fn ray_from_inside() {
        let r = Ray::new(Vec3::new(0.5, 0.5, 0.5), Vec3::X);
        let h = unit().intersect(&r, 0.0, f32::INFINITY).unwrap();
        assert_eq!(h.t_enter, 0.0);
        assert!((h.t_exit - 0.5).abs() < 1e-6);
        assert_eq!(h.entry_axis, None);
        let (t, n) = unit().hit_with_normal(&r, 0.0, f32::INFINITY).unwrap();
        assert!((t - 0.5).abs() < 1e-6);
        assert_eq!(n, Vec3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn ray_parallel_to_face() {
        // Paralelo a las caras Y, dentro del slab: choca.
        let r = Ray::new(Vec3::new(-2.0, 0.25, 0.75), Vec3::X);
        let h = unit().intersect(&r, 0.0, f32::INFINITY).unwrap();
        assert!((h.t_enter - 2.0).abs() < 1e-6);
        // Paralelo, fuera del slab: no choca (y no produce NaN).
        let r = Ray::new(Vec3::new(-2.0, 1.5, 0.75), Vec3::X);
        assert!(unit().intersect(&r, 0.0, f32::INFINITY).is_none());
    }

    #[test]
    fn cell_box() {
        let c = Aabb::cell(2, 3, -1);
        assert_eq!(c.min, Vec3::new(2.0, 3.0, -1.0));
        assert_eq!(c.max, Vec3::new(3.0, 4.0, 0.0));
        assert!(c.contains(Vec3::new(2.5, 3.5, -0.5)));
        assert!(!c.contains(Vec3::new(1.5, 3.5, -0.5)));
    }
}

//! Rayo paramétrico `origin + t * dir`.

use super::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    /// Dirección del rayo. Se espera normalizada para que `t` sea una distancia.
    pub dir: Vec3,
}

impl Ray {
    #[inline]
    pub fn new(origin: Vec3, dir: Vec3) -> Ray {
        Ray { origin, dir }
    }

    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_moves_along_direction() {
        let r = Ray::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(r.at(0.0), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(r.at(2.5), Vec3::new(1.0, 2.5, 0.0));
    }
}

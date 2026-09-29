//! Luces de la escena y rayos de sombra.

use crate::material::Material;
use crate::math::{Ray, Vec3};
use crate::world::{dda, VoxelGrid, AIR};

/// Separación a lo largo de la normal para evitar el acné de sombras.
pub const SHADOW_EPS: f32 = 1e-3;

/// Cuántas luces puntuales considera cada punto sombreado.
pub const MAX_POINT_LIGHTS: usize = 4;

/// Luz direccional (el sol).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sun {
    /// Dirección normalizada hacia el sol.
    pub dir: Vec3,
    /// Color × intensidad.
    pub color: Vec3,
}

/// Luz puntual registrada en el centro de un bloque emisivo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    pub pos: Vec3,
    /// Color × intensidad.
    pub color: Vec3,
    /// Celda del bloque emisivo (los rayos de sombra terminan al llegar a ella).
    pub cell: [i32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lights {
    pub sun: Sun,
    /// Luz ambiental uniforme.
    pub ambient: Vec3,
    pub points: Vec<PointLight>,
    /// Coeficiente de atenuación `k` en `1 / (1 + k·d²)`.
    pub k: f32,
    /// Distancia a partir de la cual una luz puntual se ignora.
    pub max_distance: f32,
}

/// Atenuación de una luz puntual a distancia² `d2`.
#[inline]
pub fn attenuation(d2: f32, k: f32) -> f32 {
    1.0 / (1.0 + k * d2)
}

impl Lights {
    /// Índices de hasta `MAX_POINT_LIGHTS` luces más cercanas a `pos` (dentro de
    /// `max_distance`), ordenadas por distancia. Devuelve cuántas se encontraron.
    #[inline]
    pub fn nearest(&self, pos: Vec3, out: &mut [(usize, f32); MAX_POINT_LIGHTS]) -> usize {
        let limit = self.max_distance * self.max_distance;
        let mut count = 0;
        for (i, l) in self.points.iter().enumerate() {
            let d2 = (l.pos - pos).length_squared();
            if d2 > limit {
                continue;
            }
            if count < MAX_POINT_LIGHTS {
                out[count] = (i, d2);
                count += 1;
            } else if d2 < out[MAX_POINT_LIGHTS - 1].1 {
                out[MAX_POINT_LIGHTS - 1] = (i, d2);
            } else {
                continue;
            }
            // Inserción: mantiene el arreglo ordenado.
            let mut j = count - 1;
            while j > 0 && out[j].1 < out[j - 1].1 {
                out.swap(j, j - 1);
                j -= 1;
            }
        }
        count
    }
}

/// Fracción de luz que llega a lo largo de `ray` hasta `t_max` ("any hit").
///
/// Un bloque opaco bloquea la luz por completo. Cada vez que el rayo entra en un material
/// transparente la transmisión se multiplica por su `transparency`. Si el rayo nace dentro de
/// un material transparente (p. ej. el fondo del lago) se aplica una vez su transparencia.
/// `target` es la celda de una luz puntual: al llegar a ella el rayo termina con éxito.
pub fn transmission(
    grid: &VoxelGrid,
    materials: &[Material],
    ray: &Ray,
    t_max: f32,
    target: Option<[i32; 3]>,
) -> f32 {
    let o = ray.origin;
    let start = grid.get(o.x.floor() as i32, o.y.floor() as i32, o.z.floor() as i32);
    let mut trans = if start != AIR {
        materials[start as usize].transparency.max(0.0)
    } else {
        1.0
    };
    let result = dda::walk(grid, ray, 0.0, t_max, |c| {
        if Some(c.cell) == target {
            return Some(trans);
        }
        if c.to == AIR {
            return None;
        }
        let m = &materials[c.to as usize];
        if m.transparency <= 0.0 {
            return Some(0.0);
        }
        trans *= m.transparency;
        if trans < 0.01 {
            Some(0.0)
        } else {
            None
        }
    });
    result.unwrap_or(trans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{material_table, GLASS, GLOWSTONE, STONE};

    #[test]
    fn opaque_blocks_and_glass_attenuates() {
        let mats = material_table();
        let mut g = VoxelGrid::new(8, 8, 8);
        let up = Ray::new(Vec3::new(2.5, 0.5, 2.5), Vec3::Y);
        assert_eq!(transmission(&g, &mats, &up, f32::INFINITY, None), 1.0);
        g.set(2, 5, 2, GLASS);
        let t = transmission(&g, &mats, &up, f32::INFINITY, None);
        assert!((t - 0.85).abs() < 1e-6);
        g.set(2, 6, 2, STONE);
        assert_eq!(transmission(&g, &mats, &up, f32::INFINITY, None), 0.0);
        // Con t_max corto no se llega a los bloques.
        assert_eq!(transmission(&g, &mats, &up, 3.0, None), 1.0);
        // Llegar a la celda objetivo cuenta como luz visible.
        g.set(2, 6, 2, GLOWSTONE);
        assert!((transmission(&g, &mats, &up, 10.0, Some([2, 6, 2])) - 0.85).abs() < 1e-6);
    }

    #[test]
    fn attenuation_decreases_with_distance() {
        assert_eq!(attenuation(0.0, 0.3), 1.0);
        assert!(attenuation(4.0, 0.3) > attenuation(9.0, 0.3));
        assert!(attenuation(1e6, 0.3) < 1e-4);
    }

    #[test]
    fn nearest_lights_sorted_and_limited() {
        let points = (0..7)
            .map(|i| PointLight {
                pos: Vec3::new(i as f32 * 2.0, 0.0, 0.0),
                color: Vec3::ONE,
                cell: [i * 2, 0, 0],
            })
            .collect();
        let lights = Lights {
            sun: Sun {
                dir: Vec3::Y,
                color: Vec3::ONE,
            },
            ambient: Vec3::ZERO,
            points,
            k: 0.3,
            max_distance: 9.0,
        };
        let mut out = [(0, 0.0); MAX_POINT_LIGHTS];
        let n = lights.nearest(Vec3::new(5.2, 0.0, 0.0), &mut out);
        assert_eq!(n, 4);
        let ids: Vec<usize> = out.iter().map(|o| o.0).collect();
        assert_eq!(ids, vec![3, 2, 4, 1]);
        // Fuera de max_distance no hay luces.
        assert_eq!(lights.nearest(Vec3::new(100.0, 0.0, 0.0), &mut out), 0);
    }
}

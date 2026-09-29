//! Luces de la escena y rayos de sombra.

use crate::material::Material;
use crate::math::{Ray, Vec3};
use crate::world::{dda, VoxelGrid, AIR};

/// Separación a lo largo de la normal para evitar el acné de sombras.
pub const SHADOW_EPS: f32 = 1e-3;

/// Luz direccional (el sol).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sun {
    /// Dirección normalizada hacia el sol.
    pub dir: Vec3,
    /// Color × intensidad.
    pub color: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lights {
    pub sun: Sun,
    /// Luz ambiental uniforme.
    pub ambient: Vec3,
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
    use crate::material::{material_table, GLASS, STONE};

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
        assert!((transmission(&g, &mats, &up, 10.0, Some([2, 6, 2])) - 0.85).abs() < 1e-6);
    }
}

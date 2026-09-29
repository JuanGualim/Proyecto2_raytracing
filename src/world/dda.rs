//! Recorrido del grid de vóxeles con el algoritmo de Amanatides & Woo (DDA 3D).
//!
//! Solo hay "superficie" donde cambia el material entre la celda actual y la siguiente:
//! dos bloques de agua contiguos no generan un hit, pero salir del agua al aire sí
//! (con la normal apuntando hacia el agua, es decir, contra el rayo).

use super::voxel_grid::{VoxelGrid, AIR};
use super::Aabb;
use crate::math::{Ray, Vec3};

/// Un cambio de material a lo largo del rayo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crossing {
    pub t: f32,
    /// Eje de la cara cruzada (0 = X, 1 = Y, 2 = Z).
    pub axis: usize,
    /// Normal de la cara cruzada, orientada contra el rayo.
    pub normal: Vec3,
    /// Material del medio por el que venía el rayo.
    pub from: u8,
    /// Material de la celda a la que entra (aire si sale del grid).
    pub to: u8,
    /// Celda a la que entra el rayo (puede quedar fuera del grid).
    pub cell: [i32; 3],
    /// Celda de la que sale el rayo.
    pub prev: [i32; 3],
}

/// Intersección del rayo con una superficie del grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub t: f32,
    pub pos: Vec3,
    /// Normal geométrica de la cara, orientada contra el rayo (`dot(normal, dir) < 0`).
    pub normal: Vec3,
    pub axis: usize,
    /// Coordenadas de textura de la cara, en [0, 1). `v = 0` es el borde superior.
    pub u: f32,
    pub v: f32,
    /// Material de la superficie que se sombrea.
    pub material: u8,
    /// Medio del que viene el rayo.
    pub from: u8,
    /// Medio al otro lado de la interfaz.
    pub to: u8,
    /// Celda del bloque al que pertenece la superficie.
    pub cell: [i32; 3],
}

impl Hit {
    /// `true` si el rayo sale de un material hacia el aire (p. ej. desde dentro del agua).
    #[inline]
    pub fn exiting(&self) -> bool {
        self.to == AIR
    }

    /// Normal de la cara hacia afuera del bloque de la superficie.
    #[inline]
    pub fn outward_normal(&self) -> Vec3 {
        if self.exiting() {
            -self.normal
        } else {
            self.normal
        }
    }
}

/// Coordenadas UV de la cara de la celda `cell` con normal `normal`.
///
/// Cada cara tiene un marco tangente fijo (ver `shading::tangent_frame`): `u` crece a lo
/// largo de la tangente y `v` crece en sentido opuesto a la bitangente, de modo que en las
/// caras laterales `v = 0` es el borde de arriba del bloque.
#[inline]
pub fn face_uv(pos: Vec3, cell: [i32; 3], normal: Vec3) -> (f32, f32) {
    let f = |a: usize| (pos[a] - cell[a] as f32).clamp(0.0, 0.99999);
    let (fx, fy, fz) = (f(0), f(1), f(2));
    if normal.y > 0.5 {
        (fx, fz)
    } else if normal.y < -0.5 {
        (fx, 1.0 - fz)
    } else if normal.x > 0.5 {
        (1.0 - fz, 1.0 - fy)
    } else if normal.x < -0.5 {
        (fz, 1.0 - fy)
    } else if normal.z > 0.5 {
        (fx, 1.0 - fy)
    } else {
        (1.0 - fx, 1.0 - fy)
    }
}

/// Recorre el grid llamando a `visit` en cada cambio de material, en orden de `t`.
/// Se detiene y devuelve el primer `Some` que produzca `visit`; `None` si el rayo
/// sale del grid o supera `t_max` sin que `visit` lo detenga.
///
/// El medio inicial es el material de la celda que contiene el origen (aire si el
/// origen está fuera del grid).
#[inline]
pub fn walk<T>(
    grid: &VoxelGrid,
    ray: &Ray,
    t_min: f32,
    t_max: f32,
    mut visit: impl FnMut(&Crossing) -> Option<T>,
) -> Option<T> {
    let slab = grid.bounds().intersect(ray, t_min, t_max)?;
    let dims = [grid.nx as i32, grid.ny as i32, grid.nz as i32];
    let p = ray.at(slab.t_enter);
    let mut cell = [0i32; 3];
    for a in 0..3 {
        cell[a] = (p[a].floor() as i32).clamp(0, dims[a] - 1);
    }

    let mut step = [0i32; 3];
    let mut next_t = [f32::INFINITY; 3];
    let mut delta = [f32::INFINITY; 3];
    for a in 0..3 {
        let d = ray.dir[a];
        if d > 0.0 {
            step[a] = 1;
            delta[a] = 1.0 / d;
        } else if d < 0.0 {
            step[a] = -1;
            delta[a] = -1.0 / d;
        }
    }

    let mut current = match slab.entry_axis {
        Some(axis) => {
            // Entra desde fuera: la celda sobre el eje de entrada es la del borde.
            cell[axis] = if step[axis] > 0 { 0 } else { dims[axis] - 1 };
            let m = grid.get_cell(cell);
            if m != AIR {
                let mut prev = cell;
                prev[axis] -= step[axis];
                let c = Crossing {
                    t: slab.t_enter,
                    axis,
                    normal: Vec3::axis(axis, -step[axis] as f32),
                    from: AIR,
                    to: m,
                    cell,
                    prev,
                };
                if let Some(r) = visit(&c) {
                    return Some(r);
                }
            }
            m
        }
        None => grid.get_cell(cell),
    };

    for a in 0..3 {
        if step[a] > 0 {
            next_t[a] = ((cell[a] + 1) as f32 - ray.origin[a]) / ray.dir[a];
        } else if step[a] < 0 {
            next_t[a] = (cell[a] as f32 - ray.origin[a]) / ray.dir[a];
        }
    }

    loop {
        let a = if next_t[0] < next_t[1] {
            if next_t[0] < next_t[2] {
                0
            } else {
                2
            }
        } else if next_t[1] < next_t[2] {
            1
        } else {
            2
        };
        let t = next_t[a];
        if t > t_max {
            return None;
        }
        let prev = cell;
        cell[a] += step[a];
        next_t[a] += delta[a];
        let inside = cell[a] >= 0 && cell[a] < dims[a];
        let m = if inside { grid.get_cell(cell) } else { AIR };
        if m != current {
            let c = Crossing {
                t,
                axis: a,
                normal: Vec3::axis(a, -step[a] as f32),
                from: current,
                to: m,
                cell,
                prev,
            };
            if let Some(r) = visit(&c) {
                return Some(r);
            }
            current = m;
        }
        if !inside {
            return None;
        }
    }
}

/// Construye el `Hit` de un cruce.
#[inline]
pub fn hit_from_crossing(ray: &Ray, c: &Crossing) -> Hit {
    let (surface, material) = if c.to != AIR {
        (c.cell, c.to)
    } else {
        (c.prev, c.from)
    };
    let pos = ray.at(c.t);
    let (u, v) = face_uv(pos, surface, c.normal);
    Hit {
        t: c.t,
        pos,
        normal: c.normal,
        axis: c.axis,
        u,
        v,
        material,
        from: c.from,
        to: c.to,
        cell: surface,
    }
}

/// Primera superficie (cambio de material) a lo largo del rayo en `[t_min, t_max]`.
#[inline]
pub fn trace(grid: &VoxelGrid, ray: &Ray, t_min: f32, t_max: f32) -> Option<Hit> {
    walk(grid, ray, t_min, t_max, |c| Some(hit_from_crossing(ray, c)))
}

/// Referencia de fuerza bruta: prueba el rayo contra la AABB de cada cubo no vacío.
/// Solo es válida para rayos que viajan por el aire. Se usa en tests y benchmarks.
pub fn brute_force_trace(
    grid: &VoxelGrid,
    ray: &Ray,
    t_min: f32,
    t_max: f32,
) -> Option<(f32, [i32; 3], u8)> {
    let mut best: Option<(f32, [i32; 3], u8)> = None;
    for (cell, m) in grid.solid_cells() {
        let limit = best.map_or(t_max, |b| b.0);
        if let Some(h) = Aabb::cell(cell[0], cell[1], cell[2]).intersect(ray, t_min, limit) {
            if h.entry_axis.is_some() && best.is_none_or(|b| h.t_enter < b.0) {
                best = Some((h.t_enter, cell, m));
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER: u8 = 4;
    const STONE: u8 = 2;

    fn grid_with_block(cell: [i32; 3], m: u8) -> VoxelGrid {
        let mut g = VoxelGrid::new(8, 8, 8);
        g.set(cell[0], cell[1], cell[2], m);
        g
    }

    #[test]
    fn finds_the_right_voxel() {
        let g = grid_with_block([3, 4, 5], STONE);
        let r = Ray::new(Vec3::new(-2.0, 4.5, 5.5), Vec3::X);
        let h = trace(&g, &r, 0.0, f32::INFINITY).unwrap();
        assert_eq!(h.cell, [3, 4, 5]);
        assert_eq!(h.material, STONE);
        assert!((h.t - 5.0).abs() < 1e-5);
        assert_eq!(h.normal, Vec3::new(-1.0, 0.0, 0.0));
        assert!((h.u - 0.5).abs() < 1e-5 && (h.v - 0.5).abs() < 1e-5);
        // Un rayo diagonal desde dentro del grid también lo encuentra.
        let target = Vec3::new(3.5, 4.5, 5.5);
        let origin = Vec3::new(0.3, 0.2, 0.9);
        let r = Ray::new(origin, (target - origin).normalized());
        assert_eq!(trace(&g, &r, 0.0, f32::INFINITY).unwrap().cell, [3, 4, 5]);
    }

    #[test]
    fn reports_face_normal_for_each_axis() {
        let g = grid_with_block([4, 4, 4], STONE);
        let c = Vec3::splat(4.5);
        for (dir, normal) in [
            (Vec3::X, Vec3::new(-1.0, 0.0, 0.0)),
            (-Vec3::X, Vec3::new(1.0, 0.0, 0.0)),
            (Vec3::Y, Vec3::new(0.0, -1.0, 0.0)),
            (-Vec3::Y, Vec3::new(0.0, 1.0, 0.0)),
            (Vec3::Z, Vec3::new(0.0, 0.0, -1.0)),
            (-Vec3::Z, Vec3::new(0.0, 0.0, 1.0)),
        ] {
            // Desde dentro del grid y desde fuera.
            for dist in [3.0, 20.0] {
                let r = Ray::new(c - dir * dist, dir);
                let h = trace(&g, &r, 0.0, f32::INFINITY).unwrap();
                assert_eq!(h.normal, normal, "dir {dir:?} dist {dist}");
                assert_eq!(h.cell, [4, 4, 4]);
                assert!((h.pos - (c - dir * 0.5)).length() < 1e-4);
            }
        }
    }

    #[test]
    fn does_not_stop_between_adjacent_water_cells() {
        let mut g = VoxelGrid::new(10, 8, 8);
        g.fill_box([2, 4, 4], [6, 4, 4], WATER);
        let r = Ray::new(Vec3::new(-1.0, 4.5, 4.5), Vec3::X);
        let enter = trace(&g, &r, 0.0, f32::INFINITY).unwrap();
        assert_eq!((enter.from, enter.to, enter.material), (AIR, WATER, WATER));
        assert!((enter.t - 3.0).abs() < 1e-5);
        // Continuando desde dentro del agua, el siguiente hit es la salida al aire en x = 7.
        let inside = Ray::new(enter.pos + Vec3::X * 1e-3, Vec3::X);
        let exit = trace(&g, &inside, 0.0, f32::INFINITY).unwrap();
        assert_eq!((exit.from, exit.to, exit.material), (WATER, AIR, WATER));
        assert!((exit.pos.x - 7.0).abs() < 1e-4);
        assert!(exit.exiting());
        // Normal contra el rayo (hacia el agua); la normal exterior del bloque apunta a +X.
        assert_eq!(exit.normal, -Vec3::X);
        assert_eq!(exit.outward_normal(), Vec3::X);
        assert_eq!(exit.cell, [6, 4, 4]);
    }

    #[test]
    fn terminates_when_leaving_the_grid() {
        let g = VoxelGrid::new(8, 8, 8);
        let r = Ray::new(
            Vec3::new(4.0, 4.0, 4.0),
            Vec3::new(0.3, 0.8, -0.5).normalized(),
        );
        assert!(trace(&g, &r, 0.0, f32::INFINITY).is_none());
        let r = Ray::new(Vec3::new(-5.0, 20.0, 4.0), Vec3::X);
        assert!(trace(&g, &r, 0.0, f32::INFINITY).is_none());
        // Un bloque más allá de t_max no se reporta.
        let g = grid_with_block([6, 1, 1], STONE);
        let r = Ray::new(Vec3::new(0.5, 1.5, 1.5), Vec3::X);
        assert!(trace(&g, &r, 0.0, 4.0).is_none());
        assert!(trace(&g, &r, 0.0, 6.0).is_some());
    }

    #[test]
    fn exiting_the_grid_from_water_is_a_surface() {
        let mut g = VoxelGrid::new(4, 4, 4);
        g.fill_box([0, 0, 0], [3, 1, 3], WATER);
        let r = Ray::new(Vec3::new(1.5, 0.5, 1.5), -Vec3::Y);
        let h = trace(&g, &r, 0.0, f32::INFINITY).unwrap();
        assert_eq!((h.from, h.to), (WATER, AIR));
        assert!(h.pos.y.abs() < 1e-5);
    }

    #[test]
    fn matches_brute_force() {
        let mut g = VoxelGrid::new(12, 10, 12);
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut rnd = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 40) as f32 / (1u64 << 24) as f32
        };
        for _ in 0..60 {
            let c = [
                (rnd() * 12.0) as i32,
                (rnd() * 10.0) as i32,
                (rnd() * 12.0) as i32,
            ];
            g.set(c[0], c[1], c[2], 1 + (rnd() * 3.0) as u8);
        }
        let mut hits = 0;
        for _ in 0..500 {
            let origin = Vec3::new(rnd() * 30.0 - 9.0, rnd() * 30.0 - 10.0, rnd() * 30.0 - 9.0);
            if g.bounds().contains(origin) {
                continue;
            }
            let target = Vec3::new(rnd() * 12.0, rnd() * 10.0, rnd() * 12.0);
            let r = Ray::new(origin, (target - origin).normalized());
            let a = trace(&g, &r, 0.0, f32::INFINITY);
            let b = brute_force_trace(&g, &r, 0.0, f32::INFINITY);
            match (a, b) {
                (Some(h), Some((t, _, m))) => {
                    hits += 1;
                    assert!((h.t - t).abs() < 1e-3, "t distinto: {} vs {t}", h.t);
                    assert_eq!(h.material, m);
                }
                (None, None) => {}
                other => panic!("DDA y fuerza bruta no coinciden: {other:?}"),
            }
        }
        assert!(hits > 50);
    }

    #[test]
    fn face_uv_orientation() {
        // Cara superior: u sigue a x, v sigue a z.
        let (u, v) = face_uv(Vec3::new(2.25, 3.0, 5.75), [2, 2, 5], Vec3::Y);
        assert!((u - 0.25).abs() < 1e-5 && (v - 0.75).abs() < 1e-5);
        // Cara lateral: v = 0 arriba.
        let (_, v) = face_uv(Vec3::new(2.0, 2.9, 5.5), [2, 2, 5], -Vec3::X);
        assert!(v < 0.15);
    }
}

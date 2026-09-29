//! Generador de la isla flotante con su lago.
//!
//! Altura de cada columna: `h = base + amp · fbm(x, z)` (4 octavas de Perlin), atenuada
//! por una máscara radial `1 - smoothstep(r0, r1, distancia)` para que los bordes caigan.
//! Debajo de la superficie la isla se afila como un cono invertido irregular.

use super::noise::Perlin;
use crate::material::{GRASS, SAND, STONE, WATER};
use crate::math::smoothstep;
use crate::world::VoxelGrid;

/// Tamaño del mundo en vóxeles.
pub const WORLD_X: usize = 32;
pub const WORLD_Y: usize = 28;
pub const WORLD_Z: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainParams {
    pub seed: u64,
    /// Lado del área de la isla en columnas.
    pub size: usize,
    /// Centro de la isla (x, z).
    pub center: (f32, f32),
    pub base: f32,
    pub amp: f32,
    /// Altura a la que caen los bordes de la isla.
    pub edge_height: f32,
    /// Máscara radial: `1 - smoothstep(r0, r1, d)` con `d` = distancia normalizada al centro.
    pub r0: f32,
    pub r1: f32,
    /// Plano de la superficie del agua: las celdas de agua tienen `y < water_level`.
    pub water_level: i32,
    pub lake_center: (f32, f32),
    pub lake_radius: f32,
    /// Altura mínima de la parte inferior (punta del cono).
    pub bottom: i32,
}

impl Default for TerrainParams {
    fn default() -> TerrainParams {
        TerrainParams {
            seed: 2024,
            size: 24,
            center: (16.0, 16.0),
            base: 15.0,
            amp: 3.0,
            edge_height: 12.0,
            r0: 0.74,
            r1: 1.0,
            water_level: 14,
            lake_center: (20.5, 17.0),
            lake_radius: 3.9,
            bottom: 2,
        }
    }
}

/// Resultado del generador: alturas por columna para ubicar las estructuras.
#[derive(Clone, Debug, PartialEq)]
pub struct Terrain {
    pub params: TerrainParams,
    /// Esquina (x, z) del área de la isla.
    pub origin: (i32, i32),
    /// Altura del bloque superior de cada columna del área (`None` = sin tierra).
    heights: Vec<Option<i32>>,
}

impl Terrain {
    /// Altura del bloque superior de la columna `(x, z)` en coordenadas del mundo.
    pub fn height(&self, x: i32, z: i32) -> Option<i32> {
        let (lx, lz) = (x - self.origin.0, z - self.origin.1);
        let n = self.params.size as i32;
        if lx < 0 || lz < 0 || lx >= n || lz >= n {
            return None;
        }
        self.heights[(lz * n + lx) as usize]
    }

    /// Distancia horizontal del centro de la columna al centro del lago.
    pub fn lake_distance(&self, x: i32, z: i32) -> f32 {
        let (cx, cz) = self.params.lake_center;
        ((x as f32 + 0.5 - cx).powi(2) + (z as f32 + 0.5 - cz).powi(2)).sqrt()
    }

    pub fn is_lake(&self, x: i32, z: i32) -> bool {
        self.lake_distance(x, z) < self.params.lake_radius
    }

    /// Columnas con tierra: `(x, z, altura)`.
    pub fn columns(&self) -> impl Iterator<Item = (i32, i32, i32)> + '_ {
        let n = self.params.size as i32;
        (0..n * n).filter_map(move |i| {
            self.heights[i as usize].map(|h| (self.origin.0 + i % n, self.origin.1 + i / n, h))
        })
    }
}

/// Distancia normalizada al centro de la isla (1 = borde del área), con un poco de ruido
/// para que la silueta no sea un círculo perfecto.
fn radial_distance(p: &TerrainParams, noise: &Perlin, cx: f32, cz: f32) -> f32 {
    let half = p.size as f32 * 0.5;
    let d = ((cx - p.center.0).powi(2) + (cz - p.center.1).powi(2)).sqrt() / half;
    d + 0.09 * noise.noise2(cx * 0.21 + 100.0, cz * 0.21 - 50.0)
}

/// Genera la isla en un grid nuevo de `WORLD_X × WORLD_Y × WORLD_Z`.
pub fn generate(params: &TerrainParams) -> (VoxelGrid, Terrain) {
    let mut grid = VoxelGrid::new(WORLD_X, WORLD_Y, WORLD_Z);
    let noise = Perlin::new(params.seed);
    let detail = Perlin::new(params.seed ^ 0xa5a5_5a5a);
    let n = params.size as i32;
    let origin = (
        (params.center.0 - params.size as f32 * 0.5).round() as i32,
        (params.center.1 - params.size as f32 * 0.5).round() as i32,
    );
    let mut terrain = Terrain {
        params: params.clone(),
        origin,
        heights: vec![None; params.size * params.size],
    };
    let wl = params.water_level;
    let max_h = WORLD_Y as i32 - 8;

    for lz in 0..n {
        for lx in 0..n {
            let (x, z) = (origin.0 + lx, origin.1 + lz);
            let (cx, cz) = (x as f32 + 0.5, z as f32 + 0.5);
            let r = radial_distance(params, &detail, cx, cz);
            let mask = 1.0 - smoothstep(params.r0, params.r1, r);
            if mask < 0.06 {
                continue;
            }

            // Altura de la superficie: fBm de 4 octavas atenuado por la máscara radial.
            let f = noise.fbm2(cx * 0.06, cz * 0.06, 4, 2.0, 0.5);
            let h = params.base + params.amp * f;
            let mut surface = params.edge_height + (h - params.edge_height) * mask;

            // Lago: depresión con fondo en cuenco. La orilla baja suavemente hasta el nivel
            // del agua y nunca queda por debajo de él, así el agua queda contenida.
            let dl = terrain.lake_distance(x, z);
            let in_lake = dl < params.lake_radius;
            let shore = (wl - 1) as f32;
            if in_lake {
                let t = dl / params.lake_radius;
                let floor = shore - 0.6 - 2.6 * (1.0 - t * t);
                surface = surface.min(floor);
            } else if dl < params.lake_radius + 4.5 {
                let blend = smoothstep(params.lake_radius, params.lake_radius + 4.5, dl);
                surface = (shore + (surface - shore) * blend).max(shore);
            }
            let top = (surface.round() as i32).clamp(params.bottom + 1, max_h);
            terrain.heights[(lz * n + lx) as usize] = Some(top);

            // Parte inferior: cono invertido irregular hasta y ≈ bottom.
            let cone = params.bottom as f32
                + (top - params.bottom) as f32 * r.clamp(0.0, 1.0).powf(1.25)
                + 2.2 * (detail.noise2(cx * 0.35, cz * 0.35) + 0.4);
            let bottom = (cone.round() as i32).clamp(params.bottom, top - 1);

            let beach = !in_lake && dl < params.lake_radius + 1.6 && top <= wl;
            for y in bottom..=top {
                let depth = top - y;
                let m = if in_lake {
                    // Fondo de arena con manchas de piedra.
                    if depth < 2 && noise.noise2(cx * 0.7, cz * 0.7 + y as f32) > -0.25 {
                        SAND
                    } else {
                        STONE
                    }
                } else if beach && depth < 2 {
                    SAND
                } else if depth <= 3 {
                    // Grama arriba; debajo se ve como tierra (ver renderer::surface_texture).
                    GRASS
                } else if detail.noise2(cx * 0.3 + y as f32 * 0.21, cz * 0.3 - y as f32 * 0.17)
                    > 0.3
                {
                    // Bolsas de tierra dentro de la piedra de la parte inferior.
                    GRASS
                } else {
                    STONE
                };
                grid.set(x, y, z, m);
            }
            if in_lake {
                for y in top + 1..wl {
                    grid.set(x, y, z, WATER);
                }
            }
        }
    }
    (grid, terrain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_params_match_the_spec() {
        let p = TerrainParams::default();
        assert_eq!(p.size, 24);
        assert_eq!(p.water_level, 14);
        let (_, t) = generate(&p);
        assert_eq!(t.origin, (4, 4));
        assert_eq!(t.height(3, 16), None);
        assert!(t.height(16, 16).is_some());
    }

    #[test]
    fn water_is_contained() {
        let (g, _) = generate(&TerrainParams::default());
        for (c, m) in g.solid_cells() {
            if m != WATER {
                continue;
            }
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let side = g.get(c[0] + dx, c[1], c[2] + dz);
                assert_ne!(side, crate::world::AIR, "el agua se derrama en {c:?}");
            }
            assert_ne!(g.get(c[0], c[1] - 1, c[2]), crate::world::AIR);
        }
    }
}

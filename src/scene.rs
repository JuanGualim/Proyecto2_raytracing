//! Escena: grid de vóxeles, materiales, texturas, luces y cielo. Es inmutable durante el
//! render y se comparte por referencia entre los hilos.
//!
//! `Scene::island` construye el diorama completo: la isla procedural con su lago, un
//! invernadero de vidrio, una estatua de oro de un gato sentado sobre un pedestal junto al
//! agua, ruinas de piedra, faroles de glowstone y árboles.

use crate::lighting::{Lights, PointLight, Sun};
use crate::material::{self, Material, GLOW_COLOR};
use crate::material::{GLASS, GLOWSTONE, GOLD, GRASS, LEAVES, SAND, STONE, WATER, WOOD};
use crate::math::Vec3;
use crate::skybox::Skybox;
use crate::terrain::{self, TerrainParams};
use crate::texture::Texture;
use crate::texture_gen::{self, AssetSource};
use crate::world::{VoxelGrid, AIR};
use std::io;

/// Intensidad de la luz puntual de cada bloque emisivo.
pub const GLOW_LIGHT_INTENSITY: f32 = 3.0;

/// Puntos de interés de la escena, usados por la cámara y los tests.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Landmarks {
    pub island_center: Vec3,
    pub statue: Vec3,
    pub lake: Vec3,
    pub greenhouse: Vec3,
    pub ruins: Vec3,
}

pub struct Scene {
    pub grid: VoxelGrid,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub lights: Lights,
    pub skybox: Skybox,
    /// Densidad de la neblina atmosférica (0 = sin neblina).
    pub fog_density: f32,
    /// Distancia a partir de la cual empieza la neblina.
    pub fog_start: f32,
    pub landmarks: Landmarks,
}

/// Configuración para construir la isla.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneConfig {
    pub seed: u64,
    pub assets: AssetSource,
    /// Lado de cada cara del skybox en píxeles.
    pub sky_size: usize,
}

impl Default for SceneConfig {
    fn default() -> SceneConfig {
        SceneConfig {
            seed: TerrainParams::default().seed,
            assets: AssetSource::Generated,
            sky_size: 256,
        }
    }
}

/// Sol bajo de atardecer, color cálido.
pub fn sunset_sun() -> Sun {
    Sun {
        dir: Vec3::new(-0.62, 0.30, -0.72).normalized(),
        color: Vec3::new(1.0, 0.62, 0.38) * 2.4,
    }
}

impl Scene {
    /// Arma la escena alrededor de un grid ya construido: carga texturas y cielo,
    /// deriva la luz ambiental del cielo y registra una luz por bloque emisivo.
    pub fn from_grid(
        grid: VoxelGrid,
        assets: &AssetSource,
        sun: Sun,
        sky_size: usize,
    ) -> io::Result<Scene> {
        let skybox = Skybox::load(assets, sky_size, sun.dir)?;
        let mut scene = Scene {
            grid,
            materials: material::material_table(),
            textures: texture_gen::load_textures(assets)?,
            lights: Lights {
                sun,
                ambient: skybox.average * 0.35,
                points: Vec::new(),
                k: 0.35,
                max_distance: 10.0,
            },
            skybox,
            fog_density: 0.0,
            fog_start: 0.0,
            landmarks: Landmarks::default(),
        };
        scene.register_emissive_lights();
        Ok(scene)
    }

    /// Escena pequeña con un bloque de cada material, útil para pruebas y benchmarks.
    pub fn demo(assets: &AssetSource) -> io::Result<Scene> {
        Scene::from_grid(demo_grid(), assets, sunset_sun(), 64)
    }

    /// El diorama completo: isla flotante al atardecer.
    pub fn island(config: &SceneConfig) -> io::Result<Scene> {
        let params = TerrainParams {
            seed: config.seed,
            ..TerrainParams::default()
        };
        let (grid, landmarks) = build_island(&params);
        let mut scene = Scene::from_grid(grid, &config.assets, sunset_sun(), config.sky_size)?;
        scene.fog_density = 0.0045;
        scene.fog_start = 22.0;
        scene.landmarks = landmarks;
        Ok(scene)
    }

    /// Registra una luz puntual en el centro de cada bloque emisivo.
    pub fn register_emissive_lights(&mut self) {
        let mut points = Vec::new();
        for (cell, m) in self.grid.solid_cells() {
            if self.materials[m as usize].is_emissive() {
                points.push(PointLight {
                    pos: cell_center(cell),
                    color: GLOW_COLOR * GLOW_LIGHT_INTENSITY,
                    cell,
                });
            }
        }
        self.lights.points = points;
    }

    #[inline]
    pub fn material(&self, id: u8) -> &Material {
        &self.materials[id as usize]
    }

    /// Color del cielo en la dirección `dir`.
    #[inline]
    pub fn sky(&self, dir: Vec3) -> Vec3 {
        self.skybox.sample(dir)
    }

    /// Color de la neblina en la dirección `dir`: el cielo cerca del horizonte.
    #[inline]
    pub fn fog_color(&self, dir: Vec3) -> Vec3 {
        let flat = Vec3::new(dir.x, dir.y.clamp(0.1, 0.25), dir.z).normalized();
        self.sky(flat) * 0.75
    }
}

fn cell_center(c: [i32; 3]) -> Vec3 {
    Vec3::new(c[0] as f32 + 0.5, c[1] as f32 + 0.5, c[2] as f32 + 0.5)
}

/// Grid de la escena de demostración (12×8×12).
pub fn demo_grid() -> VoxelGrid {
    let mut g = VoxelGrid::new(12, 8, 12);
    g.fill_box([0, 0, 0], [11, 0, 11], STONE);
    g.fill_box([0, 1, 0], [11, 1, 11], GRASS);
    g.fill_box([5, 1, 6], [9, 1, 9], WATER);
    g.fill_box([5, 0, 6], [9, 0, 9], SAND);
    g.fill_box([4, 1, 5], [10, 1, 5], SAND);
    g.fill_box([2, 2, 2], [3, 3, 3], STONE);
    g.fill_box([8, 2, 2], [8, 5, 2], WOOD);
    g.fill_box([7, 5, 1], [9, 6, 3], LEAVES);
    g.fill_box([2, 2, 8], [3, 4, 9], GLASS);
    g.fill_box([5, 2, 3], [5, 3, 3], GOLD);
    g.set(5, 4, 3, GOLD);
    g.set(10, 2, 8, WOOD);
    g.set(10, 3, 8, GLOWSTONE);
    g
}

// ---------------------------------------------------------------------------
// Construcción de la isla
// ---------------------------------------------------------------------------

/// Altura del bloque sólido más alto de la columna, sin contar agua.
fn ground(grid: &VoxelGrid, x: i32, z: i32) -> i32 {
    (0..grid.ny as i32)
        .rev()
        .find(|&y| {
            let m = grid.get(x, y, z);
            m != AIR && m != WATER
        })
        .unwrap_or(0)
}

/// Deja las columnas de `[x0, x1] × [z0, z1]` con su bloque superior en `level`:
/// rellena con grama (tierra debajo) y vacía lo que sobra encima. Las columnas sin tierra
/// (fuera de la isla) reciben solo una losa delgada, no un pilar hasta el fondo.
fn flatten(grid: &mut VoxelGrid, x0: i32, x1: i32, z0: i32, z1: i32, level: i32) {
    for z in z0..=z1 {
        for x in x0..=x1 {
            let top = ground(grid, x, z);
            let from = if top == 0 { level - 2 } else { top + 1 };
            for y in from.min(level)..=level {
                grid.set(x, y, z, if y <= level - 3 { STONE } else { GRASS });
            }
            for y in level + 1..grid.ny as i32 {
                grid.set(x, y, z, AIR);
            }
        }
    }
}

/// Invernadero de vidrio de 5×4×5 con marco de madera, glowstone y una planta adentro.
/// Devuelve el centro.
fn build_greenhouse(grid: &mut VoxelGrid, x0: i32, z0: i32) -> Vec3 {
    let (x1, z1) = (x0 + 4, z0 + 4);
    let floor = (x0..=x1)
        .flat_map(|x| (z0..=z1).map(move |z| (x, z)))
        .map(|(x, z)| ground(grid, x, z))
        .max()
        .unwrap_or(14);
    flatten(grid, x0 - 1, x1 + 1, z0 - 1, z1 + 1, floor);
    let (y0, y1) = (floor + 1, floor + 4);
    for y in y0..=y1 {
        for z in z0..=z1 {
            for x in x0..=x1 {
                let edges = (x == x0 || x == x1) as u8
                    + (y == y0 || y == y1) as u8
                    + (z == z0 || z == z1) as u8;
                let m = match edges {
                    0 => AIR,
                    1 => GLASS,
                    // Aristas y esquinas: marco de madera.
                    _ => WOOD,
                };
                grid.set(x, y, z, m);
            }
        }
    }
    // Puerta en la pared este, hacia el camino.
    grid.set(x1, y0, z0 + 2, AIR);
    grid.set(x1, y0 + 1, z0 + 2, AIR);
    // Glowstone sobre un bloque de madera y una planta de hojas.
    grid.set(x0 + 1, y0, z0 + 1, WOOD);
    grid.set(x0 + 1, y0 + 1, z0 + 1, GLOWSTONE);
    grid.set(x0 + 2, y0, z0 + 3, WOOD);
    for (dx, dy, dz) in [
        (2, 1, 3),
        (1, 1, 3),
        (3, 1, 3),
        (2, 1, 2),
        (2, 2, 3),
        (1, 0, 3),
    ] {
        grid.set(x0 + dx, y0 + dy, z0 + dz, LEAVES);
    }
    Vec3::new(x0 as f32 + 2.5, y0 as f32 + 2.0, z0 as f32 + 2.5)
}

/// Pedestal de piedra de 3×1×3 con la estatua de oro de un gato sentado mirando hacia +X.
/// Devuelve el centro de la estatua.
fn build_statue(grid: &mut VoxelGrid, x0: i32, z0: i32) -> Vec3 {
    let base = (x0..x0 + 3)
        .flat_map(|x| (z0..z0 + 3).map(move |z| (x, z)))
        .map(|(x, z)| ground(grid, x, z))
        .max()
        .unwrap_or(13);
    let p = base + 1;
    for z in z0..z0 + 3 {
        for x in x0..x0 + 3 {
            // Relleno de piedra desde el suelo hasta el pedestal.
            for y in ground(grid, x, z) + 1..=p {
                grid.set(x, y, z, STONE);
            }
        }
    }
    // Voxels del gato: (x hacia el frente, y, z a lo ancho). Diseño original.
    const CAT: [(i32, i32, i32); 23] = [
        // Ancas y patas delanteras.
        (0, 0, 0),
        (0, 0, 1),
        (0, 0, 2),
        (1, 0, 0),
        (1, 0, 1),
        (1, 0, 2),
        (2, 0, 0),
        (2, 0, 2),
        // Cuerpo.
        (0, 1, 0),
        (0, 1, 1),
        (0, 1, 2),
        (1, 1, 0),
        (1, 1, 1),
        (1, 1, 2),
        // Cuello angosto: separa la cabeza del cuerpo.
        (1, 2, 1),
        // Cabeza y hocico.
        (1, 3, 0),
        (1, 3, 1),
        (1, 3, 2),
        (2, 3, 1),
        // Orejas.
        (1, 4, 0),
        (1, 4, 2),
        // Cola levantada detrás.
        (0, 2, 2),
        (0, 3, 2),
    ];
    for (dx, dy, dz) in CAT {
        grid.set(x0 + dx, p + 1 + dy, z0 + dz, GOLD);
    }
    Vec3::new(x0 as f32 + 1.5, (p + 3) as f32, z0 as f32 + 1.5)
}

/// Ruinas: dos muros bajos rotos y un arco parcial de piedra. Devuelve su centro.
fn build_ruins(grid: &mut VoxelGrid, x0: i32, z0: i32) -> Vec3 {
    let column = |grid: &mut VoxelGrid, x: i32, z: i32, h: i32| {
        let g = ground(grid, x, z);
        for y in g + 1..=g + h {
            grid.set(x, y, z, STONE);
        }
    };
    // Muro norte (a lo largo de x) con la parte superior rota.
    for (i, h) in [3, 2, 3, 1].into_iter().enumerate() {
        column(grid, x0 + i as i32, z0, h);
    }
    // Muro oeste (a lo largo de z).
    for (i, h) in [2, 3, 1, 2].into_iter().enumerate() {
        column(grid, x0, z0 + 1 + i as i32, h);
    }
    // Arco parcial: dos pilares y un dintel roto.
    let (ax, az) = (x0 + 3, z0 + 3);
    let g = ground(grid, ax, az).max(ground(grid, ax + 2, az));
    for y in ground(grid, ax, az) + 1..=g + 3 {
        grid.set(ax, y, az, STONE);
    }
    for y in ground(grid, ax + 2, az) + 1..=g + 3 {
        grid.set(ax + 2, y, az, STONE);
    }
    grid.set(ax, g + 4, az, STONE);
    grid.set(ax + 1, g + 4, az, STONE);
    // Escombros sueltos.
    column(grid, x0 + 2, z0 + 2, 1);
    column(grid, x0 + 4, z0 + 1, 1);
    Vec3::new(x0 as f32 + 2.5, g as f32 + 1.5, z0 as f32 + 2.5)
}

/// Farol: poste de madera de 2 bloques con glowstone arriba.
fn build_lantern(grid: &mut VoxelGrid, x: i32, z: i32) {
    let g = ground(grid, x, z);
    grid.set(x, g + 1, z, WOOD);
    grid.set(x, g + 2, z, WOOD);
    grid.set(x, g + 3, z, GLOWSTONE);
}

/// Árbol: tronco de madera y copa de hojas.
fn build_tree(grid: &mut VoxelGrid, x: i32, z: i32, trunk: i32) {
    let g = ground(grid, x, z);
    let top = g + trunk;
    for y in g + 1..=top {
        grid.set(x, y, z, WOOD);
    }
    for dy in -1..=2 {
        let r: i32 = if dy <= 0 { 2 } else { 1 };
        for dz in -r..=r {
            for dx in -r..=r {
                let corner = dx.abs() == r && dz.abs() == r;
                if (corner && (r == 2 || dy == 2)) || (dx == 0 && dz == 0 && dy <= 0) {
                    continue;
                }
                let (cx, cy, cz) = (x + dx, top + dy, z + dz);
                if grid.get(cx, cy, cz) == AIR {
                    grid.set(cx, cy, cz, LEAVES);
                }
            }
        }
    }
}

/// Camino de arena: reemplaza el bloque superior de cada celda indicada.
fn build_path(grid: &mut VoxelGrid, cells: &[(i32, i32)]) {
    for &(x, z) in cells {
        let g = ground(grid, x, z);
        if grid.get(x, g, z) == GRASS {
            grid.set(x, g, z, SAND);
        }
    }
}

/// Genera el terreno y coloca todas las estructuras encima.
pub fn build_island(params: &TerrainParams) -> (VoxelGrid, Landmarks) {
    let (mut grid, _) = terrain::generate(params);
    let greenhouse = build_greenhouse(&mut grid, 9, 9);
    let statue = build_statue(&mut grid, 14, 16);
    let ruins = build_ruins(&mut grid, 7, 18);
    build_path(
        &mut grid,
        &[(14, 11), (15, 11), (15, 12), (16, 12), (16, 13), (17, 13)],
    );
    for (x, z) in [(14, 13), (18, 12), (24, 13), (20, 22), (12, 16)] {
        build_lantern(&mut grid, x, z);
    }
    build_tree(&mut grid, 21, 8, 4);
    build_tree(&mut grid, 17, 24, 5);
    build_tree(&mut grid, 6, 14, 4);

    let (lx, lz) = params.lake_center;
    let landmarks = Landmarks {
        island_center: Vec3::new(params.center.0, 13.0, params.center.1),
        statue,
        lake: Vec3::new(lx, params.water_level as f32, lz),
        greenhouse,
        ruins,
    };
    (grid, landmarks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_levels_columns() {
        let mut g = VoxelGrid::new(6, 10, 6);
        g.fill_box([0, 0, 0], [5, 3, 5], GRASS);
        g.fill_box([2, 4, 2], [2, 6, 2], STONE);
        flatten(&mut g, 1, 3, 1, 3, 5);
        for z in 1..=3 {
            for x in 1..=3 {
                assert_eq!(ground(&g, x, z), 5);
            }
        }
        assert_eq!(ground(&g, 0, 0), 3);
        // Una columna vacía recibe una losa de 3 bloques, no un pilar.
        let mut g = VoxelGrid::new(4, 12, 4);
        flatten(&mut g, 1, 1, 1, 1, 9);
        assert_eq!(g.get(1, 9, 1), GRASS);
        assert_eq!(g.get(1, 7, 1), GRASS);
        assert_eq!(g.get(1, 6, 1), AIR);
    }

    #[test]
    fn statue_is_a_gold_cat_on_a_stone_pedestal() {
        let mut g = VoxelGrid::new(8, 16, 8);
        g.fill_box([0, 0, 0], [7, 4, 7], GRASS);
        let c = build_statue(&mut g, 2, 2);
        let gold = g.material_counts()[GOLD as usize];
        assert_eq!(gold, 23);
        // Pedestal 3×3 de piedra justo debajo.
        for z in 2..5 {
            for x in 2..5 {
                assert_eq!(g.get(x, 5, z), STONE);
            }
        }
        // Dos orejas separadas en la parte superior.
        assert_eq!(g.get(3, 10, 2), GOLD);
        assert_eq!(g.get(3, 10, 4), GOLD);
        assert_eq!(g.get(3, 10, 3), AIR);
        assert!(c.y > 6.0);
    }

    #[test]
    fn greenhouse_has_frame_glass_and_light() {
        let mut g = VoxelGrid::new(16, 16, 16);
        g.fill_box([0, 0, 0], [15, 4, 15], GRASS);
        build_greenhouse(&mut g, 4, 4);
        let counts = g.material_counts();
        assert!(counts[GLASS as usize] >= 30);
        assert!(counts[WOOD as usize] >= 30);
        assert_eq!(counts[GLOWSTONE as usize], 1);
        assert!(counts[LEAVES as usize] >= 4);
        // Esquina de madera y pared de vidrio.
        assert_eq!(g.get(4, 5, 4), WOOD);
        assert_eq!(g.get(6, 6, 4), GLASS);
    }
}

//! Escena: grid de vóxeles, materiales, texturas, luces y cielo. Es inmutable durante el
//! render y se comparte por referencia entre los hilos.

use crate::lighting::{Lights, PointLight, Sun};
use crate::material::{self, Material, GLOW_COLOR};
use crate::math::Vec3;
use crate::skybox::Skybox;
use crate::texture::Texture;
use crate::texture_gen::{self, AssetSource};
use crate::world::VoxelGrid;
use std::io;

/// Intensidad de la luz puntual de cada bloque emisivo.
pub const GLOW_LIGHT_INTENSITY: f32 = 3.0;

pub struct Scene {
    pub grid: VoxelGrid,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub lights: Lights,
    pub skybox: Skybox,
}

/// Sol bajo de atardecer, color cálido.
pub fn sunset_sun() -> Sun {
    Sun {
        dir: Vec3::new(-0.62, 0.34, -0.70).normalized(),
        color: Vec3::new(1.0, 0.62, 0.38) * 2.6,
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
                ambient: skybox.average * 0.45,
                points: Vec::new(),
                k: 0.35,
                max_distance: 10.0,
            },
            skybox,
        };
        scene.register_emissive_lights();
        Ok(scene)
    }

    /// Escena pequeña con un bloque de cada material, útil para pruebas y benchmarks.
    pub fn demo(assets: &AssetSource) -> io::Result<Scene> {
        Scene::from_grid(demo_grid(), assets, sunset_sun(), 64)
    }

    /// Registra una luz puntual en el centro de cada bloque emisivo.
    pub fn register_emissive_lights(&mut self) {
        let mut points = Vec::new();
        for (cell, m) in self.grid.solid_cells() {
            if self.materials[m as usize].is_emissive() {
                points.push(PointLight {
                    pos: Vec3::new(cell[0] as f32, cell[1] as f32, cell[2] as f32)
                        + Vec3::splat(0.5),
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
}

/// Grid de la escena de demostración (12×8×12).
pub fn demo_grid() -> VoxelGrid {
    use material::*;
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

//! Escena: grid de vóxeles, materiales, texturas y luces. Es inmutable durante el render.

use crate::lighting::{Lights, Sun};
use crate::material::{self, Material};
use crate::math::Vec3;
use crate::texture::Texture;
use crate::texture_gen::{self, AssetSource};
use crate::world::VoxelGrid;
use std::io;

pub struct Scene {
    pub grid: VoxelGrid,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    pub lights: Lights,
}

impl Scene {
    /// Escena pequeña con un bloque de cada material, útil para pruebas y benchmarks.
    pub fn demo(assets: &AssetSource) -> io::Result<Scene> {
        use material::*;
        let mut g = VoxelGrid::new(12, 8, 12);
        g.fill_box([0, 0, 0], [11, 0, 11], STONE);
        g.fill_box([0, 1, 0], [11, 1, 11], GRASS);
        g.fill_box([5, 1, 6], [9, 1, 9], WATER);
        g.fill_box([4, 1, 5], [10, 1, 5], SAND);
        g.fill_box([2, 2, 2], [3, 3, 3], STONE);
        g.fill_box([8, 2, 2], [8, 5, 2], WOOD);
        g.fill_box([7, 5, 1], [9, 6, 3], LEAVES);
        g.set(8, 6, 2, LEAVES);
        g.fill_box([2, 2, 8], [3, 4, 9], GLASS);
        g.fill_box([5, 2, 3], [5, 3, 3], GOLD);
        g.set(5, 4, 3, GOLD);
        g.set(10, 2, 8, WOOD);
        g.set(10, 3, 8, GLOWSTONE);
        Ok(Scene {
            grid: g,
            materials: material_table(),
            textures: texture_gen::load_textures(assets)?,
            lights: Lights {
                sun: Sun {
                    dir: Vec3::new(-0.55, 0.45, -0.7).normalized(),
                    color: Vec3::new(1.0, 0.62, 0.38) * 2.6,
                },
                ambient: Vec3::new(0.32, 0.26, 0.34),
            },
        })
    }

    #[inline]
    pub fn material(&self, id: u8) -> &Material {
        &self.materials[id as usize]
    }
}

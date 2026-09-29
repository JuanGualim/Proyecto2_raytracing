//! Materiales de los bloques. El id de material es el valor guardado en cada celda del grid.

use crate::math::Vec3;
use crate::texture::TextureId;
use crate::texture_gen::tex;

pub type MaterialId = u8;

pub use crate::world::AIR;
pub const GRASS: MaterialId = 1;
pub const STONE: MaterialId = 2;
pub const WOOD: MaterialId = 3;
pub const WATER: MaterialId = 4;
pub const GLASS: MaterialId = 5;
pub const GOLD: MaterialId = 6;
pub const GLOWSTONE: MaterialId = 7;
pub const SAND: MaterialId = 8;
pub const LEAVES: MaterialId = 9;
pub const MATERIAL_COUNT: usize = 10;

/// Los siete materiales principales que deben verse en el video.
pub const MAIN_MATERIALS: [MaterialId; 7] = [GRASS, STONE, WOOD, WATER, GLASS, GOLD, GLOWSTONE];

/// Color de la luz del glowstone.
pub const GLOW_COLOR: Vec3 = Vec3::new(1.0, 0.85, 0.5);

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: &'static str,
    /// Textura por cara (arriba / lados / abajo); iguales si no aplica.
    pub tex_top: TextureId,
    pub tex_side: TextureId,
    pub tex_bottom: TextureId,
    pub normal_map: Option<TextureId>,
    /// Factor multiplicador del color difuso.
    pub albedo: f32,
    /// Intensidad especular (Blinn-Phong).
    pub specular: f32,
    pub shininess: f32,
    /// 0 = opaco, 1 = totalmente transmisivo.
    pub transparency: f32,
    /// 0..1
    pub reflectivity: f32,
    /// Índice de refracción (1.0 si es opaco).
    pub ior: f32,
    /// Color × intensidad emitida; cero si no emite.
    pub emission: Vec3,
}

impl Material {
    /// Material con la misma textura en todas las caras y valores neutros.
    const fn basic(name: &'static str, texture: TextureId) -> Material {
        Material {
            name,
            tex_top: texture,
            tex_side: texture,
            tex_bottom: texture,
            normal_map: None,
            albedo: 1.0,
            specular: 0.0,
            shininess: 1.0,
            transparency: 0.0,
            reflectivity: 0.0,
            ior: 1.0,
            emission: Vec3::ZERO,
        }
    }

    #[inline]
    pub fn is_transparent(&self) -> bool {
        self.transparency > 0.0
    }

    #[inline]
    pub fn is_emissive(&self) -> bool {
        self.emission.max_component() > 0.0
    }

    /// Textura de la cara según su normal hacia afuera del bloque.
    #[inline]
    pub fn texture_for_normal(&self, outward: Vec3) -> TextureId {
        if outward.y > 0.5 {
            self.tex_top
        } else if outward.y < -0.5 {
            self.tex_bottom
        } else {
            self.tex_side
        }
    }
}

/// Tabla de materiales indexada por id (el índice 0 es el aire).
pub fn material_table() -> Vec<Material> {
    let air = Material::basic("Aire", tex::GLASS);
    let grass = Material {
        tex_top: tex::GRASS_TOP,
        tex_side: tex::GRASS_SIDE,
        // La tierra es la cara inferior de la grama.
        tex_bottom: tex::DIRT,
        albedo: 0.90,
        specular: 0.10,
        shininess: 8.0,
        ..Material::basic("Grama", tex::GRASS_TOP)
    };
    let stone = Material {
        albedo: 0.85,
        specular: 0.30,
        shininess: 16.0,
        reflectivity: 0.05,
        normal_map: Some(tex::STONE_NORMAL),
        ..Material::basic("Piedra", tex::STONE)
    };
    let wood = Material {
        tex_top: tex::WOOD_TOP,
        tex_bottom: tex::WOOD_TOP,
        albedo: 0.90,
        specular: 0.20,
        shininess: 12.0,
        ..Material::basic("Madera", tex::WOOD_SIDE)
    };
    let water = Material {
        albedo: 0.60,
        specular: 0.80,
        shininess: 64.0,
        transparency: 0.70,
        reflectivity: 0.20,
        ior: 1.33,
        ..Material::basic("Agua", tex::WATER)
    };
    let glass = Material {
        albedo: 0.90,
        specular: 0.90,
        shininess: 96.0,
        transparency: 0.85,
        reflectivity: 0.10,
        ior: 1.50,
        ..Material::basic("Vidrio", tex::GLASS)
    };
    let gold = Material {
        albedo: 1.00,
        specular: 1.00,
        shininess: 128.0,
        reflectivity: 0.70,
        ..Material::basic("Oro", tex::GOLD)
    };
    let glowstone = Material {
        albedo: 1.00,
        specular: 0.20,
        shininess: 8.0,
        emission: GLOW_COLOR * 2.2,
        ..Material::basic("Glowstone", tex::GLOWSTONE)
    };
    let sand = Material {
        albedo: 0.90,
        specular: 0.05,
        shininess: 4.0,
        ..Material::basic("Arena", tex::SAND)
    };
    let leaves = Material {
        albedo: 0.85,
        specular: 0.10,
        shininess: 8.0,
        ..Material::basic("Hojas", tex::LEAVES)
    };
    vec![
        air, grass, stone, wood, water, glass, gold, glowstone, sand, leaves,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_indexed_by_id() {
        let t = material_table();
        assert_eq!(t.len(), MATERIAL_COUNT);
        assert_eq!(t[GRASS as usize].name, "Grama");
        assert_eq!(t[STONE as usize].name, "Piedra");
        assert_eq!(t[WOOD as usize].name, "Madera");
        assert_eq!(t[WATER as usize].name, "Agua");
        assert_eq!(t[GLASS as usize].name, "Vidrio");
        assert_eq!(t[GOLD as usize].name, "Oro");
        assert_eq!(t[GLOWSTONE as usize].name, "Glowstone");
        assert_eq!(t[SAND as usize].name, "Arena");
        assert_eq!(t[LEAVES as usize].name, "Hojas");
    }

    #[test]
    fn parameters_are_physically_sane() {
        for m in material_table().iter().skip(1) {
            assert!((0.0..=1.0).contains(&m.albedo), "{}", m.name);
            assert!((0.0..=1.0).contains(&m.transparency), "{}", m.name);
            assert!((0.0..=1.0).contains(&m.reflectivity), "{}", m.name);
            // La energía no supera 1: lo que se refleja más lo que se transmite.
            assert!(m.transparency + m.reflectivity <= 1.0, "{}", m.name);
            assert!(m.shininess >= 1.0);
            // Solo los transparentes tienen IOR distinto de 1.
            if !m.is_transparent() {
                assert_eq!(m.ior, 1.0, "{}", m.name);
            }
        }
    }

    #[test]
    fn special_materials() {
        let t = material_table();
        assert_eq!(t[WATER as usize].ior, 1.33);
        assert_eq!(t[GLASS as usize].ior, 1.5);
        assert!(t[GOLD as usize].reflectivity >= 0.7);
        assert_eq!(t[STONE as usize].normal_map, Some(tex::STONE_NORMAL));
        let emissive: Vec<_> = t.iter().filter(|m| m.is_emissive()).collect();
        assert_eq!(emissive.len(), 1);
        assert_eq!(emissive[0].name, "Glowstone");
        // La grama tiene arriba, lados y abajo distintos.
        let g = &t[GRASS as usize];
        assert_ne!(g.tex_top, g.tex_side);
        assert_ne!(g.tex_side, g.tex_bottom);
        assert_eq!(g.texture_for_normal(Vec3::Y), tex::GRASS_TOP);
        assert_eq!(g.texture_for_normal(-Vec3::Y), tex::DIRT);
        assert_eq!(g.texture_for_normal(Vec3::X), tex::GRASS_SIDE);
    }
}

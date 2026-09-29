//! Texturas procedurales de 16×16 estilo pixel art, generadas de forma determinista.
//!
//! Cada textura se exporta a `assets/textures/<nombre>.bmp`. Si ese archivo ya existe,
//! se carga en lugar de regenerarse, así el usuario puede dibujar las suyas.

use crate::image_io::{bmp, Image};
use crate::texture::{ColorSpace, Texture};
use std::io;
use std::path::{Path, PathBuf};

/// Lado de las texturas generadas.
pub const SIZE: usize = 16;

/// Ids de textura (índices en `Scene::textures`).
pub mod tex {
    pub const GRASS_TOP: usize = 0;
    pub const GRASS_SIDE: usize = 1;
    pub const DIRT: usize = 2;
    pub const STONE: usize = 3;
    pub const STONE_HEIGHT: usize = 4;
    pub const WOOD_SIDE: usize = 5;
    pub const WOOD_TOP: usize = 6;
    pub const WATER: usize = 7;
    pub const GLASS: usize = 8;
    pub const GOLD: usize = 9;
    pub const GLOWSTONE: usize = 10;
    pub const SAND: usize = 11;
    pub const LEAVES: usize = 12;
    pub const COUNT: usize = 13;
}

/// Nombre de archivo (sin extensión) de cada textura.
pub const TEXTURE_NAMES: [&str; tex::COUNT] = [
    "grass_top",
    "grass_side",
    "dirt",
    "stone",
    "stone_height",
    "wood_side",
    "wood_top",
    "water",
    "glass",
    "gold",
    "glowstone",
    "sand",
    "leaves",
];

/// De dónde salen las texturas y el skybox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetSource {
    /// Todo se genera en memoria, sin tocar el disco (tests).
    Generated,
    /// Carpeta raíz de assets: las texturas van en `textures/` y el cielo en `skybox/`.
    /// Se cargan los BMP existentes; los que falten se generan y se exportan ahí.
    Directory(PathBuf),
}

/// Los datos (alturas, normales) no llevan corrección de gamma.
pub fn color_space(id: usize) -> ColorSpace {
    match id {
        tex::STONE_HEIGHT => ColorSpace::Linear,
        _ => ColorSpace::Srgb,
    }
}

// ---------------------------------------------------------------------------
// Utilidades deterministas
// ---------------------------------------------------------------------------

/// Hash entero de tres valores (mezcla tipo murmur).
#[inline]
pub fn hash3(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x
        .wrapping_mul(0x8da6_b343)
        .wrapping_add(y.wrapping_mul(0xd816_3841))
        .wrapping_add(seed.wrapping_mul(0xcb1a_b31f))
        ^ 0x5bd1_e995;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    h
}

/// Valor pseudoaleatorio en [0, 1) para el texel `(x, y)`.
#[inline]
fn rand01(x: usize, y: usize, seed: u32) -> f32 {
    (hash3(x as u32, y as u32, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// Ruido de valor suave que repite cada `SIZE` texels (para que la textura haga tile).
fn tile_noise(x: usize, y: usize, cell: usize, seed: u32) -> f32 {
    let period = SIZE / cell;
    let gx = x / cell;
    let gy = y / cell;
    let fx = (x % cell) as f32 / cell as f32;
    let fy = (y % cell) as f32 / cell as f32;
    let corner = |i: usize, j: usize| rand01(i % period, j % period, seed);
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let top = corner(gx, gy) + (corner(gx + 1, gy) - corner(gx, gy)) * sx;
    let bottom = corner(gx, gy + 1) + (corner(gx + 1, gy + 1) - corner(gx, gy + 1)) * sx;
    top + (bottom - top) * sy
}

/// Distancias al punto de Voronoi más cercano y al segundo (con wrap), e índice del más cercano.
fn cellular(x: usize, y: usize, cells: usize, seed: u32) -> (f32, f32, u32) {
    let cell = SIZE as f32 / cells as f32;
    let px = x as f32 + 0.5;
    let py = y as f32 + 0.5;
    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;
    let mut id = 0;
    for j in 0..cells {
        for i in 0..cells {
            let fx = (i as f32 + 0.2 + 0.6 * rand01(i, j, seed)) * cell;
            let fy = (j as f32 + 0.2 + 0.6 * rand01(i, j, seed ^ 0xabcd)) * cell;
            let mut dx = (px - fx).abs();
            let mut dy = (py - fy).abs();
            dx = dx.min(SIZE as f32 - dx);
            dy = dy.min(SIZE as f32 - dy);
            let d = (dx * dx + dy * dy).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
                id = (j * cells + i) as u32;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2, id)
}

/// Elige un color de una paleta ordenada de oscuro a claro según `t` en [0, 1].
fn pick(palette: &[[u8; 3]], t: f32) -> [u8; 3] {
    let i = (t.clamp(0.0, 0.9999) * palette.len() as f32) as usize;
    palette[i]
}

fn scale(c: [u8; 3], f: f32) -> [u8; 3] {
    let s = |v: u8| (v as f32 * f).round().clamp(0.0, 255.0) as u8;
    [s(c[0]), s(c[1]), s(c[2])]
}

// ---------------------------------------------------------------------------
// Paletas
// ---------------------------------------------------------------------------

const GRASS: [[u8; 3]; 5] = [
    [62, 112, 38],
    [74, 128, 44],
    [88, 146, 52],
    [104, 164, 62],
    [122, 180, 74],
];
const DIRT: [[u8; 3]; 5] = [
    [92, 63, 42],
    [110, 77, 51],
    [126, 89, 60],
    [142, 102, 69],
    [156, 115, 79],
];
const BARK: [[u8; 3]; 4] = [[70, 52, 32], [88, 67, 40], [104, 80, 48], [120, 93, 56]];
const WATER: [[u8; 3]; 5] = [
    [28, 70, 160],
    [36, 86, 180],
    [46, 102, 196],
    [62, 122, 210],
    [96, 154, 226],
];
const GOLD: [[u8; 3]; 5] = [
    [186, 132, 24],
    [220, 170, 40],
    [244, 200, 60],
    [252, 224, 110],
    [255, 244, 180],
];
const GLOW: [[u8; 3]; 5] = [
    [170, 110, 50],
    [214, 150, 70],
    [240, 190, 100],
    [255, 222, 150],
    [255, 244, 200],
];
const SAND: [[u8; 3]; 4] = [
    [200, 184, 130],
    [212, 196, 142],
    [222, 208, 156],
    [232, 220, 172],
];
const LEAVES: [[u8; 3]; 5] = [
    [34, 78, 26],
    [46, 96, 34],
    [58, 114, 42],
    [72, 132, 52],
    [90, 150, 64],
];

// ---------------------------------------------------------------------------
// Generadores
// ---------------------------------------------------------------------------

fn grass_texel(x: usize, y: usize) -> [u8; 3] {
    let t = 0.55 * rand01(x, y, 11) + 0.45 * tile_noise(x, y, 4, 12);
    pick(&GRASS, t)
}

fn dirt_texel(x: usize, y: usize) -> [u8; 3] {
    let r = rand01(x, y, 21);
    if r > 0.95 {
        return [120, 116, 110]; // piedrita
    }
    pick(&DIRT, 0.6 * r + 0.4 * tile_noise(x, y, 4, 22))
}

fn grass_top(x: usize, y: usize) -> [u8; 3] {
    grass_texel(x, y)
}

fn grass_side(x: usize, y: usize) -> [u8; 3] {
    // Franja verde irregular arriba, con algunas "gotas" que bajan.
    let mut depth = 3 + (hash3(x as u32, 0, 31) % 2) as usize;
    if hash3(x as u32, 1, 32).is_multiple_of(5) {
        depth += 2;
    }
    if y < depth {
        grass_texel(x, y)
    } else {
        dirt_texel(x, y)
    }
}

fn dirt(x: usize, y: usize) -> [u8; 3] {
    dirt_texel(x, y)
}

/// Altura de la piedra en [0, 1]: adoquines (celdas de Voronoi) con juntas hundidas.
pub fn stone_height_value(x: usize, y: usize) -> f32 {
    let (f1, f2, _) = cellular(x, y, 3, 41);
    let edge = ((f2 - f1) / 2.2).clamp(0.0, 1.0);
    let bump = edge.sqrt();
    (0.15 + 0.75 * bump + 0.1 * rand01(x, y, 42)).clamp(0.0, 1.0)
}

fn stone(x: usize, y: usize) -> [u8; 3] {
    let (_, _, id) = cellular(x, y, 3, 41);
    let h = stone_height_value(x, y);
    let tint = (hash3(id, 7, 43) % 3) as f32 * 8.0;
    let g = 70.0 + 70.0 * h + tint + 10.0 * rand01(x, y, 44);
    let g = g.clamp(0.0, 255.0);
    [g as u8, g as u8, (g + 4.0).min(255.0) as u8]
}

fn stone_height(x: usize, y: usize) -> [u8; 3] {
    let v = (stone_height_value(x, y) * 255.0).round() as u8;
    [v, v, v]
}

fn wood_side(x: usize, y: usize) -> [u8; 3] {
    // Vetas verticales: cada columna tiene su tono, con grietas oscuras.
    let column = rand01(x, 0, 51);
    let t = 0.55 * column + 0.3 * rand01(x, y / 3, 52) + 0.15 * rand01(x, y, 53);
    let c = pick(&BARK, t);
    if hash3(x as u32, 0, 54).is_multiple_of(5) && rand01(x, y, 55) > 0.25 {
        scale(c, 0.7)
    } else {
        c
    }
}

fn wood_top(x: usize, y: usize) -> [u8; 3] {
    if x == 0 || y == 0 || x == SIZE - 1 || y == SIZE - 1 {
        return pick(&BARK, rand01(x, y, 61));
    }
    let dx = x as f32 + 0.5 - 8.0;
    let dy = y as f32 + 0.5 - 8.0;
    let d = (dx * dx + dy * dy).sqrt() + 0.35 * rand01(x, y, 62);
    let ring = (d * 0.85) as u32 % 2;
    if ring == 0 {
        [184, 148, 94]
    } else {
        [150, 116, 70]
    }
}

fn water(x: usize, y: usize) -> [u8; 3] {
    let fx = x as f32;
    let fy = y as f32;
    let wave = 0.5 + 0.5 * ((fx + 2.0 * (fy * 0.8).sin()) * 0.8).sin();
    pick(&WATER, 0.65 * wave + 0.35 * rand01(x, y, 71))
}

fn glass(x: usize, y: usize) -> [u8; 3] {
    if x == 0 || y == 0 || x == SIZE - 1 || y == SIZE - 1 {
        return [170, 196, 206];
    }
    let s = x + y;
    if (s == 6 || s == 7) && (2..7).contains(&x) || (s == 20 || s == 21) && (9..14).contains(&x) {
        return [252, 254, 255];
    }
    if rand01(x, y, 81) > 0.93 {
        [210, 226, 232]
    } else {
        [228, 240, 244]
    }
}

fn gold(x: usize, y: usize) -> [u8; 3] {
    if x == 0 || y == 0 {
        return GOLD[4];
    }
    if x == SIZE - 1 || y == SIZE - 1 {
        return GOLD[0];
    }
    let mut t = 0.45 + 0.35 * tile_noise(x, y, 4, 91) + 0.2 * rand01(x, y, 92) - 0.2;
    if (x + y).is_multiple_of(7) {
        t += 0.35;
    }
    pick(&GOLD, t)
}

fn glowstone(x: usize, y: usize) -> [u8; 3] {
    let (f1, f2, id) = cellular(x, y, 4, 101);
    if f2 - f1 < 0.45 {
        return GLOW[0];
    }
    let t = 0.25 + 0.75 * (hash3(id, 3, 102) % 1000) as f32 / 1000.0 - 0.15 * (f1 / 3.0);
    pick(&GLOW, t)
}

fn sand(x: usize, y: usize) -> [u8; 3] {
    pick(
        &SAND,
        0.7 * rand01(x, y, 111) + 0.3 * tile_noise(x, y, 4, 112),
    )
}

fn leaves(x: usize, y: usize) -> [u8; 3] {
    let r = rand01(x, y, 121);
    if r > 0.9 {
        return [20, 44, 16];
    }
    pick(&LEAVES, 0.6 * r / 0.9 + 0.4 * tile_noise(x, y, 4, 122))
}

/// Genera la imagen de la textura `id`.
pub fn generate_image(id: usize) -> Image {
    let f: fn(usize, usize) -> [u8; 3] = match id {
        tex::GRASS_TOP => grass_top,
        tex::GRASS_SIDE => grass_side,
        tex::DIRT => dirt,
        tex::STONE => stone,
        tex::STONE_HEIGHT => stone_height,
        tex::WOOD_SIDE => wood_side,
        tex::WOOD_TOP => wood_top,
        tex::WATER => water,
        tex::GLASS => glass,
        tex::GOLD => gold,
        tex::GLOWSTONE => glowstone,
        tex::SAND => sand,
        tex::LEAVES => leaves,
        _ => panic!("id de textura desconocido: {id}"),
    };
    Image::from_fn(SIZE, SIZE, f)
}

/// Carga el BMP `dir/name.bmp` si existe y es válido; si no, genera la imagen y la exporta.
pub fn load_or_create(
    dir: &Path,
    name: &str,
    generate: impl FnOnce() -> Image,
) -> io::Result<(Image, bool)> {
    let path = dir.join(format!("{name}.bmp"));
    if path.exists() {
        match bmp::read_bmp(&path) {
            Ok(img) => return Ok((img, true)),
            Err(e) => eprintln!(
                "aviso: no se pudo leer {}: {e}; se regenera",
                path.display()
            ),
        }
    }
    let img = generate();
    std::fs::create_dir_all(dir)?;
    bmp::write_bmp(&path, &img)?;
    Ok((img, false))
}

/// Obtiene todas las texturas, generándolas o cargándolas según `source`.
pub fn load_textures(source: &AssetSource) -> io::Result<Vec<Texture>> {
    let mut textures = Vec::with_capacity(tex::COUNT);
    for (id, name) in TEXTURE_NAMES.iter().enumerate() {
        let img = match source {
            AssetSource::Generated => generate_image(id),
            AssetSource::Directory(root) => {
                load_or_create(&root.join("textures"), name, || generate_image(id))?.0
            }
        };
        textures.push(Texture::from_image(&img, color_space(id)));
    }
    Ok(textures)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_and_sized() {
        for id in 0..tex::COUNT {
            let a = generate_image(id);
            let b = generate_image(id);
            assert_eq!(a, b);
            assert_eq!((a.width, a.height), (SIZE, SIZE));
            // Ninguna textura es de un solo color.
            let first = a.pixels[0];
            assert!(a.pixels.iter().any(|&p| p != first), "textura {id} plana");
        }
    }

    #[test]
    fn palettes_have_expected_hues() {
        let avg = |id: usize| {
            let t = Texture::from_image(&generate_image(id), ColorSpace::Srgb);
            t.average()
        };
        let g = avg(tex::GRASS_TOP);
        assert!(g.y > g.x && g.y > g.z, "la grama debe ser verde");
        let w = avg(tex::WATER);
        assert!(w.z > w.x && w.z > w.y, "el agua debe ser azul");
        let au = avg(tex::GOLD);
        assert!(au.x > au.z * 2.0, "el oro debe ser amarillo");
        let s = avg(tex::STONE);
        assert!((s.x - s.y).abs() < 0.02, "la piedra debe ser gris");
    }

    #[test]
    fn tile_noise_wraps() {
        for y in 0..SIZE {
            // El ruido es periódico: el borde derecho continúa en el izquierdo.
            let right = tile_noise(SIZE - 1, y, 4, 5);
            let left = tile_noise(0, y, 4, 5);
            assert!((right - left).abs() < 0.6);
        }
    }

    #[test]
    fn load_or_create_prefers_existing_files() {
        let dir = std::env::temp_dir().join(format!("diorama_tex_test_{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let (img, loaded) = load_or_create(&dir, "sand", || generate_image(tex::SAND)).unwrap();
        assert!(!loaded);
        assert!(dir.join("sand.bmp").exists());
        // Si el usuario reemplaza el archivo, se carga el suyo.
        let custom = Image::from_fn(8, 8, |_, _| [255, 0, 255]);
        bmp::write_bmp(&dir.join("sand.bmp"), &custom).unwrap();
        let (img2, loaded2) = load_or_create(&dir, "sand", || generate_image(tex::SAND)).unwrap();
        assert!(loaded2);
        assert_eq!(img2, custom);
        assert_ne!(img, img2);
        std::fs::remove_dir_all(&dir).ok();
    }
}

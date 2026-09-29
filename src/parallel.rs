//! Render multihilo por bloques de 16×16 píxeles.
//!
//! Un `AtomicUsize` reparte los bloques entre `threads` hilos creados con
//! `std::thread::scope`. Cada hilo escribe en su propio buffer local de bloque y, al
//! terminar, los bloques se copian al framebuffer: no hay `Mutex` en el loop caliente ni
//! asignaciones por píxel. El jitter del antialiasing sale de un hash de
//! `(x, y, muestra, frame)`, así que la imagen no depende del número de hilos.

use crate::camera::{Camera, CameraFrame};
use crate::color;
use crate::image_io::Image;
use crate::math::Vec3;
use crate::renderer::{RenderSettings, Tracer};
use crate::scene::Scene;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Lado de los bloques en píxeles.
pub const TILE: usize = 16;

type TileBuffer = [[u8; 3]; TILE * TILE];

/// Métricas de un frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameStats {
    pub elapsed: Duration,
    /// Rayos lanzados (primarios, secundarios y de sombra).
    pub rays: u64,
    pub threads: usize,
}

impl FrameStats {
    pub fn millis(&self) -> f64 {
        self.elapsed.as_secs_f64() * 1000.0
    }

    pub fn rays_per_second(&self) -> f64 {
        self.rays as f64 / self.elapsed.as_secs_f64().max(1e-9)
    }
}

/// Número de hilos disponibles en la máquina.
pub fn available_threads() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// Hash determinista de 4 enteros (mezcla tipo murmur3).
#[inline]
pub fn hash4(a: u32, b: u32, c: u32, d: u32) -> u32 {
    let mut h = 0x9e37_79b9u32;
    for k in [a, b, c, d] {
        let mut k = k.wrapping_mul(0xcc9e_2d51);
        k = k.rotate_left(15).wrapping_mul(0x1b87_3593);
        h ^= k;
        h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xe654_6b64);
    }
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^ (h >> 16)
}

#[inline]
fn unit(h: u32) -> f32 {
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Desplazamiento dentro del píxel de la muestra `s` de `spp`, en [0, 1)².
/// Con 1 muestra se usa el centro. Con más, se estratifica en una cuadrícula de
/// `ceil(sqrt(spp))²` celdas y se agrega jitter dentro de cada celda.
#[inline]
pub fn sample_offset(x: u32, y: u32, s: u32, spp: u32, frame: u32) -> (f32, f32) {
    if spp <= 1 {
        return (0.5, 0.5);
    }
    let n = (spp as f32).sqrt().ceil() as u32;
    let (cx, cy) = (s % n, (s / n) % n);
    let h = hash4(x, y, s, frame);
    let jx = unit(h);
    let jy = unit(hash4(h, y, x, frame ^ 0x5bd1_e995));
    ((cx as f32 + jx) / n as f32, (cy as f32 + jy) / n as f32)
}

/// Color lineal promedio del píxel `(x, y)`.
#[inline]
pub fn render_pixel(
    tracer: &mut Tracer,
    cam: &CameraFrame,
    spp: u32,
    frame: u32,
    x: usize,
    y: usize,
) -> Vec3 {
    let spp = spp.max(1);
    let mut sum = Vec3::ZERO;
    for s in 0..spp {
        let (ox, oy) = sample_offset(x as u32, y as u32, s, spp, frame);
        let ray = cam.ray(x as f32 + ox, y as f32 + oy);
        sum += tracer.trace(&ray, 0, 1.0);
    }
    sum / spp as f32
}

/// Renderiza el bloque `tile` en `out` (filas de `TILE` píxeles).
fn render_tile(
    tracer: &mut Tracer,
    cam: &CameraFrame,
    settings: &RenderSettings,
    frame: u32,
    tile: usize,
    out: &mut TileBuffer,
) {
    let tiles_x = settings.width.div_ceil(TILE);
    let x0 = (tile % tiles_x) * TILE;
    let y0 = (tile / tiles_x) * TILE;
    let x1 = (x0 + TILE).min(settings.width);
    let y1 = (y0 + TILE).min(settings.height);
    for y in y0..y1 {
        for x in x0..x1 {
            let c = render_pixel(tracer, cam, settings.spp, frame, x, y);
            out[(y - y0) * TILE + (x - x0)] = color::linear_to_rgb8(c, settings.exposure);
        }
    }
}

/// Renderiza un frame completo con `threads` hilos.
pub fn render_frame(
    scene: &Scene,
    camera: &Camera,
    settings: &RenderSettings,
    frame: u32,
    threads: usize,
) -> (Image, FrameStats) {
    let start = Instant::now();
    let (w, h) = (settings.width, settings.height);
    let tiles_x = w.div_ceil(TILE);
    let n_tiles = tiles_x * h.div_ceil(TILE);
    let threads = threads.clamp(1, n_tiles.max(1));
    let cam = camera.frame(w, h);
    let next = AtomicUsize::new(0);

    let per_thread: Vec<(Vec<(usize, TileBuffer)>, u64)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut tracer = Tracer::new(scene, settings.max_depth);
                    let mut done = Vec::with_capacity(n_tiles / threads + 1);
                    loop {
                        let tile = next.fetch_add(1, Ordering::Relaxed);
                        if tile >= n_tiles {
                            break;
                        }
                        let mut block = [[0u8; 3]; TILE * TILE];
                        render_tile(&mut tracer, &cam, settings, frame, tile, &mut block);
                        done.push((tile, block));
                    }
                    (done, tracer.rays)
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().expect("un hilo de render falló"))
            .collect()
    });

    // Copia de los bloques al framebuffer.
    let mut img = Image::new(w, h);
    let mut rays = 0;
    for (blocks, r) in per_thread {
        rays += r;
        for (tile, block) in blocks {
            let x0 = (tile % tiles_x) * TILE;
            let y0 = (tile / tiles_x) * TILE;
            let cols = TILE.min(w - x0);
            for row in 0..TILE.min(h - y0) {
                let dst = (y0 + row) * w + x0;
                img.pixels[dst..dst + cols].copy_from_slice(&block[row * TILE..row * TILE + cols]);
            }
        }
    }
    let stats = FrameStats {
        elapsed: start.elapsed(),
        rays,
        threads,
    };
    (img, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture_gen::AssetSource;

    #[test]
    fn hash_is_deterministic_and_spread() {
        assert_eq!(hash4(1, 2, 3, 4), hash4(1, 2, 3, 4));
        assert_ne!(hash4(1, 2, 3, 4), hash4(1, 2, 3, 5));
        assert_ne!(hash4(1, 2, 3, 4), hash4(2, 1, 3, 4));
        let mean: f32 = (0..1000).map(|i| unit(hash4(i, 7, 0, 0))).sum::<f32>() / 1000.0;
        assert!((mean - 0.5).abs() < 0.05);
    }

    #[test]
    fn sample_offsets_are_stratified_and_inside_the_pixel() {
        assert_eq!(sample_offset(3, 4, 0, 1, 0), (0.5, 0.5));
        let offs: Vec<_> = (0..4).map(|s| sample_offset(10, 20, s, 4, 7)).collect();
        for &(ox, oy) in &offs {
            assert!((0.0..1.0).contains(&ox) && (0.0..1.0).contains(&oy));
        }
        // Una muestra por cuadrante con 4 spp.
        let quadrants: std::collections::HashSet<_> = offs
            .iter()
            .map(|&(x, y)| ((x >= 0.5) as u8, (y >= 0.5) as u8))
            .collect();
        assert_eq!(quadrants.len(), 4);
        // Cambia con el frame, pero es reproducible.
        assert_ne!(
            sample_offset(10, 20, 1, 4, 7),
            sample_offset(10, 20, 1, 4, 8)
        );
        assert_eq!(sample_offset(10, 20, 1, 4, 7), offs[1]);
    }

    #[test]
    fn same_image_for_any_thread_count_and_odd_sizes() {
        let scene = Scene::demo(&AssetSource::Generated).unwrap();
        let cam = Camera::new(Vec3::new(6.0, 2.0, 6.0), 2.4, 0.5, 20.0);
        // Tamaño que no es múltiplo de 16 para probar los bloques del borde.
        let settings = RenderSettings {
            width: 37,
            height: 21,
            spp: 2,
            max_depth: 3,
            exposure: 1.0,
        };
        let (a, sa) = render_frame(&scene, &cam, &settings, 3, 1);
        let (b, sb) = render_frame(&scene, &cam, &settings, 3, 3);
        assert_eq!(a, b);
        assert_eq!(sa.rays, sb.rays);
        assert_eq!(sb.threads, 3);
        assert!(sa.rays >= 37 * 21 * 2);
    }
}

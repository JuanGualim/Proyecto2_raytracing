//! Renderizador: traza rayos por el grid y sombrea cada superficie.

use crate::camera::Camera;
use crate::color;
use crate::image_io::Image;
use crate::lighting::{self, SHADOW_EPS};
use crate::material::GRASS;
use crate::math::{Ray, Vec3};
use crate::scene::Scene;
use crate::texture::TextureId;
use crate::world::{dda, Hit, AIR};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSettings {
    pub width: usize,
    pub height: usize,
    /// Muestras por píxel.
    pub spp: u32,
    /// Profundidad máxima de recursión.
    pub max_depth: u32,
    pub exposure: f32,
}

impl Default for RenderSettings {
    fn default() -> RenderSettings {
        RenderSettings {
            width: 640,
            height: 360,
            spp: 1,
            max_depth: 3,
            exposure: 1.0,
        }
    }
}

/// Cielo provisional: gradiente de atardecer.
pub fn sky_color(dir: Vec3) -> Vec3 {
    let t = dir.y.clamp(-1.0, 1.0);
    let horizon = Vec3::new(1.0, 0.55, 0.3);
    let zenith = Vec3::new(0.25, 0.22, 0.5);
    if t >= 0.0 {
        horizon.lerp(zenith, t.sqrt())
    } else {
        horizon.lerp(Vec3::new(0.3, 0.2, 0.3), (-t).sqrt())
    }
}

/// Textura de la cara golpeada. La grama tapada por otro bloque se ve como tierra.
#[inline]
pub fn surface_texture(scene: &Scene, hit: &Hit) -> TextureId {
    let mat = scene.material(hit.material);
    if hit.material == GRASS {
        let [x, y, z] = hit.cell;
        let above = scene.grid.get(x, y + 1, z);
        if above != AIR && !scene.material(above).is_transparent() {
            return mat.tex_bottom;
        }
    }
    mat.texture_for_normal(hit.outward_normal())
}

/// Iluminación directa (ambiente + sol con sombra) con Lambert y Blinn-Phong.
fn direct_light(scene: &Scene, hit: &Hit, n: Vec3, view: Vec3, base: Vec3) -> Vec3 {
    let mat = scene.material(hit.material);
    let lights = &scene.lights;
    let mut c = lights.ambient * base;
    let origin = hit.pos + hit.normal * SHADOW_EPS;

    let l = lights.sun.dir;
    let ndl = n.dot(l);
    if ndl > 0.0 && hit.normal.dot(l) > 0.0 {
        let shadow = lighting::transmission(
            &scene.grid,
            &scene.materials,
            &Ray::new(origin, l),
            f32::INFINITY,
            None,
        );
        if shadow > 0.0 {
            let h = (l + view).normalized();
            let spec = mat.specular * n.dot(h).max(0.0).powf(mat.shininess);
            c += lights.sun.color * shadow * (base * ndl + Vec3::splat(spec));
        }
    }
    c
}

/// Color que llega por `ray`.
pub fn trace(scene: &Scene, ray: &Ray) -> Vec3 {
    let Some(hit) = dda::trace(&scene.grid, ray, 0.0, f32::INFINITY) else {
        return sky_color(ray.dir);
    };
    let mat = scene.material(hit.material);
    let tex = &scene.textures[surface_texture(scene, &hit)];
    let texel = tex.sample_nearest(hit.u, hit.v);
    if mat.is_emissive() {
        return texel * mat.emission;
    }
    let base = texel * mat.albedo;
    direct_light(scene, &hit, hit.normal, -ray.dir, base)
}

/// Render de un solo hilo, una muestra por píxel en el centro.
pub fn render(scene: &Scene, camera: &Camera, settings: &RenderSettings) -> Image {
    let frame = camera.frame(settings.width, settings.height);
    Image::from_fn(settings.width, settings.height, |x, y| {
        let ray = frame.ray(x as f32 + 0.5, y as f32 + 0.5);
        color::linear_to_rgb8(trace(scene, &ray), settings.exposure)
    })
}

/// Buffer con el id del material que ve cada rayo primario (0 = cielo).
pub fn material_id_buffer(scene: &Scene, camera: &Camera, width: usize, height: usize) -> Vec<u8> {
    let frame = camera.frame(width, height);
    let mut ids = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let ray = frame.ray(x as f32 + 0.5, y as f32 + 0.5);
            ids.push(dda::trace(&scene.grid, &ray, 0.0, f32::INFINITY).map_or(AIR, |h| h.material));
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{GLOWSTONE, STONE};
    use crate::texture_gen::AssetSource;

    fn demo_camera() -> Camera {
        Camera::new(Vec3::new(6.0, 2.0, 6.0), 2.4, 0.55, 20.0)
    }

    #[test]
    fn demo_scene_renders_without_nan() {
        let scene = Scene::demo(&AssetSource::Generated).unwrap();
        let cam = demo_camera();
        let frame = cam.frame(32, 18);
        for y in 0..18 {
            for x in 0..32 {
                let c = trace(&scene, &frame.ray(x as f32 + 0.5, y as f32 + 0.5));
                assert!(c.is_finite() && c.min_component() >= 0.0);
            }
        }
    }

    #[test]
    fn shadowed_point_is_darker() {
        let scene = Scene::demo(&AssetSource::Generated).unwrap();
        let sun = scene.lights.sun.dir;
        // Un punto del piso justo "detrás" del pilar de piedra respecto al sol está en sombra.
        let shadowed = Vec3::new(2.5, 2.0, 2.5) - sun * 1.8;
        let shadowed = Vec3::new(shadowed.x, 2.0, shadowed.z);
        let lit = Vec3::new(6.5, 2.0, 1.5);
        let probe = |p: Vec3| {
            let from = p + Vec3::new(2.0, 5.0, 2.0);
            trace(&scene, &Ray::new(from, (p - from).normalized())).luminance()
        };
        assert!(probe(shadowed) < probe(lit));
    }

    #[test]
    fn emissive_block_is_bright_and_id_buffer_sees_materials() {
        let scene = Scene::demo(&AssetSource::Generated).unwrap();
        let ids = material_id_buffer(&scene, &demo_camera(), 64, 36);
        assert!(ids.contains(&STONE));
        assert!(ids.contains(&GRASS));
        let target = Vec3::new(10.5, 3.5, 8.5);
        let from = target + Vec3::new(0.0, 0.0, 4.0);
        let c = trace(&scene, &Ray::new(from, (target - from).normalized()));
        assert!(c.max_component() > 0.8);
        assert_eq!(scene.grid.get(10, 3, 8), GLOWSTONE);
    }
}

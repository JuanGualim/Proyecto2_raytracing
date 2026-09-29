//! Renderizador: `trace(ray, depth)` recorre el grid y sombrea cada superficie con luz
//! directa, reflexión y refracción recursivas.

use crate::camera::Camera;
use crate::lighting::{self, MAX_POINT_LIGHTS, SHADOW_EPS};
use crate::material::{GRASS, WATER};
use crate::math::{Ray, Vec3};
use crate::scene::Scene;
use crate::shading;
use crate::texture::TextureId;
use crate::world::{dda, Hit, AIR};

/// Contribución mínima para seguir lanzando rayos secundarios.
pub const MIN_CONTRIBUTION: f32 = 0.01;

/// Coeficientes de absorción del agua por unidad de distancia (Beer–Lambert): el rojo se
/// absorbe más rápido, así que lo que se ve a través del lago se tiñe de azul verdoso.
pub const WATER_ABSORPTION: Vec3 = Vec3::new(0.42, 0.13, 0.08);

/// Color que el agua dispersa hacia el ojo (le da cuerpo turquesa al lago).
pub const WATER_SCATTER: Vec3 = Vec3::new(0.03, 0.16, 0.22);

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

/// Traza rayos sobre una escena y cuenta cuántos lanzó.
pub struct Tracer<'a> {
    pub scene: &'a Scene,
    pub max_depth: u32,
    /// Rayos lanzados (primarios, secundarios y de sombra).
    pub rays: u64,
}

impl<'a> Tracer<'a> {
    pub fn new(scene: &'a Scene, max_depth: u32) -> Tracer<'a> {
        Tracer {
            scene,
            max_depth,
            rays: 0,
        }
    }

    /// Fracción de luz entre `origin` y la luz en dirección `dir` hasta `t_max`.
    #[inline]
    fn shadow(&mut self, origin: Vec3, dir: Vec3, t_max: f32, target: Option<[i32; 3]>) -> f32 {
        self.rays += 1;
        lighting::transmission(
            &self.scene.grid,
            &self.scene.materials,
            &Ray::new(origin, dir),
            t_max,
            target,
        )
    }

    /// Luz directa: ambiente + sol + las luces puntuales más cercanas, con sombras.
    fn direct_light(&mut self, hit: &Hit, n: Vec3, view: Vec3, base: Vec3) -> Vec3 {
        let scene = self.scene;
        let mat = scene.material(hit.material);
        let lights = &scene.lights;
        let origin = hit.pos + hit.normal * SHADOW_EPS;
        let mut c = lights.ambient * base;
        // Los metales (muy reflectivos y opacos) tienen brillo especular de su color.
        let spec_color = if mat.reflectivity >= 0.5 && !mat.is_transparent() {
            base / base.max_component().max(1e-3)
        } else {
            Vec3::ONE
        };

        let sun = lights.sun;
        if hit.normal.dot(sun.dir) > 0.0 {
            let (diff, spec) = shading::blinn_phong(n, sun.dir, view, mat.shininess);
            if diff > 0.0 {
                let vis = self.shadow(origin, sun.dir, f32::INFINITY, None);
                if vis > 0.0 {
                    c += sun.color * vis * (base * diff + spec_color * (mat.specular * spec));
                }
            }
        }

        let mut near = [(0usize, 0f32); MAX_POINT_LIGHTS];
        let count = lights.nearest(hit.pos, &mut near);
        for &(i, d2) in &near[..count] {
            let light = &lights.points[i];
            let att = lighting::attenuation(d2, lights.k);
            if att * light.color.max_component() < MIN_CONTRIBUTION {
                continue;
            }
            let dist = d2.sqrt();
            let l = (light.pos - hit.pos) / dist;
            if hit.normal.dot(l) <= 0.0 {
                continue;
            }
            let (diff, spec) = shading::blinn_phong(n, l, view, mat.shininess);
            if diff <= 0.0 {
                continue;
            }
            let vis = self.shadow(origin, l, dist, Some(light.cell));
            if vis > 0.0 {
                c += light.color * (att * vis) * (base * diff + spec_color * (mat.specular * spec));
            }
        }
        c
    }

    /// Color que llega por `ray`. `depth` es la profundidad de recursión (0 = primario) y
    /// `weight` la contribución acumulada de este rayo al píxel.
    pub fn trace(&mut self, ray: &Ray, depth: u32, weight: f32) -> Vec3 {
        self.rays += 1;
        let scene = self.scene;
        let Some(hit) = dda::trace(&scene.grid, ray, 0.0, f32::INFINITY) else {
            return scene.sky(ray.dir);
        };
        let c = self.shade(ray, &hit, depth, weight);
        if hit.from == WATER {
            // Dentro del agua: absorción y dispersión a lo largo del recorrido.
            let t = WATER_ABSORPTION.map(|s| (-s * hit.t).exp());
            c * t + WATER_SCATTER * (Vec3::ONE - t)
        } else if hit.from == AIR && scene.fog_density > 0.0 {
            // Neblina atmosférica: mezcla hacia el color del cielo con la distancia.
            let d = (hit.t - scene.fog_start).max(0.0);
            let f = 1.0 - (-scene.fog_density * d).exp();
            c.lerp(scene.fog_color(ray.dir), f)
        } else {
            c
        }
    }

    fn shade(&mut self, ray: &Ray, hit: &Hit, depth: u32, weight: f32) -> Vec3 {
        let scene = self.scene;
        let mat = scene.material(hit.material);
        let texel = scene.textures[surface_texture(scene, hit)].sample_nearest(hit.u, hit.v);
        if mat.is_emissive() {
            // El bloque emisivo se ve brillante por sí mismo.
            return texel * mat.albedo * scene.lights.ambient + texel * mat.emission;
        }

        let n = match mat.normal_map {
            Some(id) => {
                let texel = scene.textures[id].sample_nearest(hit.u, hit.v);
                let n = shading::perturb_normal(texel, hit.normal);
                if n.dot(hit.normal) > 0.05 {
                    n
                } else {
                    hit.normal
                }
            }
            None => hit.normal,
        };
        let view = -ray.dir;
        let base = texel * mat.albedo;
        let local = self.direct_light(hit, n, view, base);
        let can_recurse = depth < self.max_depth;

        if mat.is_transparent() {
            // Interfaz entre dos medios: Fresnel reparte entre reflexión y refracción.
            let n1 = scene.material(hit.from).ior;
            let n2 = scene.material(hit.to).ior;
            let cos_i = view.dot(n).max(0.0);
            let fresnel = shading::fresnel_schlick(cos_i, n1, n2);
            let kr = mat.reflectivity + mat.transparency * fresnel;
            let kt = mat.transparency * (1.0 - fresnel);
            let kl = (1.0 - mat.reflectivity - mat.transparency).max(0.0);
            let mut c = local * kl;

            if can_recurse && weight * kr > MIN_CONTRIBUTION {
                c += self.reflection(ray, hit, n, depth, weight * kr) * kr;
            }
            if can_recurse && weight * kt > MIN_CONTRIBUTION {
                if let Some(dir) = shading::refract(ray.dir, n, n1 / n2) {
                    let origin = hit.pos - hit.normal * SHADOW_EPS;
                    let tint = Vec3::ONE.lerp(texel, 0.35);
                    c += self.trace(&Ray::new(origin, dir), depth + 1, weight * kt) * tint * kt;
                }
            }
            return c;
        }

        let kr = mat.reflectivity;
        if kr <= 0.0 {
            return local;
        }
        let mut c = local * (1.0 - kr);
        if can_recurse && weight * kr > MIN_CONTRIBUTION {
            // Los reflejos de un metal se tiñen con su color.
            let tint = texel / texel.max_component().max(1e-3);
            c += self.reflection(ray, hit, n, depth, weight * kr) * tint * kr;
        } else {
            c += base * scene.lights.ambient * kr;
        }
        c
    }

    fn reflection(&mut self, ray: &Ray, hit: &Hit, n: Vec3, depth: u32, weight: f32) -> Vec3 {
        let mut dir = shading::reflect(ray.dir, n);
        if dir.dot(hit.normal) <= 0.0 {
            // Con normales perturbadas el reflejo podría meterse en la superficie.
            dir = shading::reflect(ray.dir, hit.normal);
        }
        let origin = hit.pos + hit.normal * SHADOW_EPS;
        self.trace(&Ray::new(origin, dir), depth + 1, weight)
    }
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
    use crate::material::{GLOWSTONE, GOLD, STONE, WATER};
    use crate::texture_gen::AssetSource;

    fn demo() -> Scene {
        Scene::demo(&AssetSource::Generated).unwrap()
    }

    fn demo_camera() -> Camera {
        Camera::new(Vec3::new(6.0, 2.0, 6.0), 2.4, 0.55, 20.0)
    }

    fn probe(scene: &Scene, from: Vec3, to: Vec3) -> Vec3 {
        Tracer::new(scene, 4).trace(&Ray::new(from, (to - from).normalized()), 0, 1.0)
    }

    #[test]
    fn demo_scene_renders_without_nan() {
        let scene = demo();
        let frame = demo_camera().frame(32, 18);
        let mut tracer = Tracer::new(&scene, 4);
        for y in 0..18 {
            for x in 0..32 {
                let c = tracer.trace(&frame.ray(x as f32 + 0.5, y as f32 + 0.5), 0, 1.0);
                assert!(c.is_finite() && c.min_component() >= 0.0);
            }
        }
        assert!(tracer.rays > 32 * 18);
    }

    #[test]
    fn shadowed_point_is_darker() {
        let scene = demo();
        let sun = scene.lights.sun.dir;
        // Un punto del piso "detrás" del pilar de piedra respecto al sol está en sombra.
        let s = Vec3::new(2.5, 2.0, 2.5) - sun * 1.8;
        let shadowed = Vec3::new(s.x, 2.0, s.z);
        let lit = Vec3::new(6.5, 2.0, 1.5);
        let lum = |p: Vec3| probe(&scene, p + Vec3::new(2.0, 5.0, 2.0), p).luminance();
        assert!(lum(shadowed) < lum(lit));
    }

    #[test]
    fn emissive_block_is_bright_and_registered_as_light() {
        let scene = demo();
        assert_eq!(scene.grid.get(10, 3, 8), GLOWSTONE);
        assert_eq!(scene.lights.points.len(), 1);
        let target = Vec3::new(10.5, 3.5, 8.5);
        let c = probe(&scene, target + Vec3::new(0.0, 0.0, 4.0), target);
        assert!(c.max_component() > 0.8);
        // El suelo junto al farol recibe más luz que el mismo suelo sin la luz puntual.
        let floor = Vec3::new(10.5, 2.0, 10.5);
        let from = floor + Vec3::new(0.3, 3.0, 2.0);
        let with = probe(&scene, from, floor);
        let mut dark = demo();
        dark.lights.points.clear();
        let without = probe(&dark, from, floor);
        assert!(with.luminance() > without.luminance());
    }

    #[test]
    fn misses_show_the_skybox() {
        let scene = demo();
        let dir = Vec3::new(0.3, 0.6, 0.2).normalized();
        let c = probe(
            &scene,
            Vec3::new(6.0, 30.0, 6.0),
            Vec3::new(6.0, 30.0, 6.0) + dir,
        );
        assert!((c - scene.sky(dir)).length() < 1e-6);
    }

    #[test]
    fn gold_reflects_and_water_refracts() {
        let scene = demo();
        // El oro refleja el cielo: con recursión se ve distinto que sin ella.
        let gold = Vec3::new(5.5, 2.5, 3.0);
        let from = gold + Vec3::new(0.0, 1.0, -4.0);
        let dir = (gold - from).normalized();
        let with = Tracer::new(&scene, 4).trace(&Ray::new(from, dir), 0, 1.0);
        let without = Tracer::new(&scene, 0).trace(&Ray::new(from, dir), 0, 1.0);
        assert_eq!(scene.grid.get(5, 2, 3), GOLD);
        assert!((with - without).length() > 0.01);
        // A través del agua se ve el fondo (arena), con más rayos que un bloque opaco.
        let water = Vec3::new(7.5, 2.0, 7.5);
        let mut t = Tracer::new(&scene, 4);
        let c = t.trace(
            &Ray::new(
                water + Vec3::new(0.0, 4.0, 1.0),
                (water - (water + Vec3::new(0.0, 4.0, 1.0))).normalized(),
            ),
            0,
            1.0,
        );
        assert!(c.is_finite());
        assert_eq!(scene.grid.get(7, 1, 7), WATER);
        assert!(t.rays >= 3, "reflexión + refracción: {} rayos", t.rays);
    }

    #[test]
    fn normal_map_changes_stone_shading() {
        let scene = demo();
        let mut flat = demo();
        flat.materials[STONE as usize].normal_map = None;
        // Recorre la cara del pilar de piedra iluminada por el sol y compara.
        let mut differs = 0;
        for i in 0..16 {
            let p = Vec3::new(2.0 + (i as f32 + 0.5) / 16.0, 2.5, 2.0);
            let from = p + Vec3::new(0.0, 0.5, -3.0);
            if (probe(&scene, from, p) - probe(&flat, from, p)).length() > 1e-3 {
                differs += 1;
            }
        }
        assert!(
            differs > 4,
            "solo {differs} texels cambian con el normal map"
        );
    }

    #[test]
    fn id_buffer_sees_materials() {
        let scene = demo();
        let ids = material_id_buffer(&scene, &demo_camera(), 64, 36);
        assert!(ids.contains(&STONE));
        assert!(ids.contains(&GRASS));
        assert!(ids.contains(&AIR));
    }
}

//! Pruebas de extremo a extremo del render.

use diorama::animation::{Animation, DEFAULT_FRAMES};
use diorama::camera::Camera;
use diorama::image_io::bmp;
use diorama::material::MAIN_MATERIALS;
use diorama::parallel;
use diorama::renderer::{self, RenderSettings, Tracer};
use diorama::scene::{Scene, SceneConfig};
use diorama::world::AIR;

fn island() -> Scene {
    Scene::island(&SceneConfig {
        sky_size: 32,
        ..SceneConfig::default()
    })
    .unwrap()
}

/// Cámaras de tres momentos de la animación: dos de la órbita y el punto más cercano del zoom.
fn animation_cameras(scene: &Scene) -> Vec<Camera> {
    let anim = Animation::new(DEFAULT_FRAMES, &scene.landmarks);
    [0, 96, 240].iter().map(|&f| anim.camera(f)).collect()
}

#[test]
fn all_main_materials_are_visible_to_primary_rays() {
    let scene = island();
    let mut seen = [false; 256];
    for cam in animation_cameras(&scene) {
        for id in renderer::material_id_buffer(&scene, &cam, 160, 90) {
            seen[id as usize] = true;
        }
    }
    for m in MAIN_MATERIALS {
        assert!(seen[m as usize], "{} no se ve", scene.material(m).name);
    }
}

#[test]
fn skybox_is_visible_in_orbit_frames() {
    let scene = island();
    let anim = Animation::new(DEFAULT_FRAMES, &scene.landmarks);
    for f in [0, 48, 96, 144] {
        let ids = renderer::material_id_buffer(&scene, &anim.camera(f), 96, 54);
        let sky = ids.iter().filter(|&&m| m == AIR).count() as f32 / ids.len() as f32;
        assert!(sky > 0.25, "frame {f}: solo {:.0} % de cielo", sky * 100.0);
    }
}

#[test]
fn render_is_identical_with_1_and_4_threads() {
    let scene = island();
    let cam = animation_cameras(&scene).remove(1);
    let settings = RenderSettings {
        width: 64,
        height: 36,
        spp: 2,
        max_depth: 4,
        exposure: 1.0,
    };
    let (one, _) = parallel::render_frame(&scene, &cam, &settings, 5, 1);
    let (four, _) = parallel::render_frame(&scene, &cam, &settings, 5, 4);
    assert_eq!(bmp::encode_bmp(&one), bmp::encode_bmp(&four));
}

#[test]
fn smoke_preview_frame_has_no_nan() {
    let scene = island();
    let settings = RenderSettings {
        width: 80,
        height: 45,
        spp: 1,
        max_depth: 3,
        exposure: 1.0,
    };
    for cam in animation_cameras(&scene) {
        let frame = cam.frame(settings.width, settings.height);
        let mut tracer = Tracer::new(&scene, settings.max_depth);
        for y in 0..settings.height {
            for x in 0..settings.width {
                let c = parallel::render_pixel(&mut tracer, &frame, 1, 0, x, y);
                assert!(
                    c.is_finite() && c.min_component() >= 0.0,
                    "NaN en ({x}, {y})"
                );
            }
        }
        let (img, stats) = parallel::render_frame(&scene, &cam, &settings, 0, 2);
        assert_eq!(img.pixels.len(), settings.width * settings.height);
        assert!(stats.rays > (settings.width * settings.height) as u64);
    }
}

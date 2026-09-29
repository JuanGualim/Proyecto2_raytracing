//! Pruebas de extremo a extremo del render.

use diorama::camera::Camera;
use diorama::image_io::bmp;
use diorama::material::MAIN_MATERIALS;
use diorama::parallel;
use diorama::renderer::{self, RenderSettings};
use diorama::scene::{Scene, SceneConfig};
use diorama::world::AIR;

fn island() -> Scene {
    Scene::island(&SceneConfig {
        sky_size: 32,
        ..SceneConfig::default()
    })
    .unwrap()
}

/// Cámaras de órbita alrededor de la isla (tres ángulos distintos).
fn orbit_cameras(scene: &Scene) -> Vec<Camera> {
    [0.0f32, 120.0, 240.0]
        .iter()
        .map(|&yaw| {
            Camera::new(
                scene.landmarks.island_center,
                yaw.to_radians(),
                24f32.to_radians(),
                38.0,
            )
        })
        .collect()
}

#[test]
fn all_main_materials_are_visible_to_primary_rays() {
    let scene = island();
    let mut seen = [false; 256];
    for cam in orbit_cameras(&scene) {
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
    for cam in orbit_cameras(&scene) {
        let ids = renderer::material_id_buffer(&scene, &cam, 96, 54);
        let sky = ids.iter().filter(|&&m| m == AIR).count() as f32 / ids.len() as f32;
        assert!(sky > 0.25, "solo {:.0} % de cielo", sky * 100.0);
    }
}

#[test]
fn render_is_identical_with_1_and_4_threads() {
    let scene = island();
    let cam = orbit_cameras(&scene).remove(1);
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

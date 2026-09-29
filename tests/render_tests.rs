//! Pruebas de extremo a extremo del render.

use diorama::camera::Camera;
use diorama::math::Vec3;
use diorama::parallel;
use diorama::renderer::RenderSettings;
use diorama::scene::Scene;
use diorama::texture_gen::AssetSource;

fn small_settings() -> RenderSettings {
    RenderSettings {
        width: 64,
        height: 36,
        spp: 2,
        max_depth: 4,
        exposure: 1.0,
    }
}

#[test]
fn render_is_identical_with_1_and_4_threads() {
    let scene = Scene::demo(&AssetSource::Generated).unwrap();
    let cam = Camera::new(Vec3::new(6.0, 2.0, 6.0), 2.4, 0.45, 19.0);
    let settings = small_settings();
    let (one, _) = parallel::render_frame(&scene, &cam, &settings, 5, 1);
    let (four, _) = parallel::render_frame(&scene, &cam, &settings, 5, 4);
    let a = diorama::image_io::bmp::encode_bmp(&one);
    let b = diorama::image_io::bmp::encode_bmp(&four);
    assert_eq!(a, b, "las imágenes difieren byte a byte");
}

use diorama::camera::Camera;
use diorama::image_io;
use diorama::math::Vec3;
use diorama::renderer::{self, RenderSettings};
use diorama::scene::Scene;
use diorama::texture_gen::AssetSource;
use std::path::{Path, PathBuf};

fn main() -> std::io::Result<()> {
    let scene = Scene::demo(&AssetSource::Directory(PathBuf::from("assets")))?;
    let cam = Camera::new(Vec3::new(6.0, 2.0, 6.0), 2.4, 0.4, 19.0);
    let settings = RenderSettings {
        max_depth: 4,
        ..RenderSettings::default()
    };
    let img = renderer::render(&scene, &cam, &settings);
    image_io::save(Path::new("out/fase4_reflejos.bmp"), &img)?;
    println!("Escrito out/fase4_reflejos.bmp");
    Ok(())
}

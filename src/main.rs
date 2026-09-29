use diorama::camera::Camera;
use diorama::image_io;
use diorama::math::Vec3;
use diorama::renderer::{self, RenderSettings};
use diorama::scene::Scene;
use diorama::texture_gen::AssetSource;
use std::path::{Path, PathBuf};

fn main() -> std::io::Result<()> {
    let scene = Scene::demo(&AssetSource::Directory(PathBuf::from("assets")))?;
    let cam = Camera::new(Vec3::new(3.0, 2.5, 3.0), 3.6, 0.25, 9.0);
    let settings = RenderSettings {
        max_depth: 4,
        ..RenderSettings::default()
    };
    let img = renderer::render(&scene, &cam, &settings);
    image_io::save(Path::new("out/fase5_normal_map.bmp"), &img)?;
    println!("Escrito out/fase5_normal_map.bmp");
    Ok(())
}

use diorama::camera::Camera;
use diorama::image_io;
use diorama::math::Vec3;
use diorama::renderer::{self, RenderSettings};
use diorama::scene::Scene;
use diorama::texture_gen::AssetSource;
use std::path::{Path, PathBuf};

fn main() -> std::io::Result<()> {
    let scene = Scene::demo(&AssetSource::Directory(PathBuf::from("assets/textures")))?;
    let cam = Camera::new(Vec3::new(6.0, 2.0, 6.0), 2.4, 0.5, 19.0);
    let img = renderer::render(&scene, &cam, &RenderSettings::default());
    image_io::save(Path::new("out/fase3_materiales.bmp"), &img)?;
    println!("Escrito out/fase3_materiales.bmp");
    Ok(())
}

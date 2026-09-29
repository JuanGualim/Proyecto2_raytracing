use diorama::camera::Camera;
use diorama::image_io;
use diorama::math::Vec3;
use diorama::renderer;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let grid = renderer::demo_grid();
    let cam = Camera::new(Vec3::new(6.0, 1.0, 6.0), 0.7, 0.55, 22.0);
    let img = renderer::render_grid(&grid, &cam, 640, 360);
    image_io::save(Path::new("out/fase2_cubos.bmp"), &img)?;
    println!("Escrito out/fase2_cubos.bmp");
    Ok(())
}

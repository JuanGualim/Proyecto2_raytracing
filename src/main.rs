use diorama::image_io;
use diorama::renderer;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let img = renderer::render_flat_cube(640, 360);
    image_io::save(Path::new("out/fase1_cubo.bmp"), &img)?;
    image_io::save(Path::new("out/fase1_cubo.ppm"), &img)?;
    println!("Escrito out/fase1_cubo.bmp y out/fase1_cubo.ppm");
    Ok(())
}

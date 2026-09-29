//! Escritura (y lectura mínima, para tests) de PPM binario P6.

use super::Image;
use std::io;
use std::path::Path;

pub fn encode_ppm(img: &Image) -> Vec<u8> {
    let header = format!("P6\n{} {}\n255\n", img.width, img.height);
    let mut out = Vec::with_capacity(header.len() + img.pixels.len() * 3);
    out.extend_from_slice(header.as_bytes());
    for p in &img.pixels {
        out.extend_from_slice(p);
    }
    out
}

pub fn write_ppm(path: &Path, img: &Image) -> io::Result<()> {
    std::fs::write(path, encode_ppm(img))
}

/// Lee un PPM P6 de 8 bits (sin comentarios en la cabecera).
pub fn decode_ppm(bytes: &[u8]) -> Result<Image, String> {
    let mut fields = Vec::new();
    let mut pos = 0;
    while fields.len() < 4 {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let start = pos;
        while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if start == pos {
            return Err("cabecera PPM incompleta".into());
        }
        fields.push(String::from_utf8_lossy(&bytes[start..pos]).into_owned());
    }
    pos += 1; // un único espacio en blanco antes de los datos
    if fields[0] != "P6" || fields[3] != "255" {
        return Err("solo se soporta P6 de 8 bits".into());
    }
    let w: usize = fields[1].parse().map_err(|_| "ancho inválido")?;
    let h: usize = fields[2].parse().map_err(|_| "alto inválido")?;
    let data = bytes.get(pos..pos + w * h * 3).ok_or("datos incompletos")?;
    Ok(Image {
        width: w,
        height: h,
        pixels: data.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let img = Image::from_fn(5, 3, |x, y| [x as u8 * 40, y as u8 * 80, 200]);
        let bytes = encode_ppm(&img);
        assert!(bytes.starts_with(b"P6\n5 3\n255\n"));
        assert_eq!(decode_ppm(&bytes).unwrap(), img);
    }

    #[test]
    fn rejects_other_formats() {
        assert!(decode_ppm(b"P3\n1 1\n255\n0 0 0").is_err());
        assert!(decode_ppm(b"P6\n2 2\n255\n\x00").is_err());
    }
}

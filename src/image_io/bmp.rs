//! Lectura y escritura de BMP sin compresión.
//!
//! Escritura: 24 bits, filas de abajo hacia arriba, cada fila rellenada a múltiplo de 4 bytes.
//! Lectura: 24 o 32 bits sin compresión, de abajo arriba o de arriba abajo (altura negativa).

use super::Image;
use std::io;
use std::path::Path;

const FILE_HEADER: usize = 14;
const INFO_HEADER: usize = 40;

/// Bytes por fila con el relleno a múltiplo de 4.
#[inline]
fn row_stride(width: usize, bytes_per_pixel: usize) -> usize {
    (width * bytes_per_pixel).div_ceil(4) * 4
}

/// Codifica la imagen como BMP de 24 bits.
pub fn encode_bmp(img: &Image) -> Vec<u8> {
    let stride = row_stride(img.width, 3);
    let data_size = stride * img.height;
    let offset = FILE_HEADER + INFO_HEADER;
    let file_size = offset + data_size;
    let mut out = Vec::with_capacity(file_size);

    // BITMAPFILEHEADER
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    // BITMAPINFOHEADER
    out.extend_from_slice(&(INFO_HEADER as u32).to_le_bytes());
    out.extend_from_slice(&(img.width as i32).to_le_bytes());
    out.extend_from_slice(&(img.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // planos
    out.extend_from_slice(&24u16.to_le_bytes()); // bits por píxel
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&(data_size as u32).to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes()); // 72 DPI
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    let padding = stride - img.width * 3;
    for y in (0..img.height).rev() {
        let row = &img.pixels[y * img.width..(y + 1) * img.width];
        for p in row {
            out.extend_from_slice(&[p[2], p[1], p[0]]);
        }
        out.extend(std::iter::repeat_n(0u8, padding));
    }
    out
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn i32_at(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Decodifica un BMP sin compresión de 24 o 32 bits.
pub fn decode_bmp(bytes: &[u8]) -> Result<Image, String> {
    if bytes.len() < FILE_HEADER + INFO_HEADER || &bytes[0..2] != b"BM" {
        return Err("no es un archivo BMP".into());
    }
    let offset = u32_at(bytes, 10) as usize;
    let dib_size = u32_at(bytes, 14) as usize;
    if dib_size < INFO_HEADER {
        return Err(format!("cabecera DIB no soportada ({dib_size} bytes)"));
    }
    let width = i32_at(bytes, 18);
    let raw_height = i32_at(bytes, 22);
    let bpp = u16_at(bytes, 28) as usize;
    let compression = u32_at(bytes, 30);
    if width <= 0 || raw_height == 0 {
        return Err("dimensiones inválidas".into());
    }
    // 0 = BI_RGB; 3 = BI_BITFIELDS (se asume el orden BGRA estándar en 32 bits).
    if !(bpp == 24 && compression == 0 || bpp == 32 && (compression == 0 || compression == 3)) {
        return Err(format!(
            "formato no soportado: {bpp} bits, compresión {compression}"
        ));
    }
    let width = width as usize;
    let top_down = raw_height < 0;
    let height = raw_height.unsigned_abs() as usize;
    let bytes_pp = bpp / 8;
    let stride = row_stride(width, bytes_pp);
    if offset + stride * (height - 1) + width * bytes_pp > bytes.len() {
        return Err("datos de píxeles incompletos".into());
    }

    let mut img = Image::new(width, height);
    for row in 0..height {
        let y = if top_down { row } else { height - 1 - row };
        let start = offset + row * stride;
        for x in 0..width {
            let p = start + x * bytes_pp;
            img.set(x, y, [bytes[p + 2], bytes[p + 1], bytes[p]]);
        }
    }
    Ok(img)
}

pub fn write_bmp(path: &Path, img: &Image) -> io::Result<()> {
    std::fs::write(path, encode_bmp(img))
}

pub fn read_bmp(path: &Path) -> io::Result<Image> {
    let bytes = std::fs::read(path)?;
    decode_bmp(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(w: usize, h: usize) -> Image {
        Image::from_fn(w, h, |x, y| {
            [
                (x * 37 + y * 11) as u8,
                (x * 5 + y * 91) as u8,
                (x ^ y) as u8,
            ]
        })
    }

    #[test]
    fn roundtrip_identical_pixels_with_row_padding() {
        // Anchos 1, 2, 3, 5 y 7 obligan a rellenar las filas; 4 y 8 no.
        for (w, h) in [
            (1, 1),
            (2, 3),
            (3, 2),
            (4, 4),
            (5, 7),
            (7, 5),
            (8, 3),
            (17, 9),
        ] {
            let img = pattern(w, h);
            let bytes = encode_bmp(&img);
            assert_eq!(bytes.len(), 54 + row_stride(w, 3) * h);
            assert_eq!((bytes.len() - 54) % 4, 0);
            let back = decode_bmp(&bytes).unwrap();
            assert_eq!(back, img, "falló el ida y vuelta en {w}x{h}");
        }
    }

    #[test]
    fn roundtrip_through_file() {
        let dir = std::env::temp_dir().join(format!("diorama_bmp_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.bmp");
        let img = pattern(13, 6);
        write_bmp(&path, &img).unwrap();
        assert_eq!(read_bmp(&path).unwrap(), img);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reads_top_down_32_bit() {
        // BMP de 2×2, 32 bits, de arriba abajo (altura negativa).
        let mut b = Vec::new();
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&(54u32 + 16).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&54u32.to_le_bytes());
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&2i32.to_le_bytes());
        b.extend_from_slice(&(-2i32).to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&32u16.to_le_bytes());
        b.extend_from_slice(&[0u8; 24]);
        // Fila superior: rojo, verde. Fila inferior: azul, blanco (BGRA).
        b.extend_from_slice(&[0, 0, 255, 255, 0, 255, 0, 255]);
        b.extend_from_slice(&[255, 0, 0, 255, 255, 255, 255, 255]);
        let img = decode_bmp(&b).unwrap();
        assert_eq!(img.get(0, 0), [255, 0, 0]);
        assert_eq!(img.get(1, 0), [0, 255, 0]);
        assert_eq!(img.get(0, 1), [0, 0, 255]);
        assert_eq!(img.get(1, 1), [255, 255, 255]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_bmp(b"hola").is_err());
        let mut bytes = encode_bmp(&pattern(4, 4));
        bytes.truncate(60);
        assert!(decode_bmp(&bytes).is_err());
    }
}

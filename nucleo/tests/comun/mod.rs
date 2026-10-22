//! Utilidades compartidas por las pruebas: imágenes sintéticas, codificación
//! con EXIF o ICC y comparación de píxeles.
#![allow(dead_code)]

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, RgbImage, RgbaImage};

/// Generador pseudoaleatorio determinista (xorshift), para no depender de `rand`.
pub struct Azar(u64);

impl Azar {
    pub fn new(semilla: u64) -> Self {
        Self(semilla.max(1))
    }
    pub fn siguiente(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn byte(&mut self) -> u8 {
        (self.siguiente() >> 24) as u8
    }
}

/// Imagen con degradados, bordes y ruido: ejercita todos los valores de 0 a 255.
pub fn imagen_prueba(ancho: u32, alto: u32, semilla: u64) -> RgbImage {
    let mut azar = Azar::new(semilla);
    RgbImage::from_fn(ancho, alto, |x, y| {
        let r = (x * 255 / ancho.max(2).saturating_sub(1).max(1)) as u8;
        let g = (y * 255 / alto.max(2).saturating_sub(1).max(1)) as u8;
        let b = if (x / 16 + y / 16) % 2 == 0 { azar.byte() } else { ((x + y) % 256) as u8 };
        image::Rgb([r, g, b])
    })
}

/// Imagen de 256×256 que contiene cada valor de 0 a 255 en cada canal.
pub fn todos_los_valores() -> RgbImage {
    RgbImage::from_fn(256, 256, |x, y| image::Rgb([x as u8, y as u8, ((x + 255 - y) % 256) as u8]))
}

pub fn jpeg(imagen: &RgbImage, calidad: u8) -> Vec<u8> {
    jpeg_con(imagen, calidad, None, None)
}

pub fn jpeg_con(imagen: &RgbImage, calidad: u8, exif: Option<Vec<u8>>, icc: Option<Vec<u8>>) -> Vec<u8> {
    let mut salida = Vec::new();
    let mut codificador = JpegEncoder::new_with_quality(&mut salida, calidad);
    if let Some(exif) = exif {
        codificador.set_exif_metadata(exif).unwrap();
    }
    if let Some(icc) = icc {
        codificador.set_icc_profile(icc).unwrap();
    }
    codificador
        .write_image(imagen.as_raw(), imagen.width(), imagen.height(), ExtendedColorType::Rgb8)
        .unwrap();
    salida
}

pub fn png_rgba(imagen: &RgbaImage) -> Vec<u8> {
    let mut salida = Vec::new();
    PngEncoder::new(&mut salida)
        .write_image(imagen.as_raw(), imagen.width(), imagen.height(), ExtendedColorType::Rgba8)
        .unwrap();
    salida
}

/// Bloque EXIF mínimo (TIFF big-endian) con solo la etiqueta de orientación.
pub fn exif_orientacion(orientacion: u16) -> Vec<u8> {
    let mut t = Vec::new();
    t.extend_from_slice(b"MM\x00\x2A");
    t.extend_from_slice(&8u32.to_be_bytes()); // IFD0 empieza en el byte 8
    t.extend_from_slice(&1u16.to_be_bytes()); // una entrada
    t.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
    t.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    t.extend_from_slice(&1u32.to_be_bytes()); // un valor
    t.extend_from_slice(&orientacion.to_be_bytes());
    t.extend_from_slice(&[0, 0]); // relleno del campo de 4 bytes
    t.extend_from_slice(&0u32.to_be_bytes()); // no hay más IFD
    t
}

/// Mayor diferencia absoluta entre dos imágenes del mismo tamaño.
pub fn diferencia_maxima(a: &RgbImage, b: &RgbImage) -> u8 {
    assert_eq!(a.dimensions(), b.dimensions(), "las imágenes no miden lo mismo");
    a.as_raw().iter().zip(b.as_raw()).map(|(p, q)| p.abs_diff(*q)).max().unwrap_or(0)
}

/// Color medio de un rectángulo.
pub fn color_medio(imagen: &RgbImage, x: u32, y: u32, ancho: u32, alto: u32) -> [f64; 3] {
    let mut suma = [0f64; 3];
    for j in y..y + alto {
        for i in x..x + ancho {
            let p = imagen.get_pixel(i, j);
            for c in 0..3 {
                suma[c] += f64::from(p[c]);
            }
        }
    }
    let n = f64::from(ancho * alto);
    suma.map(|s| s / n)
}

//! Utilidades compartidas por las pruebas: imágenes sintéticas, codificación
//! con EXIF o ICC y comparación de píxeles.
#![allow(dead_code)]

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage, RgbaImage};

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

/// Escena con aspecto de foto: degradados, formas con bordes nítidos, textura
/// fina y algo de ruido. Más realista que `imagen_prueba` para medir calidad.
pub fn foto_sintetica(ancho: u32, alto: u32, semilla: u64) -> RgbImage {
    let mut azar = Azar::new(semilla);
    let (w, h) = (ancho as f32, alto as f32);
    RgbImage::from_fn(ancho, alto, |x, y| {
        let (fx, fy) = (x as f32 / w, y as f32 / h);
        let mut c = [60.0 + 150.0 * fy, 110.0 + 90.0 * fx, 200.0 - 120.0 * fy];
        // Círculos y franjas con bordes nítidos.
        let d = ((fx - 0.6).powi(2) + (fy - 0.35).powi(2)).sqrt();
        if d < 0.15 {
            c = [240.0, 200.0, 90.0];
        }
        if (fx * 9.0 + fy * 3.0).fract() < 0.08 && fy > 0.6 {
            c = [40.0, 50.0, 45.0];
        }
        // Textura fina, como pelo, hojas o tela.
        let t = ((fx * w * 0.21).sin() * (fy * h * 0.17).cos()) * 14.0;
        let ruido = f32::from(azar.byte()) / 255.0 * 10.0 - 5.0;
        Rgb(c.map(|v| (v + t + ruido).clamp(0.0, 255.0) as u8))
    })
}

/// SSIM medio sobre la luma (Wang et al. 2004): ventana gaussiana de 11×11 y
/// σ = 1,5, sin contar un borde de 5 px.
pub fn ssim(a: &RgbImage, b: &RgbImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions(), "las imágenes no miden lo mismo");
    let (w, h) = (a.width() as usize, a.height() as usize);
    let luma = |i: &RgbImage| -> Vec<f64> {
        i.pixels().map(|p| 0.2126 * f64::from(p[0]) + 0.7152 * f64::from(p[1]) + 0.0722 * f64::from(p[2])).collect()
    };
    let (x, y) = (luma(a), luma(b));
    let producto = |p: &[f64], q: &[f64]| -> Vec<f64> { p.iter().zip(q).map(|(u, v)| u * v).collect() };

    let nucleo: Vec<f64> = {
        let k: Vec<f64> = (-5..=5).map(|i: i32| (-(f64::from(i * i)) / (2.0 * 1.5 * 1.5)).exp()).collect();
        let s: f64 = k.iter().sum();
        k.into_iter().map(|v| v / s).collect()
    };
    let difuminar = |datos: &[f64]| -> Vec<f64> {
        let mut tmp = vec![0.0; datos.len()];
        for j in 0..h {
            for i in 0..w {
                tmp[j * w + i] = nucleo.iter().enumerate().map(|(k, p)| p * datos[j * w + (i + k).saturating_sub(5).min(w - 1)]).sum();
            }
        }
        let mut out = vec![0.0; datos.len()];
        for j in 0..h {
            for i in 0..w {
                out[j * w + i] = nucleo.iter().enumerate().map(|(k, p)| p * tmp[(j + k).saturating_sub(5).min(h - 1) * w + i]).sum();
            }
        }
        out
    };

    let (mx, my) = (difuminar(&x), difuminar(&y));
    let (xx, yy, xy) = (difuminar(&producto(&x, &x)), difuminar(&producto(&y, &y)), difuminar(&producto(&x, &y)));
    let (c1, c2) = ((0.01f64 * 255.0).powi(2), (0.03f64 * 255.0).powi(2));
    let mut suma = 0.0;
    let mut n = 0usize;
    for j in 5..h.saturating_sub(5) {
        for i in 5..w.saturating_sub(5) {
            let k = j * w + i;
            let (ma, mb) = (mx[k], my[k]);
            let (va, vb, cov) = (xx[k] - ma * ma, yy[k] - mb * mb, xy[k] - ma * mb);
            suma += ((2.0 * ma * mb + c1) * (2.0 * cov + c2)) / ((ma * ma + mb * mb + c1) * (va + vb + c2));
            n += 1;
        }
    }
    suma / n as f64
}

/// Factores de muestreo (H<<4 | V) de cada componente, leídos del marcador SOF.
pub fn muestreo_jpeg(jpeg: &[u8]) -> Vec<u8> {
    let mut i = 2;
    while i + 4 < jpeg.len() {
        assert_eq!(jpeg[i], 0xFF, "marcador JPEG esperado");
        let marcador = jpeg[i + 1];
        let largo = usize::from(u16::from_be_bytes([jpeg[i + 2], jpeg[i + 3]]));
        if matches!(marcador, 0xC0 | 0xC1 | 0xC2) {
            let componentes = usize::from(jpeg[i + 9]);
            return (0..componentes).map(|c| jpeg[i + 10 + c * 3 + 1]).collect();
        }
        i += 2 + largo;
    }
    panic!("no se encontró el marcador SOF");
}

/// Fotos del corpus real (`pruebas/corpus/`), si hay.
pub fn fotos_corpus() -> Vec<std::path::PathBuf> {
    let carpeta = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../pruebas/corpus");
    let mut fotos: Vec<_> = std::fs::read_dir(carpeta)
        .map(|d| d.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default();
    fotos.retain(|p| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp"))
    });
    fotos.sort();
    fotos
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

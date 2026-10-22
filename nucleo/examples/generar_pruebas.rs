//! Genera las imágenes sintéticas de prueba en `pruebas/sinteticas/`.
//! Uso: `npm run sinteticas` (o `cargo run --release -p nucleo --example generar_pruebas`).
//!
//! No sustituyen al corpus de fotos reales: sirven para las pruebas en el
//! navegador (T-06, T-09) y para revisar EXIF, transparencia y perfiles de color.

use std::fs;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage, Rgba, RgbaImage};

fn main() {
    let carpeta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../pruebas/sinteticas");
    fs::create_dir_all(&carpeta).expect("crear carpeta");

    let guardar = |nombre: &str, bytes: Vec<u8>| {
        let ruta = carpeta.join(nombre);
        fs::write(&ruta, &bytes).expect("escribir archivo");
        println!("{:>8} KB  {}", bytes.len() / 1024, ruta.display());
    };

    // T-09: foto de 24 MP (6000×4000).
    guardar("24mp.jpg", jpeg(&escena(6000, 4000, 1), 92, None, None));

    // T-06 / RF-08: foto pequeña de 800 px.
    guardar("pequena_800.jpg", jpeg(&escena(800, 600, 2), 92, None, None));

    // RF-02: retrato guardado de lado, con EXIF orientación 6 (girar 90° a la derecha).
    let retrato = escena(1200, 1600, 3);
    let almacenada = image::imageops::rotate270(&retrato);
    guardar("exif_6_retrato.jpg", jpeg(&almacenada, 92, Some(exif_orientacion(6)), None));

    // Perfil Display P3, como las fotos de iPhone.
    let p3 = moxcms::ColorProfile::new_display_p3().encode().expect("perfil P3");
    guardar("display_p3.jpg", jpeg(&escena(2400, 1600, 4), 92, None, Some(p3)));

    // PNG con transparencia.
    guardar("transparente.png", png_con_alfa(1200, 1200));

    // Paisaje mediano para pruebas rápidas.
    guardar("paisaje_3000.jpg", jpeg(&escena(3000, 2000, 5), 92, None, None));
}

/// Escena con aspecto de foto: cielo, sol, colinas, textura y ruido, y una
/// flecha roja que apunta hacia arriba para ver la orientación a simple vista.
fn escena(ancho: u32, alto: u32, semilla: u64) -> RgbImage {
    let mut azar = semilla.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let (w, h) = (ancho as f32, alto as f32);
    let sol = (w * 0.72, h * 0.28, h.min(w) * 0.09);
    RgbImage::from_fn(ancho, alto, |x, y| {
        let (fx, fy) = (x as f32, y as f32);
        let horizonte = h * (0.62 + 0.06 * (fx / w * 6.0 + semilla as f32).sin());
        let mut c = if fy < horizonte {
            let t = fy / horizonte;
            [70.0 + 120.0 * t, 130.0 + 80.0 * t, 220.0 - 30.0 * t]
        } else {
            let t = (fy - horizonte) / (h - horizonte);
            let surco = ((fx * 0.05 + fy * 0.11).sin() * 0.5 + 0.5) * 30.0;
            [60.0 + 50.0 * t + surco, 120.0 - 30.0 * t + surco * 0.5, 50.0 + 10.0 * t]
        };
        let d = ((fx - sol.0).powi(2) + (fy - sol.1).powi(2)).sqrt();
        if d < sol.2 {
            c = [255.0, 214.0, 120.0];
        } else if d < sol.2 * 2.2 {
            let k = 1.0 - (d - sol.2) / (sol.2 * 1.2);
            c = [c[0] + 80.0 * k, c[1] + 50.0 * k, c[2] - 20.0 * k];
        }
        // Flecha "arriba" en la esquina superior izquierda.
        let (ax, ay, lado) = (w * 0.08, h * 0.06, h.min(w) * 0.12);
        let (dx, dy) = (fx - ax, fy - ay);
        if dy >= 0.0 && dy <= lado && dx.abs() <= dy * 0.5 || (dy > lado && dy < lado * 2.0 && dx.abs() < lado * 0.15) {
            c = [220.0, 30.0, 40.0];
        }
        azar ^= azar << 13;
        azar ^= azar >> 7;
        azar ^= azar << 17;
        let ruido = ((azar >> 40) as f32 / 16_777_216.0 - 0.5) * 18.0;
        Rgb(c.map(|v| (v + ruido).clamp(0.0, 255.0) as u8))
    })
}

fn png_con_alfa(ancho: u32, alto: u32) -> Vec<u8> {
    let img = RgbaImage::from_fn(ancho, alto, |x, y| {
        let (cx, cy) = (ancho as f32 / 2.0, alto as f32 / 2.0);
        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / cx;
        let a = if d < 0.7 { 255 } else if d < 0.9 { ((0.9 - d) / 0.2 * 255.0) as u8 } else { 0 };
        Rgba([225, (60.0 + 120.0 * y as f32 / alto as f32) as u8, 110, a])
    });
    let mut salida = Vec::new();
    PngEncoder::new(&mut salida)
        .write_image(img.as_raw(), ancho, alto, ExtendedColorType::Rgba8)
        .expect("png");
    salida
}

fn jpeg(img: &RgbImage, calidad: u8, exif: Option<Vec<u8>>, icc: Option<Vec<u8>>) -> Vec<u8> {
    let mut salida = Vec::new();
    let mut codificador = JpegEncoder::new_with_quality(&mut salida, calidad);
    if let Some(exif) = exif {
        codificador.set_exif_metadata(exif).expect("exif");
    }
    if let Some(icc) = icc {
        codificador.set_icc_profile(icc).expect("icc");
    }
    codificador
        .write_image(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgb8)
        .expect("jpeg");
    salida
}

/// Bloque EXIF mínimo (TIFF big-endian) con solo la etiqueta de orientación.
fn exif_orientacion(orientacion: u16) -> Vec<u8> {
    let mut t = b"MM\x00\x2A".to_vec();
    t.extend_from_slice(&8u32.to_be_bytes());
    t.extend_from_slice(&1u16.to_be_bytes());
    t.extend_from_slice(&0x0112u16.to_be_bytes());
    t.extend_from_slice(&3u16.to_be_bytes());
    t.extend_from_slice(&1u32.to_be_bytes());
    t.extend_from_slice(&orientacion.to_be_bytes());
    t.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    t
}

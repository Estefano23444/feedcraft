//! Decodificar: leer JPEG, PNG o WebP (RF-01), enderezar según EXIF (RF-02),
//! aplanar la transparencia sobre blanco y convertir a sRGB si la foto trae otro
//! perfil de color. Son los pasos 1 a 3 del orden obligatorio de la sección 6.

use std::io::Cursor;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, RgbImage};
use moxcms::{ColorProfile, Layout, ProfileText, TransformOptions};
use serde::Serialize;

/// RF-01: hasta 50 MB por archivo.
pub const MAX_BYTES: usize = 50 * 1024 * 1024;

/// Tope de píxeles para no agotar la memoria del navegador (100 MP ≈ 300 MB en RGB).
pub const MAX_PIXELES: u64 = 100_000_000;

/// Píxeles por bloque al convertir el perfil de color: evita duplicar la foto en memoria.
const BLOQUE_ICC: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct InfoFoto {
    pub ancho: u32,
    pub alto: u32,
    pub formato: &'static str,
    /// Orientación EXIF leída (1 = normal … 8). Ya está aplicada a los píxeles.
    pub orientacion: u8,
    /// Nombre del perfil de color convertido a sRGB, si la foto traía otro.
    pub perfil_convertido: Option<String>,
    pub tenia_transparencia: bool,
}

pub struct Foto {
    pub imagen: RgbImage,
    pub info: InfoFoto,
}

pub fn decodificar(bytes: &[u8]) -> Result<Foto, String> {
    if bytes.len() > MAX_BYTES {
        return Err(format!(
            "El archivo pesa {:.1} MB; el máximo es 50 MB.",
            bytes.len() as f64 / 1_048_576.0
        ));
    }

    let lector = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("No se pudo leer el archivo: {e}"))?;
    let formato = match lector.format() {
        Some(ImageFormat::Jpeg) => "jpeg",
        Some(ImageFormat::Png) => "png",
        Some(ImageFormat::WebP) => "webp",
        _ => return Err("Formato no soportado: usa JPEG, PNG o WebP.".into()),
    };

    let mut decodificador = lector.into_decoder().map_err(error_imagen)?;
    let (ancho, alto) = decodificador.dimensions();
    if u64::from(ancho) * u64::from(alto) > MAX_PIXELES {
        return Err(format!(
            "La foto mide {ancho}×{alto} px ({:.0} MP); el máximo es 100 MP.",
            f64::from(ancho) * f64::from(alto) / 1e6
        ));
    }
    let orientacion = decodificador.orientation().unwrap_or(Orientation::NoTransforms);
    let icc = decodificador.icc_profile().ok().flatten();

    let mut dinamica = DynamicImage::from_decoder(decodificador).map_err(error_imagen)?;
    dinamica.apply_orientation(orientacion);

    let tenia_transparencia = dinamica.color().has_alpha();
    let mut imagen = aplanar_sobre_blanco(dinamica);
    let perfil_convertido = icc.and_then(|icc| convertir_a_srgb(&mut imagen, &icc));

    Ok(Foto {
        info: InfoFoto {
            ancho: imagen.width(),
            alto: imagen.height(),
            formato,
            orientacion: orientacion.to_exif(),
            perfil_convertido,
            tenia_transparencia,
        },
        imagen,
    })
}

fn error_imagen(e: image::ImageError) -> String {
    format!("La imagen está dañada o no se puede decodificar: {e}")
}

/// JPEG no admite transparencia: un PNG o WebP con alfa se compone sobre blanco.
fn aplanar_sobre_blanco(imagen: DynamicImage) -> RgbImage {
    if !imagen.color().has_alpha() {
        return imagen.into_rgb8();
    }
    let rgba = imagen.into_rgba8();
    let (ancho, alto) = rgba.dimensions();
    let mut rgb = Vec::with_capacity(ancho as usize * alto as usize * 3);
    for p in rgba.pixels() {
        let a = u32::from(p[3]);
        for c in 0..3 {
            rgb.push(((u32::from(p[c]) * a + 255 * (255 - a) + 127) / 255) as u8);
        }
    }
    RgbImage::from_raw(ancho, alto, rgb).expect("el búfer tiene el tamaño exacto")
}

/// Convierte la foto a sRGB si su perfil ICC es otro (por ejemplo Display P3 de
/// un iPhone). Devuelve el nombre del perfil convertido. Un perfil ilegible o
/// incompatible se ignora y la foto se trata como sRGB: nunca impide cargarla.
fn convertir_a_srgb(imagen: &mut RgbImage, icc: &[u8]) -> Option<String> {
    let perfil = ColorProfile::new_from_slice(icc).ok()?;
    let srgb = ColorProfile::new_srgb();
    if mismos_primarios(&perfil, &srgb) {
        return None;
    }
    let transformacion = perfil
        .create_transform_8bit(Layout::Rgb, &srgb, Layout::Rgb, TransformOptions::default())
        .ok()?;

    let mut temporal = vec![0u8; BLOQUE_ICC * 3];
    for trozo in imagen.as_mut().chunks_mut(BLOQUE_ICC * 3) {
        let destino = &mut temporal[..trozo.len()];
        transformacion.transform(trozo, destino).ok()?;
        trozo.copy_from_slice(destino);
    }
    Some(nombre_perfil(&perfil))
}

/// Un perfil con los primarios de sRGB (sRGB IEC61966-2.1, "c2", etc.) no
/// necesita conversión: se ahorra el paso más caro de la carga.
fn mismos_primarios(a: &ColorProfile, b: &ColorProfile) -> bool {
    let cerca = |p: &moxcms::Xyzd, q: &moxcms::Xyzd| {
        (p.x - q.x).abs() < 0.003 && (p.y - q.y).abs() < 0.003 && (p.z - q.z).abs() < 0.003
    };
    a.lut_a_to_b_perceptual.is_none()
        && cerca(&a.red_colorant, &b.red_colorant)
        && cerca(&a.green_colorant, &b.green_colorant)
        && cerca(&a.blue_colorant, &b.blue_colorant)
}

fn nombre_perfil(perfil: &ColorProfile) -> String {
    let texto = match &perfil.description {
        Some(ProfileText::PlainString(s)) => s.clone(),
        Some(ProfileText::Localizable(v)) => v.first().map(|l| l.value.clone()).unwrap_or_default(),
        Some(ProfileText::Description(d)) => d.ascii_string.clone(),
        None => String::new(),
    };
    let texto = texto.trim_matches(char::from(0)).trim();
    if texto.is_empty() { "perfil ICC sin nombre".into() } else { texto.into() }
}

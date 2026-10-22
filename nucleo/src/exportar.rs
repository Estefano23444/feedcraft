//! Exportación: un solo JPEG en sRGB, sin submuestreo de color (4:4:4), con
//! tablas de Huffman optimizadas y el perfil sRGB incrustado (RF-14), y su
//! nombre de archivo (RF-15).

use std::sync::OnceLock;

use image::RgbImage;
use jpeg_encoder::{ColorType, Encoder, SamplingFactor};

use crate::recorte::Formato;

/// La especificación propone 92, pero en fotos nocturnas con ruido la
/// compresión a 92 deja el SSIM por debajo de 0,97 (T-05); a 95 queda ≥ 0,98.
pub const CALIDAD_POR_DEFECTO: u8 = 95;
pub const CALIDAD_MINIMA: u8 = 80;
pub const CALIDAD_MAXIMA: u8 = 100;

/// Perfil ICC sRGB, generado una vez.
fn perfil_srgb() -> &'static [u8] {
    static PERFIL: OnceLock<Vec<u8>> = OnceLock::new();
    PERFIL.get_or_init(|| moxcms::ColorProfile::new_srgb().encode().unwrap_or_default())
}

/// Codifica el JPEG final. La calidad se limita a 80–100.
pub fn codificar_jpeg(imagen: &RgbImage, calidad: u8) -> Result<Vec<u8>, String> {
    let (ancho, alto) = imagen.dimensions();
    let (ancho, alto) = (
        u16::try_from(ancho).map_err(|_| "La imagen es demasiado ancha para JPEG.")?,
        u16::try_from(alto).map_err(|_| "La imagen es demasiado alta para JPEG.")?,
    );
    let mut salida = Vec::with_capacity(imagen.as_raw().len() / 4);
    let mut codificador = Encoder::new(&mut salida, calidad.clamp(CALIDAD_MINIMA, CALIDAD_MAXIMA));
    codificador.set_sampling_factor(SamplingFactor::F_1_1);
    codificador.set_optimized_huffman_tables(true);
    let perfil = perfil_srgb();
    if !perfil.is_empty() {
        codificador
            .add_icc_profile(perfil)
            .map_err(|e| format!("No se pudo incrustar el perfil sRGB: {e}"))?;
    }
    codificador
        .encode(imagen.as_raw(), ancho, alto, ColorType::Rgb)
        .map_err(|e| format!("No se pudo codificar el JPEG: {e}"))?;
    Ok(salida)
}

/// RF-15: `foto_film-calido_4x5.jpg`. El nombre base y el estilo se limpian:
/// minúsculas, sin tildes ni espacios. Sin estilo, el nombre lleva solo el formato.
pub fn nombre_archivo(base: &str, estilo: Option<&str>, formato: Formato) -> String {
    let base = limpiar(base, 40);
    let base = if base.is_empty() { "foto".to_string() } else { base };
    match estilo.map(|e| limpiar(e, 30)).filter(|e| !e.is_empty()) {
        Some(estilo) => format!("{base}_{estilo}_{}.jpg", formato.etiqueta()),
        None => format!("{base}_{}.jpg", formato.etiqueta()),
    }
}

fn limpiar(texto: &str, maximo: usize) -> String {
    let mut salida = String::new();
    let mut guion = false;
    for c in texto.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            salida.push(c);
            guion = false;
        } else if !guion && !salida.is_empty() {
            salida.push('-');
            guion = true;
        }
        if salida.len() >= maximo {
            break;
        }
    }
    salida.trim_end_matches('-').to_string()
}

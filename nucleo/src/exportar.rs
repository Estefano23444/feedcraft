//! Exportación: un solo JPEG en sRGB, sin submuestreo de color (4:4:4) y con
//! tablas de Huffman optimizadas (RF-14).

use image::RgbImage;
use jpeg_encoder::{ColorType, Encoder, SamplingFactor};

pub const CALIDAD_POR_DEFECTO: u8 = 92;

pub fn codificar_jpeg(imagen: &RgbImage, calidad: u8) -> Result<Vec<u8>, String> {
    let (ancho, alto) = imagen.dimensions();
    let (ancho, alto) = (
        u16::try_from(ancho).map_err(|_| "La imagen es demasiado ancha para JPEG.")?,
        u16::try_from(alto).map_err(|_| "La imagen es demasiado alta para JPEG.")?,
    );
    let mut salida = Vec::with_capacity(imagen.as_raw().len() / 4);
    let mut codificador = Encoder::new(&mut salida, calidad.clamp(80, 100));
    codificador.set_sampling_factor(SamplingFactor::F_1_1);
    codificador.set_optimized_huffman_tables(true);
    codificador
        .encode(imagen.as_raw(), ancho, alto, ColorType::Rgb)
        .map_err(|e| format!("No se pudo codificar el JPEG: {e}"))?;
    Ok(salida)
}

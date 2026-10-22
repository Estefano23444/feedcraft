//! Redimensión con Lanczos3 en un solo paso (RF-12): el recorte y la reducción
//! ocurren en la misma operación, sin copias ni reducciones encadenadas.

use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::RgbImage;

/// Rectángulo en píxeles de la imagen de origen; admite fracciones.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecortePx {
    pub x: f64,
    pub y: f64,
    pub ancho: f64,
    pub alto: f64,
}

/// Lleva `recorte` (o la imagen completa) a `ancho`×`alto` exactos con Lanczos3.
pub fn redimensionar(
    origen: &RgbImage,
    recorte: Option<RecortePx>,
    ancho: u32,
    alto: u32,
) -> Result<RgbImage, String> {
    let fuente = ImageRef::new(origen.width(), origen.height(), origen.as_raw(), PixelType::U8x3)
        .map_err(|e| format!("Imagen de origen no válida: {e}"))?;
    let mut destino = Image::new(ancho.max(1), alto.max(1), PixelType::U8x3);

    let mut opciones = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    if let Some(r) = recorte {
        opciones = opciones.crop(r.x, r.y, r.ancho, r.alto);
    }
    Resizer::new()
        .resize(&fuente, &mut destino, &opciones)
        .map_err(|e| format!("No se pudo redimensionar: {e}"))?;

    Ok(RgbImage::from_raw(destino.width(), destino.height(), destino.into_vec()).expect("tamaño exacto"))
}

/// Medidas que caben dentro de un cuadrado de `lado` px sin cambiar la
/// proporción. Nunca amplía.
pub fn encajar(ancho: u32, alto: u32, lado: u32) -> (u32, u32) {
    let mayor = ancho.max(alto);
    if mayor <= lado {
        return (ancho, alto);
    }
    let escala = f64::from(lado) / f64::from(mayor);
    (
        ((f64::from(ancho) * escala).round() as u32).max(1),
        ((f64::from(alto) * escala).round() as u32).max(1),
    )
}

//! Máscara de enfoque suave (RF-13), aplicada después de reducir: devuelve la
//! nitidez que el promediado de Lanczos suaviza. Radio de ~0,7 px (sección 6:
//! entre 0,5 y 1 px).
//!
//! Actúa solo sobre la luminancia y con un umbral pequeño: realza bordes y
//! texturas sin realzar el ruido de color, que además gastaría bits del JPEG
//! (y de la recompresión de Instagram) sin aportar detalle.

use image::RgbImage;

/// Gaussiana de σ = 0,7 px en 5 coeficientes (suman 1).
const NUCLEO: [f32; 5] = [0.0096, 0.2054, 0.5700, 0.2054, 0.0096];

/// Diferencias de luminancia menores que esto (de 0 a 255) no se realzan.
const UMBRAL: f32 = 3.0;

/// Intensidad por defecto: suave.
pub const ENFOQUE_POR_DEFECTO: f32 = 30.0;

/// `intensidad` va de 0 (sin enfoque) a 100.
pub fn enfocar(imagen: &mut RgbImage, intensidad: f32) {
    let intensidad = if intensidad.is_finite() { intensidad.clamp(0.0, 100.0) } else { 0.0 };
    if intensidad == 0.0 {
        return;
    }
    let cantidad = intensidad / 100.0 * 1.5;
    let (ancho, alto) = (imagen.width() as usize, imagen.height() as usize);

    let luma: Vec<f32> = imagen
        .pixels()
        .map(|p| 0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2]))
        .collect();

    // Desenfoque separable: horizontal y luego vertical, con bordes repetidos.
    let mut horizontal = vec![0f32; luma.len()];
    for y in 0..alto {
        let fila = &luma[y * ancho..(y + 1) * ancho];
        for x in 0..ancho {
            horizontal[y * ancho + x] = NUCLEO
                .iter()
                .enumerate()
                .map(|(k, peso)| peso * fila[(x + k).saturating_sub(2).min(ancho - 1)])
                .sum();
        }
    }

    let datos = &mut **imagen;
    for y in 0..alto {
        for x in 0..ancho {
            let borrosa: f32 = NUCLEO
                .iter()
                .enumerate()
                .map(|(k, peso)| peso * horizontal[(y + k).saturating_sub(2).min(alto - 1) * ancho + x])
                .sum();
            let detalle = luma[y * ancho + x] - borrosa;
            if detalle.abs() > UMBRAL {
                // El mismo realce en los tres canales: cambia la luminancia, no el color.
                let d = detalle * cantidad + 0.5;
                let i = (y * ancho + x) * 3;
                for c in &mut datos[i..i + 3] {
                    *c = (f32::from(*c) + d) as u8;
                }
            }
        }
    }
}

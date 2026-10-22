//! Máscara de enfoque suave (RF-13), aplicada después de reducir: devuelve la
//! nitidez que el promediado de Lanczos suaviza. Radio de ~0,7 px (sección 6:
//! entre 0,5 y 1 px) y un umbral pequeño para no realzar el ruido.

use image::RgbImage;

/// Gaussiana de σ = 0,7 px en 5 coeficientes (suman 1).
const NUCLEO: [f32; 5] = [0.0096, 0.2054, 0.5700, 0.2054, 0.0096];

/// Diferencias menores que esto (en niveles de 0 a 255) no se realzan.
const UMBRAL: f32 = 2.0;

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
    let original = imagen.as_raw().clone();

    // Desenfoque separable: horizontal y luego vertical, con bordes repetidos.
    let mut horizontal = vec![0f32; original.len()];
    for y in 0..alto {
        for x in 0..ancho {
            for c in 0..3 {
                let mut suma = 0.0;
                for (k, peso) in NUCLEO.iter().enumerate() {
                    let xx = (x + k).saturating_sub(2).min(ancho - 1);
                    suma += peso * f32::from(original[(y * ancho + xx) * 3 + c]);
                }
                horizontal[(y * ancho + x) * 3 + c] = suma;
            }
        }
    }

    let datos = &mut **imagen;
    for y in 0..alto {
        for x in 0..ancho {
            for c in 0..3 {
                let mut borroso = 0.0;
                for (k, peso) in NUCLEO.iter().enumerate() {
                    let yy = (y + k).saturating_sub(2).min(alto - 1);
                    borroso += peso * horizontal[(yy * ancho + x) * 3 + c];
                }
                let i = (y * ancho + x) * 3 + c;
                let valor = f32::from(original[i]);
                let detalle = valor - borroso;
                if detalle.abs() > UMBRAL {
                    datos[i] = (valor + detalle * cantidad + 0.5).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

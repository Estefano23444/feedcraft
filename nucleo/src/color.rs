//! Ajustes de color (RF-09). Cada ajuste va de −100 a +100 (el grano, de 0 a 100).
//!
//! Las partes que dependen de un solo canal se precalculan como tablas de 256
//! valores en f32; el resto se calcula píxel a píxel en f32 y se guarda en u8.
//! Nunca se crea una copia de la foto en f32: 24 MP ocuparían 384 MB.

use image::RgbImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ajustes {
    pub brillo: f32,
    pub contraste: f32,
    pub saturacion: f32,
    pub temperatura: f32,
    pub tinte: f32,
    pub luces: f32,
    pub sombras: f32,
    pub vineta: f32,
    pub grano: f32,
}

impl Ajustes {
    /// Recorta cada valor a su rango; un valor no numérico cuenta como 0.
    pub fn limitar(self) -> Self {
        let l = |v: f32, min: f32| if v.is_finite() { v.clamp(min, 100.0) } else { 0.0 };
        Self {
            brillo: l(self.brillo, -100.0),
            contraste: l(self.contraste, -100.0),
            saturacion: l(self.saturacion, -100.0),
            temperatura: l(self.temperatura, -100.0),
            tinte: l(self.tinte, -100.0),
            luces: l(self.luces, -100.0),
            sombras: l(self.sombras, -100.0),
            vineta: l(self.vineta, -100.0),
            grano: l(self.grano, 0.0),
        }
    }
}

/// Rectángulo en coordenadas normalizadas (0 a 1) respecto a la foto.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Marco {
    pub x: f64,
    pub y: f64,
    pub ancho: f64,
    pub alto: f64,
}

impl Marco {
    pub const COMPLETO: Marco = Marco { x: 0.0, y: 0.0, ancho: 1.0, alto: 1.0 };
}

/// Región entera de píxeles dentro de una imagen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub ancho: u32,
    pub alto: u32,
}

impl Region {
    pub fn completa(imagen: &RgbImage) -> Self {
        Self { x: 0, y: 0, ancho: imagen.width(), alto: imagen.height() }
    }
}

/// Ajustes ya traducidos a tablas y factores, listos para recorrer píxeles.
pub struct Motor {
    /// Por canal: brillo, aplicado sobre el valor sRGB de 0 a 1.
    tabla: [[f32; 256]; 3],
    neutro: bool,
}

impl Motor {
    pub fn new(ajustes: &Ajustes) -> Self {
        let a = ajustes.limitar();
        let neutro = a.brillo == 0.0;

        // Brillo: curva de potencia x^γ. Es invertible y no recorta, así que
        // +b seguido de −b devuelve la imagen original (T-04).
        let gamma = 2f32.powf(-a.brillo / 100.0);
        let mut tabla = [[0f32; 256]; 3];
        for canal in &mut tabla {
            for (i, v) in canal.iter_mut().enumerate() {
                *v = (i as f32 / 255.0).powf(gamma);
            }
        }
        Self { tabla, neutro }
    }

    pub fn es_neutro(&self) -> bool {
        self.neutro
    }

    /// Aplica los ajustes a `region` de `origen` y escribe el resultado en
    /// `destino`, con `C` canales por píxel (3 = RGB, 4 = RGBA opaco).
    pub fn aplicar<const C: usize>(&self, origen: &RgbImage, region: Region, destino: &mut [u8]) {
        let ancho_origen = origen.width() as usize;
        let datos = origen.as_raw();
        let ancho = region.ancho as usize;
        assert_eq!(destino.len(), ancho * region.alto as usize * C, "destino de tamaño incorrecto");

        for (fila, salida) in destino.chunks_exact_mut(ancho * C).enumerate() {
            let inicio = ((region.y as usize + fila) * ancho_origen + region.x as usize) * 3;
            let entrada = &datos[inicio..inicio + ancho * 3];
            for (p, q) in entrada.chunks_exact(3).zip(salida.chunks_exact_mut(C)) {
                if self.neutro {
                    q[..3].copy_from_slice(p);
                } else {
                    q[0] = a_u8(self.tabla[0][p[0] as usize]);
                    q[1] = a_u8(self.tabla[1][p[1] as usize]);
                    q[2] = a_u8(self.tabla[2][p[2] as usize]);
                }
                if C == 4 {
                    q[3] = 255;
                }
            }
        }
    }

    /// Atajo: aplica los ajustes a toda la imagen y devuelve una imagen nueva.
    pub fn aplicar_imagen(&self, origen: &RgbImage) -> RgbImage {
        let mut salida = vec![0u8; origen.as_raw().len()];
        self.aplicar::<3>(origen, Region::completa(origen), &mut salida);
        RgbImage::from_raw(origen.width(), origen.height(), salida).expect("tamaño exacto")
    }
}

/// De 0–1 a 0–255 con redondeo. `as u8` satura, y un NaN da 0.
#[inline(always)]
fn a_u8(v: f32) -> u8 {
    (v * 255.0 + 0.5) as u8
}

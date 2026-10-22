//! Sesión de edición y orden obligatorio de la sección 6. La foto decodificada
//! vive aquí mientras el usuario la edita: la vista previa trabaja sobre una
//! copia reducida y la exportación, siempre sobre la original completa.

use image::RgbImage;
use serde::{Deserialize, Serialize};

use crate::color::{self, Ajustes, Encuadre, Marco, Motor, Region};
use crate::decodificar::{self, InfoFoto};
use crate::enfoque::{self, ENFOQUE_POR_DEFECTO};
use crate::exportar;
use crate::recorte::{self, Formato, Salida};
use crate::redimension::{self, RecortePx};

/// Lado mayor de la copia que se envía a la IA (sección 5).
pub const LADO_IA: u32 = 512;

pub struct Sesion {
    pub foto: RgbImage,
    pub vista: RgbImage,
    pub info: InfoFoto,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ParametrosVista {
    pub ajustes: Ajustes,
    /// Encuadre actual: la viñeta se centra en él.
    pub marco: Option<Marco>,
    /// RF-11: mostrar la foto sin ajustes para comparar.
    pub original: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ParametrosExportacion {
    pub formato: Formato,
    /// Marco normalizado (0 a 1). Sin marco, se usa el mayor posible centrado.
    pub marco: Option<Marco>,
    pub ajustes: Ajustes,
    /// RF-08: el usuario eligió ampliar una foto pequeña a 1080 px.
    pub ampliar: bool,
    /// Calidad JPEG de 80 a 100 (RF-14).
    pub calidad: u8,
    /// Intensidad de la máscara de enfoque, de 0 a 100 (RF-13).
    pub enfoque: f32,
}

impl Default for ParametrosExportacion {
    fn default() -> Self {
        Self {
            formato: Formato::default(),
            marco: None,
            ajustes: Ajustes::default(),
            ampliar: false,
            calidad: exportar::CALIDAD_POR_DEFECTO,
            enfoque: ENFOQUE_POR_DEFECTO,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InfoSesion<'a> {
    #[serde(flatten)]
    pub foto: &'a InfoFoto,
    pub vista_ancho: u32,
    pub vista_alto: u32,
}

impl Sesion {
    /// Decodifica la foto una sola vez y prepara la copia para la vista previa,
    /// de `lado_vista` px como máximo.
    pub fn cargar(bytes: &[u8], lado_vista: u32) -> Result<Self, String> {
        let decodificar::Foto { imagen, info } = decodificar::decodificar(bytes)?;
        let (ancho, alto) = redimension::encajar(imagen.width(), imagen.height(), lado_vista);
        let vista = if (ancho, alto) == imagen.dimensions() {
            imagen.clone()
        } else {
            redimension::redimensionar(&imagen, None, ancho, alto)?
        };
        Ok(Self { foto: imagen, vista, info })
    }

    pub fn info(&self) -> InfoSesion<'_> {
        InfoSesion { foto: &self.info, vista_ancho: self.vista.width(), vista_alto: self.vista.height() }
    }

    /// Vista previa en RGBA, lista para un `ImageData` del navegador (RF-10).
    pub fn vista_previa(&self, parametros: &ParametrosVista) -> Vec<u8> {
        let ajustes = if parametros.original { Ajustes::default() } else { parametros.ajustes };
        let (w, h) = self.vista.dimensions();
        let encuadre = parametros.marco.map(|m| {
            let (mw, mh) = (m.ancho * f64::from(w), m.alto * f64::from(h));
            Encuadre {
                cx: (m.x * f64::from(w) + mw / 2.0) as f32,
                cy: (m.y * f64::from(h) + mh / 2.0) as f32,
                rx: (mw / 2.0) as f32,
                ry: (mh / 2.0) as f32,
            }
        });
        let mut rgba = vec![0u8; w as usize * h as usize * 4];
        Motor::new(&ajustes)
            .con_grano(ajustes.grano)
            .aplicar::<4>(&self.vista, Region::completa(&self.vista), &mut rgba, encuadre);
        rgba
    }

    /// Medidas que tendrá la exportación y si la foto es pequeña (RF-08).
    pub fn evaluar(&self, parametros: &ParametrosExportacion) -> Salida {
        let (w, h) = self.foto.dimensions();
        recorte::salida(recorte_px(parametros, w, h), parametros.formato, parametros.ampliar)
    }

    /// Copia de 512 px para la IA, reducida desde la original (nunca se usa
    /// para exportar).
    pub fn copia_ia(&self) -> Result<Vec<u8>, String> {
        let (ancho, alto) = redimension::encajar(self.foto.width(), self.foto.height(), LADO_IA);
        let copia = redimension::redimensionar(&self.foto, None, ancho, alto)?;
        exportar::codificar_jpeg(&copia, 85)
    }

    pub fn exportar(&self, parametros: &ParametrosExportacion) -> Result<Vec<u8>, String> {
        let imagen = renderizar(&self.foto, parametros)?;
        exportar::codificar_jpeg(&imagen, parametros.calidad)
    }
}

fn recorte_px(parametros: &ParametrosExportacion, ancho: u32, alto: u32) -> RecortePx {
    let marco = parametros
        .marco
        .unwrap_or_else(|| recorte::marco_inicial(ancho, alto, parametros.formato, 0.5, 0.5, 1.0));
    recorte::a_pixeles(marco, ancho, alto, parametros.formato)
}

/// Pasos 4 a 7 de la sección 6 sobre la foto ya decodificada, enderezada y en
/// sRGB: color a resolución completa, recorte y Lanczos3 a la medida exacta en
/// un solo paso, enfoque y, al final, grano. Devuelve los píxeles finales,
/// todavía sin codificar.
pub fn renderizar(foto: &RgbImage, parametros: &ParametrosExportacion) -> Result<RgbImage, String> {
    let (w, h) = foto.dimensions();
    let recorte = recorte_px(parametros, w, h);
    let salida = recorte::salida(recorte, parametros.formato, parametros.ampliar);
    let motor = Motor::new(&parametros.ajustes);

    let mut imagen = if motor.es_neutro() {
        redimension::redimensionar(foto, Some(recorte), salida.ancho, salida.alto)?
    } else {
        // El color se aplica solo dentro del recorte: para ajustes por píxel el
        // resultado es idéntico al de la foto entera y el trabajo es menor.
        let region = region_que_cubre(recorte, w, h);
        let relativo = RecortePx { x: recorte.x - f64::from(region.x), y: recorte.y - f64::from(region.y), ..recorte };
        let encuadre = Encuadre {
            cx: (relativo.x + relativo.ancho / 2.0) as f32,
            cy: (relativo.y + relativo.alto / 2.0) as f32,
            rx: (relativo.ancho / 2.0) as f32,
            ry: (relativo.alto / 2.0) as f32,
        };
        let mut ajustada = vec![0u8; region.ancho as usize * region.alto as usize * 3];
        motor.aplicar::<3>(foto, region, &mut ajustada, Some(encuadre));
        let ajustada = RgbImage::from_raw(region.ancho, region.alto, ajustada).expect("tamaño exacto");
        redimension::redimensionar(&ajustada, Some(relativo), salida.ancho, salida.alto)?
    };

    enfoque::enfocar(&mut imagen, parametros.enfoque);
    // El grano va a la medida final: Lanczos lo borraría y el enfoque lo exageraría.
    color::aplicar_grano(&mut imagen, salida.ancho, 3, parametros.ajustes.grano);
    Ok(imagen)
}

/// Menor región de píxeles enteros que contiene el recorte.
fn region_que_cubre(r: RecortePx, ancho: u32, alto: u32) -> Region {
    let x0 = (r.x.floor().max(0.0) as u32).min(ancho - 1);
    let y0 = (r.y.floor().max(0.0) as u32).min(alto - 1);
    let x1 = ((r.x + r.ancho).ceil() as u32).clamp(x0 + 1, ancho);
    let y1 = ((r.y + r.alto).ceil() as u32).clamp(y0 + 1, alto);
    Region { x: x0, y: y0, ancho: x1 - x0, alto: y1 - y0 }
}

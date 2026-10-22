//! Formatos de Instagram (RF-04), marco de recorte (RF-06, RF-07) y aviso de
//! foto pequeña (RF-08).

use serde::{Deserialize, Serialize};

use crate::color::Marco;
use crate::redimension::RecortePx;

/// Todos los formatos de feed comparten 1080 px de ancho (sección 6).
pub const ANCHO_INSTAGRAM: u32 = 1080;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Formato {
    #[serde(rename = "1:1")]
    Cuadrado,
    #[default]
    #[serde(rename = "4:5")]
    Vertical,
    #[serde(rename = "3:4")]
    Retrato,
    #[serde(rename = "9:16")]
    Historia,
}

impl Formato {
    pub const TODOS: [Formato; 4] = [Formato::Cuadrado, Formato::Vertical, Formato::Retrato, Formato::Historia];

    /// Medidas exactas que espera Instagram.
    pub fn medidas(self) -> (u32, u32) {
        match self {
            Formato::Cuadrado => (1080, 1080),
            Formato::Vertical => (1080, 1350),
            Formato::Retrato => (1080, 1440),
            Formato::Historia => (1080, 1920),
        }
    }

    /// Ancho / alto.
    pub fn proporcion(self) -> f64 {
        let (w, h) = self.medidas();
        f64::from(w) / f64::from(h)
    }

    /// Para nombres de archivo: `4x5` (RF-15).
    pub fn etiqueta(self) -> &'static str {
        match self {
            Formato::Cuadrado => "1x1",
            Formato::Vertical => "4x5",
            Formato::Retrato => "3x4",
            Formato::Historia => "9x16",
        }
    }

    pub fn desde_texto(texto: &str) -> Option<Formato> {
        match texto.trim() {
            "1:1" => Some(Formato::Cuadrado),
            "4:5" => Some(Formato::Vertical),
            "3:4" => Some(Formato::Retrato),
            "9:16" => Some(Formato::Historia),
            _ => None,
        }
    }
}

/// El mayor marco del formato que cabe en la foto, reducido por `zoom`
/// (1 = el mayor posible) y centrado en (`cx`, `cy`) sin salirse.
/// Devuelve coordenadas normalizadas (0 a 1).
pub fn marco_inicial(ancho: u32, alto: u32, formato: Formato, cx: f64, cy: f64, zoom: f64) -> Marco {
    let (w, h) = (f64::from(ancho), f64::from(alto));
    let proporcion = formato.proporcion();
    let (mut mw, mut mh) = if w / h > proporcion { (h * proporcion, h) } else { (w, w / proporcion) };
    let zoom = if zoom.is_finite() { zoom.max(1.0) } else { 1.0 };
    mw /= zoom;
    mh /= zoom;
    let x = (limitar01(cx) * w - mw / 2.0).clamp(0.0, w - mw);
    let y = (limitar01(cy) * h - mh / 2.0).clamp(0.0, h - mh);
    Marco { x: x / w, y: y / h, ancho: mw / w, alto: mh / h }
}

/// Pasa un marco normalizado a píxeles de la foto. Corrige la proporción
/// exacta del formato y lo mantiene dentro de la foto, aunque la interfaz
/// mande valores imprecisos.
pub fn a_pixeles(marco: Marco, ancho: u32, alto: u32, formato: Formato) -> RecortePx {
    let (w, h) = (f64::from(ancho), f64::from(alto));
    let proporcion = formato.proporcion();
    let valido = |v: f64, defecto: f64| if v.is_finite() { v } else { defecto };

    let cx = (valido(marco.x, 0.0) + valido(marco.ancho, 1.0) / 2.0) * w;
    let cy = (valido(marco.y, 0.0) + valido(marco.alto, 1.0) / 2.0) * h;
    let mut mw = (valido(marco.ancho, 1.0) * w).clamp(1.0, w);
    let mut mh = mw / proporcion;
    if mh > h {
        mh = h;
        mw = mh * proporcion;
    }
    RecortePx {
        x: (cx - mw / 2.0).clamp(0.0, w - mw),
        y: (cy - mh / 2.0).clamp(0.0, h - mh),
        ancho: mw,
        alto: mh,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Salida {
    pub ancho: u32,
    pub alto: u32,
    /// RF-08: el recorte mide menos de 1080 px en su lado corto.
    pub pequena: bool,
    /// Se amplió porque el usuario lo pidió.
    pub ampliada: bool,
}

/// Medidas de la exportación. Si el recorte es más pequeño que 1080 px, no se
/// amplía salvo que el usuario lo elija (`ampliar`): se exporta a su tamaño
/// real con la proporción exacta del formato.
pub fn salida(recorte: RecortePx, formato: Formato, ampliar: bool) -> Salida {
    let (ancho, alto) = formato.medidas();
    let lado_corto = recorte.ancho.min(recorte.alto);
    let pequena = lado_corto < f64::from(ANCHO_INSTAGRAM) - 0.5;
    if !pequena || ampliar {
        return Salida { ancho, alto, pequena, ampliada: pequena };
    }
    let ancho_real = (recorte.ancho.floor() as u32).max(1);
    let alto_real = ((f64::from(ancho_real) / formato.proporcion()).round() as u32).max(1);
    Salida { ancho: ancho_real, alto: alto_real, pequena, ampliada: false }
}

fn limitar01(v: f64) -> f64 {
    if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.5 }
}

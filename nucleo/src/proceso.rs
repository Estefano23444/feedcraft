//! Sesión de edición: la foto decodificada vive aquí mientras el usuario la
//! edita. La vista previa trabaja sobre una copia reducida; la exportación,
//! siempre sobre la original completa.

use image::RgbImage;
use serde::{Deserialize, Serialize};

use crate::color::{Ajustes, Motor, Region};
use crate::decodificar::{self, InfoFoto};
use crate::redimension;

pub struct Sesion {
    pub foto: RgbImage,
    pub vista: RgbImage,
    pub info: InfoFoto,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ParametrosVista {
    pub ajustes: Ajustes,
    /// RF-11: mostrar la foto sin ajustes para comparar.
    pub original: bool,
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
        let mut rgba = vec![0u8; self.vista.width() as usize * self.vista.height() as usize * 4];
        Motor::new(&ajustes).aplicar::<4>(&self.vista, Region::completa(&self.vista), &mut rgba);
        rgba
    }
}

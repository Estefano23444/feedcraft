//! Núcleo de FeedCraft. Todo el trabajo de píxeles ocurre aquí; la interfaz en
//! JavaScript solo decide qué pedir y el Web Worker hace de puente.
//! Los parámetros cruzan la frontera JS ↔ Rust como JSON.

pub mod color;
pub mod decodificar;
pub mod enfoque;
pub mod exportar;
pub mod proceso;
pub mod recorte;
pub mod redimension;

use wasm_bindgen::prelude::*;

use crate::proceso::{ParametrosExportacion, ParametrosVista, Sesion};

#[wasm_bindgen(start)]
pub fn iniciar() {
    console_error_panic_hook::set_once();
}

/// Versión del núcleo, para comprobar que el puente Rust ↔ navegador funciona.
#[wasm_bindgen]
pub fn version() -> String {
    format!("FeedCraft núcleo {}", env!("CARGO_PKG_VERSION"))
}

/// Una foto abierta en el editor. Se decodifica una vez y todas las vistas
/// previas y exportaciones parten de ella.
#[wasm_bindgen]
pub struct Editor {
    sesion: Sesion,
}

#[wasm_bindgen]
impl Editor {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8], lado_vista: u32) -> Result<Editor, JsError> {
        let sesion = Sesion::cargar(bytes, lado_vista).map_err(|e| JsError::new(&e))?;
        Ok(Editor { sesion })
    }

    /// Medidas, formato, orientación EXIF y perfil de color, en JSON.
    pub fn info(&self) -> String {
        serde_json::to_string(&self.sesion.info()).unwrap_or_default()
    }

    pub fn vista_ancho(&self) -> u32 {
        self.sesion.vista.width()
    }

    pub fn vista_alto(&self) -> u32 {
        self.sesion.vista.height()
    }

    /// Vista previa en RGBA del tamaño de la copia reducida.
    pub fn vista_previa(&self, parametros: &str) -> Result<Vec<u8>, JsError> {
        let parametros: ParametrosVista = leer_json(parametros)?;
        Ok(self.sesion.vista_previa(&parametros))
    }

    /// Medidas de la exportación y aviso de foto pequeña (RF-08), en JSON.
    pub fn evaluar(&self, parametros: &str) -> Result<String, JsError> {
        let parametros: ParametrosExportacion = leer_json(parametros)?;
        Ok(serde_json::to_string(&self.sesion.evaluar(&parametros)).unwrap_or_default())
    }

    /// Procesa la foto original completa y devuelve el JPEG final.
    pub fn exportar(&self, parametros: &str) -> Result<Vec<u8>, JsError> {
        let parametros: ParametrosExportacion = leer_json(parametros)?;
        self.sesion.exportar(&parametros).map_err(|e| JsError::new(&e))
    }
}

fn leer_json<T: serde::de::DeserializeOwned>(texto: &str) -> Result<T, JsError> {
    serde_json::from_str(texto).map_err(|e| JsError::new(&format!("Parámetros no válidos: {e}")))
}

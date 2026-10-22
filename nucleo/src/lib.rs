//! Núcleo de FeedCraft. Todo el trabajo de píxeles ocurre aquí; la interfaz en
//! JavaScript solo decide qué pedir y el Web Worker hace de puente.
//! Los parámetros cruzan la frontera JS ↔ Rust como JSON.

pub mod color;
pub mod decodificar;
pub mod enfoque;
pub mod estilos;
pub mod exportar;
pub mod ia;
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

/// RF-15: nombre del archivo exportado, por ejemplo `foto_film-calido_4x5.jpg`.
/// `formato` es "1:1", "4:5", "3:4" o "9:16"; `estilo` puede ir vacío.
#[wasm_bindgen]
pub fn nombre_archivo(base: &str, estilo: &str, formato: &str) -> String {
    let formato = recorte::Formato::desde_texto(formato).unwrap_or_default();
    exportar::nombre_archivo(base, Some(estilo), formato)
}

/// Valida el contenido de `estilos.json` y devuelve el catálogo normalizado en
/// JSON (ajustes recortados a su rango). Si no es válido, el error explica por qué.
#[wasm_bindgen]
pub fn cargar_catalogo(json: &str) -> Result<String, JsError> {
    let catalogo = estilos::Catalogo::desde_json(json).map_err(|e| JsError::new(&e))?;
    Ok(serde_json::to_string(&catalogo).unwrap_or_default())
}

/// El catálogo incrustado en el wasm, para cuando `estilos.json` no carga.
#[wasm_bindgen]
pub fn catalogo_incrustado() -> String {
    serde_json::to_string(&estilos::Catalogo::incrustado()).unwrap_or_default()
}

/// Convierte la respuesta de la IA en una sugerencia válida (JSON). Solo falla
/// si la respuesta no contiene un JSON legible: la app reintenta una vez.
#[wasm_bindgen]
pub fn normalizar_sugerencia(texto: &str, catalogo: &str) -> Result<String, JsError> {
    let catalogo = estilos::Catalogo::desde_json(catalogo).unwrap_or_else(|_| estilos::Catalogo::incrustado());
    let sugerencia = ia::normalizar(texto, &catalogo).map_err(|e| JsError::new(&e))?;
    Ok(serde_json::to_string(&sugerencia).unwrap_or_default())
}

/// La opción segura: Natural mejorado en 4:5 y dos alternativas del catálogo.
#[wasm_bindgen]
pub fn sugerencia_respaldo(catalogo: &str, motivo: &str) -> String {
    let catalogo = estilos::Catalogo::desde_json(catalogo).unwrap_or_else(|_| estilos::Catalogo::incrustado());
    serde_json::to_string(&ia::respaldo(&catalogo, motivo)).unwrap_or_default()
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

    /// Copia reducida (512 px de lado mayor, JPEG) que se envía a la IA. Es lo
    /// único que sale del navegador, y solo si el usuario pulsa el botón de IA.
    pub fn copia_ia(&self) -> Result<Vec<u8>, JsError> {
        self.sesion.copia_ia().map_err(|e| JsError::new(&e))
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

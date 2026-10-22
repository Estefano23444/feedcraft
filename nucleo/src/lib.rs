//! Núcleo de FeedCraft. Todo el trabajo de píxeles ocurre aquí; la interfaz en
//! JavaScript solo decide qué pedir y el Web Worker hace de puente.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn iniciar() {
    console_error_panic_hook::set_once();
}

/// Versión del núcleo, para comprobar que el puente Rust ↔ navegador funciona.
#[wasm_bindgen]
pub fn version() -> String {
    format!("FeedCraft núcleo {}", env!("CARGO_PKG_VERSION"))
}

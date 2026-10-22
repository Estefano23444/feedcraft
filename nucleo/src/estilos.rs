//! Catálogo de estilos en tendencia (sección 4). Las tendencias viven en
//! `web/estilos.json`, que mantiene el usuario; la IA solo elige del catálogo.
//! Una copia va incrustada en el wasm como respaldo si el archivo no carga.

use serde::{Deserialize, Serialize};

use crate::color::Ajustes;

/// La opción segura: siempre existe (sección 4).
pub const NATURAL: &str = "natural-mejorado";

/// Más opciones empeoran la elección de la IA (sección 4).
pub const MINIMO: usize = 5;
pub const MAXIMO: usize = 20;

/// Límite de cada corrección que la IA suma sobre el estilo base (sección 5).
pub const LIMITE_EXTRA: f32 = 20.0;

/// El catálogo de `web/estilos.json`, incrustado al compilar.
pub const CATALOGO_INCRUSTADO: &str = include_str!("../../web/estilos.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Estilo {
    pub id: String,
    pub nombre: String,
    pub descripcion: String,
    #[serde(default)]
    pub ajustes: Ajustes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vigente_desde: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Catalogo {
    estilos: Vec<Estilo>,
}

impl Catalogo {
    /// Lee y valida un catálogo. Los ajustes fuera de rango se recortan.
    pub fn desde_json(texto: &str) -> Result<Self, String> {
        let mut estilos: Vec<Estilo> =
            serde_json::from_str(texto).map_err(|e| format!("estilos.json no es válido: {e}"))?;

        if !(MINIMO..=MAXIMO).contains(&estilos.len()) {
            return Err(format!(
                "estilos.json tiene {} estilos; deben ser entre {MINIMO} y {MAXIMO}.",
                estilos.len()
            ));
        }
        let mut vistos = std::collections::HashSet::new();
        for estilo in &mut estilos {
            estilo.id = estilo.id.trim().to_string();
            if estilo.id.is_empty() || !estilo.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(format!(
                    "El id «{}» no es válido: usa minúsculas, números y guiones (por ejemplo film-calido).",
                    estilo.id
                ));
            }
            if !vistos.insert(estilo.id.clone()) {
                return Err(format!("El id «{}» está repetido en estilos.json.", estilo.id));
            }
            if estilo.nombre.trim().is_empty() {
                return Err(format!("El estilo «{}» no tiene nombre.", estilo.id));
            }
            estilo.ajustes = estilo.ajustes.limitar();
        }
        if !vistos.contains(NATURAL) {
            return Err(format!("estilos.json debe incluir el estilo «{NATURAL}»: es la opción segura."));
        }
        Ok(Self { estilos })
    }

    /// El catálogo incrustado en el wasm. Una prueba garantiza que es válido.
    pub fn incrustado() -> Self {
        Self::desde_json(CATALOGO_INCRUSTADO).expect("web/estilos.json debe ser válido")
    }

    pub fn estilos(&self) -> &[Estilo] {
        &self.estilos
    }

    pub fn buscar(&self, id: &str) -> Option<&Estilo> {
        self.estilos.iter().find(|e| e.id == id)
    }

    /// El estilo seguro, que siempre existe.
    pub fn natural(&self) -> &Estilo {
        self.buscar(NATURAL).expect("el catálogo validado incluye natural-mejorado")
    }
}

/// Estilo base + correcciones de la IA. Cada corrección se limita a ±20 y el
/// resultado, al rango de cada ajuste.
pub fn mezclar(base: &Ajustes, extra: &Ajustes) -> Ajustes {
    let e = |v: f32| if v.is_finite() { v.clamp(-LIMITE_EXTRA, LIMITE_EXTRA) } else { 0.0 };
    Ajustes {
        brillo: base.brillo + e(extra.brillo),
        contraste: base.contraste + e(extra.contraste),
        saturacion: base.saturacion + e(extra.saturacion),
        temperatura: base.temperatura + e(extra.temperatura),
        tinte: base.tinte + e(extra.tinte),
        luces: base.luces + e(extra.luces),
        sombras: base.sombras + e(extra.sombras),
        vineta: base.vineta + e(extra.vineta),
        grano: base.grano + e(extra.grano),
    }
    .limitar()
}

//! Validación de la respuesta de la IA (sección 5). La app nunca confía a
//! ciegas en la IA: cualquier texto se convierte en una sugerencia válida o en
//! un error de JSON (para reintentar). Ninguna entrada provoca un pánico.
//!
//! Reglas:
//! - estilo que no existe en el catálogo → "Natural mejorado";
//! - siempre 3 variantes con estilos distintos del catálogo;
//! - formato desconocido → 4:5; centro de interés recortado a 0–1;
//! - cada ajuste extra limitado a ±20; claves desconocidas, ignoradas;
//! - caption de hasta 2200 caracteres y hasta 30 hashtags, todos con "#".

use serde::Serialize;
use serde_json::{Map, Value};

use crate::color::Ajustes;
use crate::estilos::{mezclar, Catalogo, LIMITE_EXTRA, NATURAL};
use crate::recorte::Formato;

pub const VARIANTES: usize = 3;
const MAX_CAPTION: usize = 2200;
const MAX_HASHTAGS: usize = 30;
const MAX_MOTIVO: usize = 140;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Centro {
    pub cx: f64,
    pub cy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Variante {
    pub estilo: String,
    /// Correcciones de la IA ya limitadas a ±20.
    pub ajustes_extra: Ajustes,
    /// Estilo base + correcciones: lo que se aplica a la foto.
    pub ajustes: Ajustes,
    pub motivo: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sugerencia {
    pub formato: Formato,
    pub recorte: Centro,
    pub variantes: Vec<Variante>,
    pub caption: String,
    pub hashtags: Vec<String>,
    /// "ia" si viene de la respuesta, "respaldo" si se usó la opción segura.
    pub origen: &'static str,
    /// Qué se corrigió de la respuesta, para depurar y para mostrarlo.
    pub correcciones: Vec<String>,
}

/// Convierte el texto de la IA en una sugerencia válida. Solo falla si no hay
/// un objeto JSON legible: en ese caso la app reintenta una vez.
pub fn normalizar(texto: &str, catalogo: &Catalogo) -> Result<Sugerencia, String> {
    let objeto = extraer_objeto(texto).ok_or("La IA no devolvió un JSON válido.")?;
    let mut correcciones = Vec::new();

    let formato = match objeto.get("formato").and_then(Value::as_str).and_then(Formato::desde_texto) {
        Some(f) => f,
        None => {
            correcciones.push("formato no válido: se usa 4:5".into());
            Formato::Vertical
        }
    };

    let recorte = objeto.get("recorte").and_then(Value::as_object);
    let coordenada = |clave: &str, correcciones: &mut Vec<String>| match recorte.and_then(|r| r.get(clave)).and_then(numero) {
        Some(v) if (0.0..=1.0).contains(&v) => v,
        Some(v) => {
            correcciones.push(format!("recorte.{clave} fuera de 0–1"));
            v.clamp(0.0, 1.0)
        }
        None => 0.5,
    };
    let recorte = Centro { cx: coordenada("cx", &mut correcciones), cy: coordenada("cy", &mut correcciones) };

    let mut variantes: Vec<Variante> = Vec::new();
    for v in objeto.get("variantes").and_then(Value::as_array).into_iter().flatten() {
        if variantes.len() == VARIANTES {
            correcciones.push("sobraban variantes".into());
            break;
        }
        let Some(v) = v.as_object() else {
            correcciones.push("variante que no es un objeto".into());
            continue;
        };
        let pedido = v.get("estilo").and_then(Value::as_str).unwrap_or("").trim();
        let mut estilo = if catalogo.buscar(pedido).is_some() {
            pedido.to_string()
        } else {
            correcciones.push(format!("estilo «{}» no existe: se usa {NATURAL}", recortar(pedido, 40)));
            NATURAL.to_string()
        };
        if variantes.iter().any(|x| x.estilo == estilo) {
            match libre(catalogo, &variantes) {
                Some(otro) => {
                    correcciones.push(format!("estilo «{estilo}» repetido: se usa {otro}"));
                    estilo = otro;
                }
                None => continue,
            }
        }
        let extra = leer_extra(v.get("ajustes_extra"), &mut correcciones);
        let motivo = v.get("motivo").and_then(Value::as_str).map(|m| recortar(m.trim(), MAX_MOTIVO)).unwrap_or_default();
        variantes.push(variante(catalogo, &estilo, extra, motivo));
    }
    if variantes.len() < VARIANTES {
        correcciones.push(format!("faltaban {} variantes", VARIANTES - variantes.len()));
        completar(catalogo, &mut variantes);
    }

    let caption = objeto.get("caption").and_then(Value::as_str).map(|c| recortar(c.trim(), MAX_CAPTION)).unwrap_or_default();
    let hashtags = leer_hashtags(objeto.get("hashtags"));

    Ok(Sugerencia { formato, recorte, variantes, caption, hashtags, origen: "ia", correcciones })
}

/// La opción segura (sección 5): "Natural mejorado" en 4:5, centrado, y dos
/// alternativas del catálogo para que el usuario siga teniendo dónde elegir.
pub fn respaldo(catalogo: &Catalogo, motivo: &str) -> Sugerencia {
    let mut variantes = Vec::new();
    completar(catalogo, &mut variantes);
    Sugerencia {
        formato: Formato::Vertical,
        recorte: Centro { cx: 0.5, cy: 0.5 },
        variantes,
        caption: String::new(),
        hashtags: Vec::new(),
        origen: "respaldo",
        correcciones: vec![motivo.to_string()],
    }
}

/// Encuentra el objeto JSON dentro del texto: quita razonamientos `<think>`,
/// bloques de código y texto alrededor.
fn extraer_objeto(texto: &str) -> Option<Map<String, Value>> {
    let mut limpio = texto.to_string();
    while let Some(inicio) = limpio.find("<think>") {
        match limpio[inicio..].find("</think>") {
            Some(fin) => limpio.replace_range(inicio..inicio + fin + "</think>".len(), ""),
            None => limpio.truncate(inicio),
        }
    }
    if let Ok(Value::Object(o)) = serde_json::from_str::<Value>(limpio.trim()) {
        return Some(o);
    }
    let desde = limpio.find('{')?;
    let hasta = limpio.rfind('}')?;
    if hasta <= desde {
        return None;
    }
    match serde_json::from_str::<Value>(&limpio[desde..=hasta]) {
        Ok(Value::Object(o)) => Some(o),
        _ => None,
    }
}

/// Un número, aunque venga como texto ("12").
fn numero(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64().filter(|x| x.is_finite()),
        Value::String(s) => s.trim().parse::<f64>().ok().filter(|x| x.is_finite()),
        _ => None,
    }
}

fn leer_extra(valor: Option<&Value>, correcciones: &mut Vec<String>) -> Ajustes {
    let mut a = Ajustes::default();
    let Some(objeto) = valor.and_then(Value::as_object) else {
        return a;
    };
    for (clave, v) in objeto {
        let campo = match clave.as_str() {
            "brillo" => &mut a.brillo,
            "contraste" => &mut a.contraste,
            "saturacion" | "saturación" => &mut a.saturacion,
            "temperatura" => &mut a.temperatura,
            "tinte" => &mut a.tinte,
            "luces" => &mut a.luces,
            "sombras" => &mut a.sombras,
            "vineta" | "viñeta" => &mut a.vineta,
            "grano" => &mut a.grano,
            otra => {
                correcciones.push(format!("ajuste «{}» desconocido: se ignora", recortar(otra, 30)));
                continue;
            }
        };
        let Some(n) = numero(v) else { continue };
        let limitado = n.clamp(-f64::from(LIMITE_EXTRA), f64::from(LIMITE_EXTRA));
        if limitado != n {
            correcciones.push(format!("{clave} = {n} fuera de ±20"));
        }
        *campo = limitado as f32;
    }
    a
}

fn leer_hashtags(valor: Option<&Value>) -> Vec<String> {
    let crudos: Vec<String> = match valor {
        Some(Value::Array(lista)) => lista.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(Value::String(s)) => s.split([' ', ',', '\n']).map(str::to_string).collect(),
        _ => Vec::new(),
    };
    let mut hashtags: Vec<String> = Vec::new();
    for crudo in crudos {
        let cuerpo: String = crudo.trim().trim_start_matches('#').chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();
        if cuerpo.is_empty() {
            continue;
        }
        let etiqueta = format!("#{}", recortar(&cuerpo, 60));
        if !hashtags.iter().any(|h| h.to_lowercase() == etiqueta.to_lowercase()) {
            hashtags.push(etiqueta);
        }
        if hashtags.len() == MAX_HASHTAGS {
            break;
        }
    }
    hashtags
}

fn variante(catalogo: &Catalogo, estilo: &str, extra: Ajustes, motivo: String) -> Variante {
    let base = catalogo.buscar(estilo).unwrap_or_else(|| catalogo.natural());
    Variante { estilo: base.id.clone(), ajustes_extra: extra, ajustes: mezclar(&base.ajustes, &extra), motivo }
}

/// Primer estilo del catálogo que aún no está en las variantes; Natural
/// mejorado va primero porque es la opción segura.
fn libre(catalogo: &Catalogo, variantes: &[Variante]) -> Option<String> {
    std::iter::once(catalogo.natural())
        .chain(catalogo.estilos())
        .find(|e| !variantes.iter().any(|v| v.estilo == e.id))
        .map(|e| e.id.clone())
}

fn completar(catalogo: &Catalogo, variantes: &mut Vec<Variante>) {
    while variantes.len() < VARIANTES {
        let Some(id) = libre(catalogo, variantes) else { break };
        let motivo = if id == NATURAL { "Opción segura" } else { "Alternativa del catálogo" };
        variantes.push(variante(catalogo, &id, Ajustes::default(), motivo.into()));
    }
}

fn recortar(texto: &str, maximo: usize) -> String {
    texto.chars().take(maximo).collect()
}

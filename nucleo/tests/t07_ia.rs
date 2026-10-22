//! T-07 IA robusta (sección 5): con 10 respuestas JSON rotas o inventadas, la
//! app nunca falla y cae en "Natural mejorado".

use nucleo::estilos::{Catalogo, LIMITE_EXTRA, NATURAL};
use nucleo::ia::{normalizar, respaldo, Sugerencia};
use nucleo::recorte::Formato;

/// El flujo de la app: si el JSON no es válido, reintenta una vez; si vuelve
/// a fallar, usa la opción segura.
fn flujo(primera: &str, segunda: &str, catalogo: &Catalogo) -> Sugerencia {
    normalizar(primera, catalogo)
        .or_else(|_| normalizar(segunda, catalogo))
        .unwrap_or_else(|e| respaldo(catalogo, &e))
}

fn anidado(niveles: usize) -> String {
    "{\"a\":".repeat(niveles) + "1" + &"}".repeat(niveles)
}

/// Todo lo que debe cumplir cualquier sugerencia que llegue a la interfaz.
fn comprobar_valida(s: &Sugerencia, catalogo: &Catalogo, caso: &str) {
    assert!(Formato::TODOS.contains(&s.formato), "{caso}");
    assert!((0.0..=1.0).contains(&s.recorte.cx) && (0.0..=1.0).contains(&s.recorte.cy), "{caso}: {:?}", s.recorte);
    assert_eq!(s.variantes.len(), 3, "{caso}");
    for (i, v) in s.variantes.iter().enumerate() {
        assert!(catalogo.buscar(&v.estilo).is_some(), "{caso}: estilo {} fuera del catálogo", v.estilo);
        assert!(s.variantes[..i].iter().all(|w| w.estilo != v.estilo), "{caso}: estilos repetidos");
        let e = v.ajustes_extra;
        for x in [e.brillo, e.contraste, e.saturacion, e.temperatura, e.tinte, e.luces, e.sombras, e.vineta, e.grano] {
            assert!(x.is_finite() && x.abs() <= LIMITE_EXTRA, "{caso}: extra {x}");
        }
        assert_eq!(v.ajustes, v.ajustes.limitar(), "{caso}: ajustes fuera de rango");
    }
    assert!(s.hashtags.len() <= 30 && s.hashtags.iter().all(|h| h.starts_with('#') && h.len() > 1), "{caso}");
    assert!(s.caption.chars().count() <= 2200, "{caso}");
}

#[test]
fn t07_diez_respuestas_rotas_o_inventadas() {
    let catalogo = Catalogo::incrustado();
    let casos: [(&str, String); 10] = [
        ("texto sin JSON", "Claro, te recomiendo un estilo cálido con mucho contraste.".into()),
        ("JSON cortado", r#"{"formato": "4:5", "variantes": [{"estilo": "film-calido""#.into()),
        ("vacía", String::new()),
        ("arreglo en vez de objeto", "[1, 2, 3]".into()),
        ("estilos y formato inventados", r#"{"formato":"16:9","variantes":[{"estilo":"cyberpunk"},{"estilo":"vaporwave"},{"estilo":"<script>alert(1)</script>"}]}"#.into()),
        (
            "razonamiento y bloque de código con estilo inventado",
            "<think>La foto es de noche...</think>\n```json\n{\"formato\":\"4:5\",\"variantes\":[{\"estilo\":\"noche-neon\",\"motivo\":\"x\"},{\"estilo\":\"alto-contraste-urbano\"}]}\n```".into(),
        ),
        (
            "valores fuera de rango y repetidos",
            r#"{"formato":"4:5","recorte":{"cx":5,"cy":-2},"variantes":[{"estilo":"film-calido","ajustes_extra":{"brillo":999,"contraste":"abc","inventado":5}},{"estilo":"film-calido"},{"estilo":"film-calido"}]}"#.into(),
        ),
        ("tipos equivocados", r##"{"formato":45,"recorte":"centro","variantes":"tres","caption":["no"],"hashtags":"#uno #dos dos ###"}"##.into()),
        ("objeto vacío", "{}".into()),
        ("anidación extrema", anidado(5000)),
    ];

    for (caso, texto) in casos {
        let s = flujo(&texto, &texto, &catalogo);
        comprobar_valida(&s, &catalogo, caso);
        assert!(s.variantes.iter().any(|v| v.estilo == NATURAL), "{caso}: debe caer en Natural mejorado");
        println!("{caso}: {} · {:?} · {}", s.origen, s.variantes.iter().map(|v| v.estilo.as_str()).collect::<Vec<_>>(), s.correcciones.join("; "));
    }
}

#[test]
fn json_roto_dos_veces_usa_natural_mejorado_en_4x5() {
    let catalogo = Catalogo::incrustado();
    let s = flujo("{roto", "tampoco", &catalogo);
    assert_eq!(s.origen, "respaldo");
    assert_eq!(s.formato, Formato::Vertical);
    assert_eq!(s.variantes[0].estilo, NATURAL);
}

#[test]
fn el_reintento_rescata_una_segunda_respuesta_buena() {
    let catalogo = Catalogo::incrustado();
    let buena = r#"{"formato":"1:1","variantes":[{"estilo":"pastel-suave"},{"estilo":"minimal-frio"},{"estilo":"film-calido"}]}"#;
    let s = flujo("no es json", buena, &catalogo);
    assert_eq!(s.origen, "ia");
    assert_eq!(s.formato, Formato::Cuadrado);
    assert_eq!(s.variantes[0].estilo, "pastel-suave");
}

#[test]
fn respuesta_real_de_groq_se_respeta_y_se_limpia() {
    // Respuesta real de qwen/qwen3.8-27b (22 oct 2026) a una imagen sintética.
    let texto = r##"{"formato":"4:5","recorte":{"cx":0.5,"cy":0.5},"variantes":[{"estilo":"natural-mejorado","ajustes_extra":{"saturacion":1.1,"brillo":1.05},"motivo":"Resalta los tonos verdes de las colinas y el azul del cielo de forma equilibrada."},{"estilo":"minimal-frio","ajustes_extra":{"temperatura":-10,"contraste":0.9},"motivo":"Enfría la paleta para crear una atmósfera serena y limpia, minimizando el impacto del elemento gráfico."},{"estilo":"film-calido","ajustes_extra":{"grano":20,"tonos_amarillos":15},"motivo":"Aporta una sensación nostálgica y cinematográfica, suavizando la transición entre el cielo y el campo."}],"caption":"Un paisaje digital minimalista con colinas onduladas.","hashtags":["#paisaje","#minimalismo","#ilustracion","#naturaleza","#arte"]}"##;
    let catalogo = Catalogo::incrustado();
    let s = normalizar(texto, &catalogo).unwrap();
    comprobar_valida(&s, &catalogo, "groq real");
    assert_eq!(s.origen, "ia");
    let estilos: Vec<_> = s.variantes.iter().map(|v| v.estilo.as_str()).collect();
    assert_eq!(estilos, ["natural-mejorado", "minimal-frio", "film-calido"]);
    assert_eq!(s.variantes[1].ajustes_extra.temperatura, -10.0);
    // Estilo base + extra: minimal-frio tiene temperatura -14.
    assert_eq!(s.variantes[1].ajustes.temperatura, -24.0);
    assert!(s.correcciones.iter().any(|c| c.contains("tonos_amarillos")), "{:?}", s.correcciones);
    assert_eq!(s.hashtags.len(), 5);
}

#[test]
fn hashtags_se_limpian() {
    let catalogo = Catalogo::incrustado();
    let muchos: Vec<String> = (0..50).map(|i| format!("\"tag{i}\"")).collect();
    let texto = format!(r##"{{"hashtags":["#Playa","playa","#playa!!","", "#atardecer en la costa", {}]}}"##, muchos.join(","));
    let s = normalizar(&texto, &catalogo).unwrap();
    assert_eq!(s.hashtags[0], "#Playa");
    assert_eq!(s.hashtags[1], "#atardecerenlacosta", "sin espacios");
    assert_eq!(s.hashtags.len(), 30, "máximo 30");
    assert_eq!(s.hashtags.iter().filter(|h| h.to_lowercase() == "#playa").count(), 1, "sin duplicados");
}

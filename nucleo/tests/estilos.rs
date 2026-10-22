//! Etapa 6 — catálogo de estilos (RF-17, sección 4): cada estilo exporta sin
//! error en cada formato, y el catálogo se valida antes de usarlo.

mod comun;

use comun::*;
use nucleo::color::Ajustes;
use nucleo::estilos::{mezclar, Catalogo, CATALOGO_INCRUSTADO, NATURAL};
use nucleo::proceso::{ParametrosExportacion, Sesion};
use nucleo::recorte::Formato;

#[test]
fn el_catalogo_de_web_es_valido() {
    let catalogo = Catalogo::incrustado();
    assert!((5..=20).contains(&catalogo.estilos().len()));
    assert_eq!(catalogo.natural().id, NATURAL);
    for id in ["film-calido", "minimal-frio", "alto-contraste-urbano", "pastel-suave", "natural-mejorado"] {
        assert!(catalogo.buscar(id).is_some(), "falta {id}");
    }
}

#[test]
fn cada_estilo_exporta_sin_error_en_cada_formato() {
    let foto = foto_sintetica(2400, 3000, 17);
    let sesion = Sesion::cargar(&jpeg(&foto, 95), 1600).unwrap();
    let catalogo = Catalogo::incrustado();
    for estilo in catalogo.estilos() {
        for formato in Formato::TODOS {
            let p = ParametrosExportacion { formato, ajustes: estilo.ajustes, ..Default::default() };
            let jpeg = sesion.exportar(&p).unwrap_or_else(|e| panic!("{} {formato:?}: {e}", estilo.id));
            let leida = image::load_from_memory(&jpeg).unwrap();
            assert_eq!((leida.width(), leida.height()), formato.medidas(), "{} {formato:?}", estilo.id);
        }
        // Y cambia la foto: un estilo que no hace nada no sirve.
        let con = sesion.exportar(&ParametrosExportacion { ajustes: estilo.ajustes, ..Default::default() }).unwrap();
        let sin = sesion.exportar(&ParametrosExportacion::default()).unwrap();
        assert_ne!(con, sin, "{} no cambia la foto", estilo.id);
    }
}

#[test]
fn catalogos_invalidos_se_rechazan_con_un_mensaje_claro() {
    let estilo = |id: &str| format!(r#"{{"id":"{id}","nombre":"X","descripcion":"d","ajustes":{{}}}}"#);
    let lista = |ids: &[&str]| format!("[{}]", ids.iter().map(|i| estilo(i)).collect::<Vec<_>>().join(","));

    let casos = [
        ("no es json".to_string(), "no es válido"),
        (lista(&["natural-mejorado", "a", "b", "c"]), "entre 5 y 20"),
        (lista(&(0..21).map(|i| if i == 0 { "natural-mejorado".to_string() } else { format!("e{i}") }).collect::<Vec<_>>().iter().map(String::as_str).collect::<Vec<_>>()), "entre 5 y 20"),
        (lista(&["a", "b", "c", "d", "e"]), NATURAL),
        (lista(&["natural-mejorado", "a", "a", "c", "d"]), "repetido"),
        (lista(&["natural-mejorado", "Con Espacios", "b", "c", "d"]), "no es válido"),
    ];
    for (texto, esperado) in casos {
        let error = Catalogo::desde_json(&texto).expect_err(&texto);
        assert!(error.contains(esperado), "«{error}» debería mencionar «{esperado}»");
    }
}

#[test]
fn ajustes_fuera_de_rango_en_el_catalogo_se_recortan() {
    let texto = CATALOGO_INCRUSTADO.replace(r#""brillo": 5,"#, r#""brillo": 500,"#);
    let catalogo = Catalogo::desde_json(&texto).unwrap();
    assert_eq!(catalogo.buscar("film-calido").unwrap().ajustes.brillo, 100.0);
}

#[test]
fn mezclar_limita_cada_correccion_a_20() {
    let base = Ajustes { brillo: 5.0, saturacion: 90.0, ..Default::default() };
    let extra = Ajustes { brillo: 50.0, saturacion: 15.0, contraste: -35.0, temperatura: f32::NAN, ..Default::default() };
    let m = mezclar(&base, &extra);
    assert_eq!(m.brillo, 25.0, "5 + 20 (50 recortado a 20)");
    assert_eq!(m.saturacion, 100.0, "90 + 15 recortado al máximo");
    assert_eq!(m.contraste, -20.0);
    assert_eq!(m.temperatura, 0.0, "NaN se ignora");
}

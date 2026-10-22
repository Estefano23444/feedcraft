//! Corpus real (sección 9): cada foto de `pruebas/corpus/` se abre y se
//! exporta en los cuatro formatos. Las fotos pequeñas avisan y no se amplían
//! solas (RF-08). Si el corpus está vacío, la prueba se omite.

mod comun;

use comun::*;
use nucleo::estilos::Catalogo;
use nucleo::proceso::{ParametrosExportacion, Sesion};
use nucleo::recorte::Formato;

#[test]
fn corpus_exporta() {
    let fotos = fotos_corpus();
    if fotos.is_empty() {
        println!("pruebas/corpus/ está vacío: se omite");
        return;
    }
    let estilo = Catalogo::incrustado().estilos()[0].ajustes;
    for ruta in &fotos {
        let nombre = ruta.file_name().unwrap().to_string_lossy();
        let sesion = Sesion::cargar(&std::fs::read(ruta).unwrap(), 1600).unwrap_or_else(|e| panic!("{nombre}: {e}"));
        for formato in Formato::TODOS {
            let p = ParametrosExportacion { formato, ajustes: estilo, ..Default::default() };
            let evaluacion = sesion.evaluar(&p);
            let jpeg = sesion.exportar(&p).unwrap_or_else(|e| panic!("{nombre} {formato:?}: {e}"));
            let leida = image::load_from_memory(&jpeg).unwrap();
            let medidas = (leida.width(), leida.height());
            assert_eq!(medidas, (evaluacion.ancho, evaluacion.alto), "{nombre} {formato:?}");
            if evaluacion.pequena {
                assert!(medidas.0 < 1080, "{nombre} {formato:?}: no se amplía sin permiso");
            } else {
                assert_eq!(medidas, formato.medidas(), "{nombre} {formato:?}");
            }
        }
        println!("{nombre}: {}×{} {} · orientación {} · {}", sesion.info.ancho, sesion.info.alto, sesion.info.formato, sesion.info.orientacion,
            if sesion.info.tenia_transparencia { "con transparencia" } else { "opaca" });
    }
}

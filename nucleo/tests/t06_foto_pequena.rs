//! T-06 Foto pequeña (RF-08): la foto de 800 px muestra el aviso y no se
//! amplía en silencio.

mod comun;

use comun::*;
use nucleo::proceso::{renderizar, ParametrosExportacion, Sesion};
use nucleo::recorte::Formato;

#[test]
fn t06_foto_de_800_px_avisa_y_no_se_amplia_sola() {
    let foto = imagen_prueba(800, 600, 5);
    let sesion = Sesion::cargar(&jpeg(&foto, 95), 1600).unwrap();

    for formato in Formato::TODOS {
        let p = ParametrosExportacion { formato, ..Default::default() };
        let evaluacion = sesion.evaluar(&p);
        assert!(evaluacion.pequena, "{formato:?}: debe avisar");
        assert!(!evaluacion.ampliada);
        assert!(evaluacion.ancho < 1080, "{formato:?}: no se amplía sin permiso");

        // Mantiene la proporción del formato a su tamaño real.
        let (w, h) = (f64::from(evaluacion.ancho), f64::from(evaluacion.alto));
        assert!((w / h - formato.proporcion()).abs() < 0.005, "{formato:?}: {w}×{h}");
        let real = renderizar(&sesion.foto, &p).unwrap();
        assert_eq!(real.dimensions(), (evaluacion.ancho, evaluacion.alto));
    }
}

#[test]
fn t06_solo_se_amplia_si_el_usuario_lo_elige() {
    let foto = imagen_prueba(800, 600, 6);
    let sesion = Sesion::cargar(&jpeg(&foto, 95), 1600).unwrap();
    let p = ParametrosExportacion { formato: Formato::Vertical, ampliar: true, ..Default::default() };
    let evaluacion = sesion.evaluar(&p);
    assert!(evaluacion.pequena && evaluacion.ampliada);
    assert_eq!(renderizar(&sesion.foto, &p).unwrap().dimensions(), (1080, 1350));
}

#[test]
fn una_foto_grande_no_avisa() {
    let foto = imagen_prueba(1080, 1350, 7);
    let sesion = Sesion::cargar(&jpeg(&foto, 95), 1600).unwrap();
    let evaluacion = sesion.evaluar(&ParametrosExportacion { formato: Formato::Vertical, ..Default::default() });
    assert!(!evaluacion.pequena);
    assert_eq!((evaluacion.ancho, evaluacion.alto), (1080, 1350));
}

#[test]
fn zoom_excesivo_en_foto_grande_tambien_avisa() {
    // Un recorte de menos de 1080 px de ancho dentro de una foto grande.
    let foto = imagen_prueba(2000, 2000, 8);
    let sesion = Sesion::cargar(&jpeg(&foto, 95), 1600).unwrap();
    let marco = nucleo::recorte::marco_inicial(2000, 2000, Formato::Cuadrado, 0.5, 0.5, 2.5);
    let p = ParametrosExportacion { formato: Formato::Cuadrado, marco: Some(marco), ..Default::default() };
    assert!(sesion.evaluar(&p).pequena);
}

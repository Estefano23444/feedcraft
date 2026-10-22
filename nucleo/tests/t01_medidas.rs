//! T-01 Medidas exactas (RF-04, RF-12): cada exportación mide exactamente lo
//! que dice su formato, sea cual sea la foto de origen y el recorte.

mod comun;

use comun::*;
use nucleo::color::{Ajustes, Marco};
use nucleo::proceso::{renderizar, ParametrosExportacion, Sesion};
use nucleo::recorte::{a_pixeles, marco_inicial, Formato};

const ORIGENES: [(u32, u32); 7] = [(4000, 3000), (3000, 4000), (1080, 1080), (1200, 2400), (5000, 1100), (1081, 1351), (2062, 2749)];

/// Medidas exactas siempre que el recorte llegue a 1080 px o el usuario elija
/// ampliar; si no, es el caso de foto pequeña de T-06.
fn medidas_esperadas(foto: &image::RgbImage, p: &ParametrosExportacion) -> Option<(u32, u32)> {
    let (w, h) = foto.dimensions();
    let marco = p.marco.unwrap_or_else(|| marco_inicial(w, h, p.formato, 0.5, 0.5, 1.0));
    let ancho_recorte = a_pixeles(marco, w, h, p.formato).ancho;
    (p.ampliar || ancho_recorte >= 1080.0 - 0.5).then(|| p.formato.medidas())
}

#[test]
fn t01_cada_formato_mide_exactamente_lo_que_dice() {
    let mut comprobadas = 0;
    for (i, &(w, h)) in ORIGENES.iter().enumerate() {
        let foto = imagen_prueba(w, h, i as u64 + 1);
        for formato in Formato::TODOS {
            for ajustes in [Ajustes::default(), Ajustes { brillo: 15.0, ..Default::default() }] {
                for ampliar in [false, true] {
                    let p = ParametrosExportacion { formato, ajustes, ampliar, ..Default::default() };
                    let salida = renderizar(&foto, &p).unwrap();
                    if let Some(esperadas) = medidas_esperadas(&foto, &p) {
                        assert_eq!(salida.dimensions(), esperadas, "{w}×{h} → {formato:?}, ampliar={ampliar}");
                        comprobadas += 1;
                    } else {
                        assert!(salida.width() < 1080, "{w}×{h} → {formato:?}: no se amplía en silencio");
                    }
                }
            }
        }
    }
    assert!(comprobadas >= 80, "solo {comprobadas} casos con medida exacta");
}

#[test]
fn t01_el_jpeg_exportado_mide_lo_mismo() {
    let foto = imagen_prueba(3000, 4000, 9);
    let sesion = sesion_de(&foto);
    for formato in Formato::TODOS {
        let jpeg = sesion.exportar(&ParametrosExportacion { formato, ..Default::default() }).unwrap();
        let leida = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((leida.width(), leida.height()), formato.medidas(), "{formato:?}");
    }
}

#[test]
fn t01_marcos_con_zoom_y_descentrados_tambien() {
    let foto = imagen_prueba(4000, 3000, 3);
    let mut azar = Azar::new(42);
    for formato in Formato::TODOS {
        for _ in 0..6 {
            let cx = f64::from(azar.byte()) / 255.0;
            let cy = f64::from(azar.byte()) / 255.0;
            let zoom = 1.0 + f64::from(azar.byte()) / 255.0 * 1.7;
            let marco = marco_inicial(4000, 3000, formato, cx, cy, zoom);
            for ampliar in [false, true] {
                let p = ParametrosExportacion { formato, marco: Some(marco), ampliar, ..Default::default() };
                let salida = renderizar(&foto, &p).unwrap();
                match medidas_esperadas(&foto, &p) {
                    Some(esperadas) => assert_eq!(salida.dimensions(), esperadas),
                    None => assert!(salida.width() < 1080),
                }
            }
        }
    }
}

#[test]
fn el_marco_siempre_tiene_la_proporcion_exacta_y_cabe() {
    // Marcos imprecisos o absurdos, como los que podría mandar la interfaz.
    let marcos = [
        Marco { x: 0.1, y: 0.1, ancho: 0.5, alto: 0.9 },
        Marco { x: -0.5, y: 2.0, ancho: 3.0, alto: 0.1 },
        Marco { x: f64::NAN, y: 0.2, ancho: f64::INFINITY, alto: 0.3 },
        Marco { x: 0.99, y: 0.99, ancho: 0.001, alto: 0.001 },
    ];
    for formato in Formato::TODOS {
        for marco in marcos {
            let r = a_pixeles(marco, 4000, 3000, formato);
            assert!((r.ancho / r.alto - formato.proporcion()).abs() < 1e-9, "{formato:?} {marco:?}");
            assert!(r.x >= 0.0 && r.y >= 0.0 && r.x + r.ancho <= 4000.0 + 1e-9 && r.y + r.alto <= 3000.0 + 1e-9);
        }
    }
}

#[test]
fn el_marco_inicial_se_centra_en_el_punto_de_interes() {
    // Foto apaisada y formato 4:5: el marco se desplaza hacia el sujeto (RF-07).
    let izquierda = marco_inicial(4000, 3000, Formato::Vertical, 0.1, 0.5, 1.0);
    let derecha = marco_inicial(4000, 3000, Formato::Vertical, 0.9, 0.5, 1.0);
    assert_eq!(izquierda.x, 0.0, "pegado al borde izquierdo");
    assert!((derecha.x + derecha.ancho - 1.0).abs() < 1e-9, "pegado al borde derecho");
    assert_eq!(izquierda.alto, 1.0, "usa toda la altura");
}

fn sesion_de(foto: &image::RgbImage) -> Sesion {
    Sesion::cargar(&jpeg(foto, 95), 1600).unwrap()
}

//! T-03 Ajuste neutro y T-04 Simetría (RF-09).

mod comun;

use comun::*;
use nucleo::color::{Ajustes, Motor};

fn aplicar(imagen: &image::RgbImage, ajustes: Ajustes) -> image::RgbImage {
    Motor::new(&ajustes).aplicar_imagen(imagen)
}

#[test]
fn t03_con_todo_en_cero_la_imagen_no_cambia() {
    for imagen in [imagen_prueba(301, 157, 7), todos_los_valores()] {
        let salida = aplicar(&imagen, Ajustes::default());
        assert!(diferencia_maxima(&imagen, &salida) <= 1);
    }
}

#[test]
fn t04_brillo_mas_20_y_luego_menos_20_vuelve_al_original() {
    for imagen in [imagen_prueba(301, 157, 11), todos_los_valores()] {
        let arriba = aplicar(&imagen, Ajustes { brillo: 20.0, ..Default::default() });
        assert!(diferencia_maxima(&imagen, &arriba) > 10, "el brillo debe notarse");
        let vuelta = aplicar(&arriba, Ajustes { brillo: -20.0, ..Default::default() });
        let d = diferencia_maxima(&imagen, &vuelta);
        assert!(d <= 2, "diferencia {d} > 2");
    }
}

#[test]
fn brillo_positivo_aclara_y_negativo_oscurece() {
    let imagen = todos_los_valores();
    let media = |i: &image::RgbImage| i.as_raw().iter().map(|&v| f64::from(v)).sum::<f64>() / i.as_raw().len() as f64;
    let base = media(&imagen);
    assert!(media(&aplicar(&imagen, Ajustes { brillo: 50.0, ..Default::default() })) > base + 10.0);
    assert!(media(&aplicar(&imagen, Ajustes { brillo: -50.0, ..Default::default() })) < base - 10.0);
}

#[test]
fn valores_fuera_de_rango_se_recortan() {
    let a = Ajustes { brillo: 500.0, contraste: -300.0, grano: -5.0, saturacion: f32::NAN, ..Default::default() }.limitar();
    assert_eq!(a.brillo, 100.0);
    assert_eq!(a.contraste, -100.0);
    assert_eq!(a.grano, 0.0);
    assert_eq!(a.saturacion, 0.0);
}

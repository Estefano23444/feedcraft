//! T-03 Ajuste neutro y T-04 Simetría (RF-09), más el comportamiento de cada
//! ajuste, la viñeta, el grano y el enfoque (RF-13).

mod comun;

use comun::*;
use image::RgbImage;
use nucleo::color::{aplicar_grano, Ajustes, Motor};
use nucleo::enfoque::enfocar;
use nucleo::proceso::{renderizar, ParametrosExportacion};
use nucleo::redimension::redimensionar;

fn aplicar(imagen: &RgbImage, ajustes: Ajustes) -> RgbImage {
    Motor::new(&ajustes).aplicar_imagen(imagen)
}

fn media(imagen: &RgbImage, canal: usize) -> f64 {
    let datos = imagen.as_raw();
    datos.iter().skip(canal).step_by(3).map(|&v| f64::from(v)).sum::<f64>() / (datos.len() / 3) as f64
}

fn luma_media(imagen: &RgbImage) -> f64 {
    0.2126 * media(imagen, 0) + 0.7152 * media(imagen, 1) + 0.0722 * media(imagen, 2)
}

fn desviacion_luma(imagen: &RgbImage) -> f64 {
    let lumas: Vec<f64> = imagen.pixels().map(|p| 0.2126 * f64::from(p[0]) + 0.7152 * f64::from(p[1]) + 0.0722 * f64::from(p[2])).collect();
    let m = lumas.iter().sum::<f64>() / lumas.len() as f64;
    (lumas.iter().map(|l| (l - m).powi(2)).sum::<f64>() / lumas.len() as f64).sqrt()
}

#[test]
fn t03_con_todo_en_cero_la_imagen_no_cambia() {
    for imagen in [imagen_prueba(301, 157, 7), todos_los_valores()] {
        let salida = aplicar(&imagen, Ajustes::default());
        assert!(diferencia_maxima(&imagen, &salida) <= 1);
    }
}

#[test]
fn t03_la_exportacion_neutra_es_solo_recorte_y_reduccion() {
    let foto = imagen_prueba(2400, 3000, 21);
    let p = ParametrosExportacion { enfoque: 0.0, ..Default::default() };
    let exportada = renderizar(&foto, &p).unwrap();
    let solo_reducida = redimensionar(&foto, None, 1080, 1350).unwrap();
    assert!(diferencia_maxima(&exportada, &solo_reducida) <= 1);
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
    let base = luma_media(&imagen);
    assert!(luma_media(&aplicar(&imagen, Ajustes { brillo: 50.0, ..Default::default() })) > base + 10.0);
    assert!(luma_media(&aplicar(&imagen, Ajustes { brillo: -50.0, ..Default::default() })) < base - 10.0);
}

#[test]
fn contraste_separa_o_junta_los_tonos() {
    let imagen = imagen_prueba(256, 256, 3);
    let base = desviacion_luma(&imagen);
    assert!(desviacion_luma(&aplicar(&imagen, Ajustes { contraste: 60.0, ..Default::default() })) > base * 1.1);
    let plana = aplicar(&imagen, Ajustes { contraste: -60.0, ..Default::default() });
    assert!(desviacion_luma(&plana) < base * 0.9);
    // Contraste negativo levanta los negros (look de película).
    let negro = RgbImage::from_pixel(4, 4, image::Rgb([0, 0, 0]));
    assert!(aplicar(&negro, Ajustes { contraste: -60.0, ..Default::default() }).get_pixel(0, 0)[0] > 30);
}

#[test]
fn saturacion_menos_100_da_blanco_y_negro() {
    let gris = aplicar(&imagen_prueba(200, 120, 4), Ajustes { saturacion: -100.0, ..Default::default() });
    for p in gris.pixels() {
        assert!(p[0].abs_diff(p[1]) <= 1 && p[1].abs_diff(p[2]) <= 1, "{p:?}");
    }
}

#[test]
fn temperatura_y_tinte_mueven_los_canales() {
    let imagen = imagen_prueba(200, 120, 5);
    let calida = aplicar(&imagen, Ajustes { temperatura: 50.0, ..Default::default() });
    let fria = aplicar(&imagen, Ajustes { temperatura: -50.0, ..Default::default() });
    let calidez = |i: &RgbImage| media(i, 0) - media(i, 2);
    assert!(calidez(&calida) > calidez(&imagen) + 5.0);
    assert!(calidez(&fria) < calidez(&imagen) - 5.0);

    let magenta = aplicar(&imagen, Ajustes { tinte: 50.0, ..Default::default() });
    let verde = aplicar(&imagen, Ajustes { tinte: -50.0, ..Default::default() });
    assert!(media(&magenta, 1) < media(&imagen, 1) - 3.0);
    assert!(media(&verde, 1) > media(&imagen, 1) + 3.0);
}

#[test]
fn sombras_y_luces_actuan_en_su_zona() {
    let oscuro = RgbImage::from_pixel(8, 8, image::Rgb([50, 50, 50]));
    let claro = RgbImage::from_pixel(8, 8, image::Rgb([210, 210, 210]));
    let sombras = Ajustes { sombras: 60.0, ..Default::default() };
    let luces = Ajustes { luces: -60.0, ..Default::default() };

    let s_oscuro = aplicar(&oscuro, sombras).get_pixel(0, 0)[0];
    let s_claro = aplicar(&claro, sombras).get_pixel(0, 0)[0];
    assert!(s_oscuro > 60, "las sombras suben los oscuros: {s_oscuro}");
    assert!(s_claro.abs_diff(210) < s_oscuro.abs_diff(50), "y casi no tocan los claros");

    let l_claro = aplicar(&claro, luces).get_pixel(0, 0)[0];
    let l_oscuro = aplicar(&oscuro, luces).get_pixel(0, 0)[0];
    assert!(l_claro < 200, "las luces bajan los claros: {l_claro}");
    assert!(l_oscuro.abs_diff(50) < l_claro.abs_diff(210), "y casi no tocan los oscuros");
}

#[test]
fn vineta_oscurece_los_bordes_del_encuadre() {
    let plano = RgbImage::from_pixel(300, 400, image::Rgb([180, 180, 180]));
    let con = aplicar(&plano, Ajustes { vineta: 80.0, ..Default::default() });
    let centro = con.get_pixel(150, 200)[0];
    let esquina = con.get_pixel(2, 2)[0];
    assert!(centro >= 178, "el centro no cambia: {centro}");
    assert!(esquina < 120, "la esquina se oscurece: {esquina}");

    let blanca = aplicar(&plano, Ajustes { vineta: -80.0, ..Default::default() });
    assert!(blanca.get_pixel(2, 2)[0] > 210, "viñeta negativa aclara los bordes");
}

#[test]
fn el_grano_es_determinista_y_no_cambia_el_tono() {
    let base = RgbImage::from_pixel(256, 256, image::Rgb([128, 128, 128]));
    let mut a = base.clone();
    let mut b = base.clone();
    aplicar_grano(&mut a, 256, 3, 40.0);
    aplicar_grano(&mut b, 256, 3, 40.0);
    assert_eq!(a, b, "mismo resultado cada vez");
    assert!(diferencia_maxima(&a, &base) > 5, "el grano se nota");
    assert!((media(&a, 1) - 128.0).abs() < 1.5, "la media no cambia");
    let mut c = base.clone();
    aplicar_grano(&mut c, 256, 3, 0.0);
    assert_eq!(c, base, "grano 0 no toca nada");
}

#[test]
fn el_enfoque_realza_bordes_y_cero_no_toca_nada() {
    // Borde suave, como el que deja una reducción.
    let borde = RgbImage::from_fn(64, 64, |x, _| {
        let v = (f64::from(x) - 32.0).clamp(-4.0, 4.0) * 16.0 + 128.0;
        image::Rgb([v as u8; 3])
    });
    let mut cero = borde.clone();
    enfocar(&mut cero, 0.0);
    assert_eq!(cero, borde);

    let mut enfocada = borde.clone();
    enfocar(&mut enfocada, 60.0);
    // La máscara de enfoque acentúa los extremos del borde: más contraste local.
    let salto = |i: &RgbImage| i32::from(i.get_pixel(36, 10)[0]) - i32::from(i.get_pixel(28, 10)[0]);
    assert!(salto(&enfocada) > salto(&borde), "el borde es más nítido");
    // Zonas planas no cambian (umbral).
    assert_eq!(enfocada.get_pixel(5, 5), borde.get_pixel(5, 5));
}

#[test]
fn ningun_ajuste_extremo_rompe_la_exportacion() {
    let foto = imagen_prueba(1500, 1500, 9);
    for v in [-100.0, 100.0] {
        let ajustes = Ajustes {
            brillo: v, contraste: v, saturacion: v, temperatura: v, tinte: v,
            luces: v, sombras: v, vineta: v, grano: v.abs(),
        };
        let p = ParametrosExportacion { ajustes, enfoque: 100.0, ..Default::default() };
        assert_eq!(renderizar(&foto, &p).unwrap().dimensions(), (1080, 1350));
    }
}

//! T-02 Orientación (RF-02): las fotos con EXIF rotado salen derechas.
//! También cubre la carga de PNG con transparencia y la conversión de perfil ICC.

mod comun;

use comun::*;
use image::{Rgb, RgbImage, Rgba, RgbaImage};
use nucleo::decodificar::decodificar;

const ROJO: [u8; 3] = [230, 30, 30];
const VERDE: [u8; 3] = [30, 200, 40];
const AZUL: [u8; 3] = [30, 40, 220];
const BLANCO: [u8; 3] = [250, 250, 250];

/// La foto "derecha", como debe verse: 64×32, cuatro cuadrantes de color.
fn derecha() -> RgbImage {
    RgbImage::from_fn(64, 32, |x, y| {
        Rgb(match (x < 32, y < 16) {
            (true, true) => ROJO,
            (false, true) => VERDE,
            (true, false) => AZUL,
            (false, false) => BLANCO,
        })
    })
}

/// Cómo la guarda la cámara para cada orientación EXIF: al aplicar la
/// orientación a esta imagen debe salir `derecha()`. Las transformaciones se
/// escriben a mano para no comprobar la librería contra sí misma.
fn almacenada(u: &RgbImage, orientacion: u16) -> RgbImage {
    let (w, h) = u.dimensions();
    let girada = matches!(orientacion, 5..=8);
    let (aw, ah) = if girada { (h, w) } else { (w, h) };
    RgbImage::from_fn(aw, ah, |x, y| {
        let (ux, uy) = match orientacion {
            1 => (x, y),
            2 => (w - 1 - x, y),
            3 => (w - 1 - x, h - 1 - y),
            4 => (x, h - 1 - y),
            5 => (y, x),
            6 => (w - 1 - y, x),
            7 => (w - 1 - y, h - 1 - x),
            8 => (y, h - 1 - x),
            _ => unreachable!(),
        };
        *u.get_pixel(ux, uy)
    })
}

fn cerca(real: [f64; 3], esperado: [u8; 3]) -> bool {
    real.iter().zip(esperado).all(|(r, e)| (r - f64::from(e)).abs() < 25.0)
}

#[test]
fn t02_las_ocho_orientaciones_salen_derechas() {
    let u = derecha();
    for orientacion in 1..=8u16 {
        let bytes = jpeg_con(&almacenada(&u, orientacion), 95, Some(exif_orientacion(orientacion)), None);
        let foto = decodificar(&bytes).expect("decodifica");
        let d = &foto.imagen;

        assert_eq!(d.dimensions(), (64, 32), "orientación {orientacion}: medidas");
        assert_eq!(foto.info.orientacion as u16, orientacion, "orientación {orientacion}: leída");
        for (x, y, color, nombre) in [(4, 2, ROJO, "rojo"), (36, 2, VERDE, "verde"), (4, 18, AZUL, "azul"), (36, 18, BLANCO, "blanco")] {
            let medio = color_medio(d, x, y, 24, 12);
            assert!(cerca(medio, color), "orientación {orientacion}: cuadrante {nombre} es {medio:?}");
        }
    }
}

#[test]
fn sin_exif_la_foto_no_se_toca() {
    let u = derecha();
    let foto = decodificar(&jpeg(&u, 95)).unwrap();
    assert_eq!(foto.info.orientacion, 1);
    assert_eq!(foto.imagen.dimensions(), (64, 32));
}

#[test]
fn png_con_transparencia_se_aplana_sobre_blanco() {
    let img = RgbaImage::from_fn(20, 10, |x, _| if x < 10 { Rgba([0, 0, 0, 0]) } else { Rgba([0, 0, 0, 255]) });
    let foto = decodificar(&png_rgba(&img)).unwrap();
    assert!(foto.info.tenia_transparencia);
    assert_eq!(foto.imagen.get_pixel(2, 2).0, [255, 255, 255], "transparente → blanco");
    assert_eq!(foto.imagen.get_pixel(15, 2).0, [0, 0, 0], "opaco se conserva");
}

#[test]
fn perfil_display_p3_se_convierte_a_srgb() {
    let p3 = moxcms::ColorProfile::new_display_p3().encode().unwrap();
    let img = RgbImage::from_fn(32, 16, |x, _| if x < 16 { Rgb([128, 128, 128]) } else { Rgb([180, 80, 60]) });
    let foto = decodificar(&jpeg_con(&img, 98, None, Some(p3))).unwrap();
    assert!(foto.info.perfil_convertido.is_some(), "debe informar la conversión");

    let gris = color_medio(&foto.imagen, 2, 2, 12, 12);
    assert!(gris.iter().all(|c| (c - 128.0).abs() < 4.0), "el gris sigue gris: {gris:?}");
    // P3 tiene una gama más amplia: el mismo número es un rojo más intenso en sRGB.
    let rojizo = color_medio(&foto.imagen, 18, 2, 12, 12);
    assert!(rojizo[0] > 185.0 && rojizo[1] < 80.0, "el rojizo se satura en sRGB: {rojizo:?}");
}

#[test]
fn perfil_srgb_no_se_convierte() {
    let srgb = moxcms::ColorProfile::new_srgb().encode().unwrap();
    let foto = decodificar(&jpeg_con(&derecha(), 95, None, Some(srgb))).unwrap();
    assert!(foto.info.perfil_convertido.is_none());
}

#[test]
fn formatos_y_errores() {
    assert!(decodificar(b"esto no es una imagen").is_err());
    assert!(decodificar(&[]).is_err());
    let gif = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";
    let error = decodificar(gif).err().unwrap();
    assert!(error.contains("JPEG, PNG o WebP"), "{error}");
}

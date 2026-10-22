//! T-05 Calidad (sección 6): la exportación final, comparada con una
//! referencia, mantiene SSIM ≥ 0,97. También cubre RF-14 (JPEG sRGB 4:4:4,
//! calidad 80–100) y RF-15 (nombre del archivo).
//!
//! La referencia es independiente del motor: el mismo recorte se reduce con
//! el Lanczos3 de la librería `image` y no se codifica en JPEG. Recibe el mismo
//! enfoque que la exportación, porque el enfoque es una mejora buscada y no
//! una pérdida: la prueba mide lo que se pierde al reducir y codificar.

mod comun;

use comun::*;
use image::imageops::{self, FilterType};
use image::RgbImage;
use nucleo::enfoque::enfocar;
use nucleo::exportar::{codificar_jpeg, nombre_archivo};
use nucleo::proceso::{ParametrosExportacion, Sesion};
use nucleo::recorte::{a_pixeles, marco_inicial, Formato};

const SSIM_MINIMO: f64 = 0.97;

/// Exporta con el motor y devuelve el JPEG decodificado.
fn exportar(sesion: &Sesion, p: &ParametrosExportacion) -> RgbImage {
    let jpeg = sesion.exportar(p).unwrap();
    image::load_from_memory(&jpeg).unwrap().to_rgb8()
}

/// Referencia: el mismo rectángulo de recorte (T-05 mide la reducción y la
/// codificación, no dónde va el marco) reducido con el Lanczos3 de `image`.
fn referencia(foto: &RgbImage, formato: Formato, enfoque: f32) -> RgbImage {
    let (w, h) = foto.dimensions();
    let r = a_pixeles(marco_inicial(w, h, formato, 0.5, 0.5, 1.0), w, h, formato);
    let recorte = imageops::crop_imm(foto, r.x as u32, r.y as u32, r.ancho as u32, r.alto as u32).to_image();
    let (fw, fh) = formato.medidas();
    let mut reducida = imageops::resize(&recorte, fw, fh, FilterType::Lanczos3);
    enfocar(&mut reducida, enfoque);
    reducida
}

#[test]
fn t05_ssim_contra_referencia_en_fotos_sinteticas() {
    for (i, (w, h, formato)) in [(2400, 3000, Formato::Vertical), (4000, 3000, Formato::Vertical), (3000, 3000, Formato::Cuadrado), (2160, 3840, Formato::Historia)]
        .into_iter()
        .enumerate()
    {
        let foto = foto_sintetica(w, h, i as u64 + 1);
        // La sesión decodifica un JPEG de la foto: la referencia parte de esos mismos píxeles.
        let sesion = Sesion::cargar(&jpeg(&foto, 97), 1600).unwrap();
        for enfoque in [0.0, 30.0] {
            let p = ParametrosExportacion { formato, enfoque, ..Default::default() };
            let s = ssim(&exportar(&sesion, &p), &referencia(&sesion.foto, formato, enfoque));
            println!("{w}×{h} {formato:?} enfoque {enfoque}: SSIM {s:.4}");
            assert!(s >= SSIM_MINIMO, "{w}×{h} {formato:?} enfoque {enfoque}: SSIM {s:.4} < {SSIM_MINIMO}");
        }
    }
}

#[test]
fn t05_ssim_en_el_corpus_real() {
    let fotos = fotos_corpus();
    if fotos.is_empty() {
        println!("pruebas/corpus/ está vacío: se omite");
        return;
    }
    let mut medidas = 0;
    for ruta in fotos {
        let sesion = Sesion::cargar(&std::fs::read(&ruta).unwrap(), 1600).unwrap();
        let (w, h) = sesion.foto.dimensions();
        for formato in [Formato::Vertical, Formato::Retrato, Formato::Cuadrado] {
            let p = ParametrosExportacion { formato, ..Default::default() };
            if sesion.evaluar(&p).pequena {
                continue; // foto pequeña: es el caso de T-06
            }
            let s = ssim(&exportar(&sesion, &p), &referencia(&sesion.foto, formato, p.enfoque));
            let nombre = ruta.file_name().unwrap().to_string_lossy();
            println!("{nombre} ({w}×{h}) {formato:?}: SSIM {s:.4}");
            assert!(s >= SSIM_MINIMO, "{nombre} {formato:?}: SSIM {s:.4}");
            medidas += 1;
        }
    }
    println!("{medidas} exportaciones del corpus comparadas");
}

#[test]
fn rf14_jpeg_444_con_perfil_srgb() {
    let foto = foto_sintetica(1200, 1500, 5);
    let jpeg = codificar_jpeg(&foto, 92).unwrap();
    assert_eq!(muestreo_jpeg(&jpeg), vec![0x11, 0x11, 0x11], "submuestreo 4:4:4");
    assert!(jpeg.windows(12).any(|v| v == b"ICC_PROFILE\0"), "lleva el perfil sRGB");
    // El decodificador lo reconoce como sRGB: no hay conversión.
    let leida = nucleo::decodificar::decodificar(&jpeg).unwrap();
    assert!(leida.info.perfil_convertido.is_none());
}

#[test]
fn rf14_la_calidad_va_de_80_a_100() {
    let foto = foto_sintetica(800, 1000, 6);
    let tamano = |q| codificar_jpeg(&foto, q).unwrap().len();
    assert_eq!(tamano(50), tamano(80), "por debajo de 80 se usa 80");
    assert!(tamano(80) < tamano(92) && tamano(92) < tamano(100));
}

#[test]
fn rf15_nombre_con_estilo_y_formato() {
    assert_eq!(nombre_archivo("foto", Some("film-calido"), Formato::Vertical), "foto_film-calido_4x5.jpg");
    assert_eq!(nombre_archivo("Mi Foto Ñandú.JPG", Some("Film cálido"), Formato::Historia), "mi-foto-nandu-jpg_film-calido_9x16.jpg");
    assert_eq!(nombre_archivo("", Some("pastel-suave"), Formato::Cuadrado), "foto_pastel-suave_1x1.jpg");
    assert_eq!(nombre_archivo("playa", None, Formato::Retrato), "playa_3x4.jpg");
    assert_eq!(nombre_archivo("playa", Some("  "), Formato::Retrato), "playa_3x4.jpg");
    assert_eq!(nombre_archivo("../../etc/passwd", Some("x"), Formato::Vertical), "etc-passwd_x_4x5.jpg");
}

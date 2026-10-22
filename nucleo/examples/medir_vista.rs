//! Mide en nativo el coste de la vista previa y de la exportación.
//! Uso: cargo run --release -p nucleo --example medir_vista -- <foto>

use std::time::Instant;

use nucleo::color::Ajustes;
use nucleo::proceso::{ParametrosExportacion, ParametrosVista, Sesion};

fn main() {
    let ruta = std::env::args().nth(1).expect("falta la ruta de la foto");
    let bytes = std::fs::read(&ruta).expect("leer foto");

    let t = Instant::now();
    let sesion = Sesion::cargar(&bytes, 1600).expect("cargar");
    println!("cargar: {:?}  ({}×{})", t.elapsed(), sesion.foto.width(), sesion.foto.height());

    let todos = Ajustes {
        brillo: 8.0, contraste: -12.0, saturacion: -10.0, temperatura: 22.0, tinte: 5.0,
        luces: -15.0, sombras: 20.0, vineta: 25.0, grano: 15.0,
    };
    let solo = |f: fn(&mut Ajustes)| {
        let mut a = Ajustes { brillo: 10.0, ..Default::default() };
        f(&mut a);
        a
    };
    for (nombre, ajustes) in [
        ("solo brillo", solo(|_| {})),
        ("brillo + sombras", solo(|a| a.sombras = 20.0)),
        ("brillo + saturación", solo(|a| a.saturacion = -10.0)),
        ("brillo + viñeta", solo(|a| a.vineta = 25.0)),
        ("brillo + grano", solo(|a| a.grano = 15.0)),
        ("todo sin grano", Ajustes { grano: 0.0, ..todos }),
        ("todo", todos),
    ] {
        let p = ParametrosVista { ajustes, ..Default::default() };
        let t = Instant::now();
        for _ in 0..10 {
            std::hint::black_box(sesion.vista_previa(&p));
        }
        println!("vista {nombre}: {:?}", t.elapsed() / 10);
    }

    let t = Instant::now();
    let jpeg = sesion.exportar(&ParametrosExportacion { ajustes: todos, ..Default::default() }).expect("exportar");
    println!("exportar 4:5 con todo: {:?} ({} KB)", t.elapsed(), jpeg.len() / 1024);
}

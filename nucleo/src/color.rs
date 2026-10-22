//! Ajustes de color (RF-09). Cada ajuste va de −100 a +100 (el grano, de 0 a 100).
//!
//! Las partes que dependen de un solo canal (temperatura, tinte, brillo y
//! contraste) se precalculan como tablas de 256 valores en f32; luces, sombras,
//! saturación y viñeta se calculan píxel a píxel en f32 y se guardan en u8.
//! Nunca se crea una copia de la foto en f32: 24 MP ocuparían 384 MB.

use image::RgbImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ajustes {
    pub brillo: f32,
    pub contraste: f32,
    pub saturacion: f32,
    pub temperatura: f32,
    pub tinte: f32,
    pub luces: f32,
    pub sombras: f32,
    pub vineta: f32,
    pub grano: f32,
}

impl Ajustes {
    /// Recorta cada valor a su rango; un valor no numérico cuenta como 0.
    pub fn limitar(self) -> Self {
        let l = |v: f32, min: f32| if v.is_finite() { v.clamp(min, 100.0) } else { 0.0 };
        Self {
            brillo: l(self.brillo, -100.0),
            contraste: l(self.contraste, -100.0),
            saturacion: l(self.saturacion, -100.0),
            temperatura: l(self.temperatura, -100.0),
            tinte: l(self.tinte, -100.0),
            luces: l(self.luces, -100.0),
            sombras: l(self.sombras, -100.0),
            vineta: l(self.vineta, -100.0),
            grano: l(self.grano, 0.0),
        }
    }
}

/// Rectángulo en coordenadas normalizadas (0 a 1) respecto a la foto.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Marco {
    pub x: f64,
    pub y: f64,
    pub ancho: f64,
    pub alto: f64,
}

impl Marco {
    pub const COMPLETO: Marco = Marco { x: 0.0, y: 0.0, ancho: 1.0, alto: 1.0 };
}

/// Región entera de píxeles dentro de una imagen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub ancho: u32,
    pub alto: u32,
}

impl Region {
    pub fn completa(imagen: &RgbImage) -> Self {
        Self { x: 0, y: 0, ancho: imagen.width(), alto: imagen.height() }
    }
}

/// Encuadre al que se refiere la viñeta, en píxeles relativos a la esquina de
/// la región procesada: centro y semiejes. La viñeta oscurece los bordes del
/// encuadre final, no los de la foto entera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Encuadre {
    pub cx: f32,
    pub cy: f32,
    pub rx: f32,
    pub ry: f32,
}

impl Encuadre {
    pub fn de_region(region: Region) -> Self {
        let (rx, ry) = (region.ancho as f32 / 2.0, region.alto as f32 / 2.0);
        Self { cx: rx, cy: ry, rx, ry }
    }
}

/// Coeficientes de luma Rec. 709 (los de sRGB).
const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];
const PASOS_TONO: usize = 1024;
/// La tabla de viñeta cubre distancias al cuadrado de 0 a 2 (más allá de las esquinas).
const PASOS_VINETA: usize = 1024;
const ESCALA_VINETA: f32 = PASOS_VINETA as f32 / 2.0;

/// Ajustes ya traducidos a tablas y factores, listos para recorrer píxeles.
pub struct Motor {
    /// Por canal: temperatura y tinte (en luz lineal), brillo y contraste.
    tabla: [[f32; 256]; 3],
    /// Desplazamiento de luces y sombras en función de la luma.
    tono: Option<Box<[f32; PASOS_TONO + 1]>>,
    saturacion: f32,
    vineta: f32,
    /// Peso de la viñeta en función de la distancia al centro al cuadrado.
    peso_vineta: Option<Box<[f32; PASOS_VINETA + 1]>>,
    /// Amplitud del grano cuando va en el mismo recorrido (solo la vista previa;
    /// la exportación lo añade aparte, a la medida final).
    amplitud_grano: f32,
    solo_tabla: bool,
    neutro: bool,
}

impl Motor {
    pub fn new(ajustes: &Ajustes) -> Self {
        let a = ajustes.limitar();
        let ganancias = ganancias_balance(a.temperatura / 100.0, a.tinte / 100.0);
        let gamma = 2f32.powf(-a.brillo / 100.0);
        let contraste = a.contraste / 100.0;

        let mut tabla = [[0f32; 256]; 3];
        for (canal, fila) in tabla.iter_mut().enumerate() {
            for (i, v) in fila.iter_mut().enumerate() {
                let mut x = i as f32 / 255.0;
                if ganancias[canal] != 1.0 {
                    x = lineal_a_srgb((srgb_a_lineal(x) * ganancias[canal]).min(1.0));
                }
                // Brillo: curva de potencia x^γ. Es invertible y no recorta, así que
                // +b seguido de −b devuelve la imagen original (T-04).
                if gamma != 1.0 {
                    x = x.powf(gamma);
                }
                if contraste > 0.0 {
                    x += (curva_s(x) - x) * contraste;
                } else if contraste < 0.0 {
                    // Aplana hacia el gris medio: levanta los negros y baja los blancos.
                    x += (0.5 - x) * 0.5 * -contraste;
                }
                *v = x;
            }
        }

        let tono = (a.luces != 0.0 || a.sombras != 0.0).then(|| {
            let (s, l) = (a.sombras / 100.0 * 1.2, a.luces / 100.0 * 1.2);
            let mut t = Box::new([0f32; PASOS_TONO + 1]);
            for (i, v) in t.iter_mut().enumerate() {
                let y = i as f32 / PASOS_TONO as f32;
                // Campanas que culminan en 1/3 (sombras) y 2/3 (luces) y valen 0
                // en el negro y el blanco puros.
                *v = s * y * (1.0 - y) * (1.0 - y) + l * y * y * (1.0 - y);
            }
            t
        });

        let saturacion = 1.0 + a.saturacion / 100.0;
        let vineta = a.vineta / 100.0;
        let peso_vineta = (vineta != 0.0).then(|| {
            let mut t = Box::new([0f32; PASOS_VINETA + 1]);
            for (i, v) in t.iter_mut().enumerate() {
                let r = (i as f32 / ESCALA_VINETA).sqrt();
                *v = suave(0.35, 1.0, r) * 0.65 * vineta.abs();
            }
            t
        });
        let tabla_neutra = ganancias == [1.0; 3] && gamma == 1.0 && contraste == 0.0;
        let solo_tabla = tono.is_none() && saturacion == 1.0 && vineta == 0.0;
        Self {
            tabla,
            tono,
            saturacion,
            vineta,
            peso_vineta,
            amplitud_grano: 0.0,
            solo_tabla,
            neutro: tabla_neutra && solo_tabla,
        }
    }

    /// Incluye el grano en el mismo recorrido, para la vista previa: ahorra una
    /// segunda pasada por la imagen.
    pub fn con_grano(mut self, grano: f32) -> Self {
        self.amplitud_grano = amplitud_grano(grano);
        if self.amplitud_grano > 0.0 {
            self.neutro = false;
        }
        self
    }

    /// Sin ningún ajuste de color (el grano va aparte, a la medida final).
    pub fn es_neutro(&self) -> bool {
        self.neutro
    }

    /// Aplica los ajustes a `region` de `origen` y escribe el resultado en
    /// `destino`, con `C` canales por píxel (3 = RGB, 4 = RGBA opaco).
    /// La viñeta se centra en `encuadre` (por defecto, la región entera).
    pub fn aplicar<const C: usize>(
        &self,
        origen: &RgbImage,
        region: Region,
        destino: &mut [u8],
        encuadre: Option<Encuadre>,
    ) {
        let ancho = region.ancho as usize;
        assert_eq!(destino.len(), ancho * region.alto as usize * C, "destino de tamaño incorrecto");

        if self.neutro {
            self.recorrer::<C, false, false, false, false>(origen, region, destino, encuadre);
            return;
        }
        // Un recorrido especializado por combinación de ajustes activos: el
        // compilador elimina las ramas que no aplican en cada píxel.
        match (self.solo_tabla, self.tono.is_some(), self.saturacion != 1.0, self.vineta != 0.0) {
            (true, ..) => self.recorrer::<C, true, false, false, false>(origen, region, destino, encuadre),
            (_, false, false, true) => self.recorrer::<C, true, false, false, true>(origen, region, destino, encuadre),
            (_, false, true, false) => self.recorrer::<C, true, false, true, false>(origen, region, destino, encuadre),
            (_, false, true, true) => self.recorrer::<C, true, false, true, true>(origen, region, destino, encuadre),
            (_, true, false, false) => self.recorrer::<C, true, true, false, false>(origen, region, destino, encuadre),
            (_, true, false, true) => self.recorrer::<C, true, true, false, true>(origen, region, destino, encuadre),
            (_, true, true, false) => self.recorrer::<C, true, true, true, false>(origen, region, destino, encuadre),
            (_, true, true, true) => self.recorrer::<C, true, true, true, true>(origen, region, destino, encuadre),
            (false, false, false, false) => unreachable!("sin ajustes por píxel es solo tabla"),
        }
    }

    /// Recorre la región. `TABLA`: aplicar la tabla por canal (si no, copia);
    /// `TONO`, `SAT`, `VIN`: luces/sombras, saturación y viñeta.
    #[inline(never)]
    fn recorrer<const C: usize, const TABLA: bool, const TONO: bool, const SAT: bool, const VIN: bool>(
        &self,
        origen: &RgbImage,
        region: Region,
        destino: &mut [u8],
        encuadre: Option<Encuadre>,
    ) {
        let ancho_origen = origen.width() as usize;
        let datos = origen.as_raw();
        let ancho = region.ancho as usize;

        // Distancias al centro del encuadre al cuadrado, por columna y por fila,
        // ya escaladas al índice de la tabla de viñeta.
        let e = encuadre.unwrap_or_else(|| Encuadre::de_region(region));
        let distancia = |i: usize, c: f32, r: f32| {
            let v = (i as f32 + 0.5 - c) / r.max(1.0);
            v * v / 2.0 * ESCALA_VINETA
        };
        let dx2: Vec<f32> = if VIN { (0..ancho).map(|i| distancia(i, e.cx, e.rx)).collect() } else { Vec::new() };
        let tono = self.tono.as_deref();
        let peso_vineta = self.peso_vineta.as_deref();
        let oscurecer = self.vineta > 0.0;
        let amplitud = self.amplitud_grano / 255.0;

        for (fila, salida) in destino.chunks_exact_mut(ancho * C).enumerate() {
            let inicio = ((region.y as usize + fila) * ancho_origen + region.x as usize) * 3;
            let entrada = &datos[inicio..inicio + ancho * 3];
            let dy2 = if VIN { distancia(fila, e.cy, e.ry) } else { 0.0 };
            let y_grano = region.y + fila as u32;

            for (col, (p, q)) in entrada.chunks_exact(3).zip(salida.chunks_exact_mut(C)).enumerate() {
                if !TABLA {
                    q[..3].copy_from_slice(p);
                } else {
                    let mut r = self.tabla[0][p[0] as usize];
                    let mut g = self.tabla[1][p[1] as usize];
                    let mut b = self.tabla[2][p[2] as usize];
                    if TONO {
                        let y = (LUMA[0] * r + LUMA[1] * g + LUMA[2] * b).clamp(0.0, 1.0);
                        let d = tono.unwrap()[(y * PASOS_TONO as f32 + 0.5) as usize];
                        r += d;
                        g += d;
                        b += d;
                    }
                    if SAT {
                        let y = LUMA[0] * r + LUMA[1] * g + LUMA[2] * b;
                        r = y + (r - y) * self.saturacion;
                        g = y + (g - y) * self.saturacion;
                        b = y + (b - y) * self.saturacion;
                    }
                    if VIN {
                        let i = ((dx2[col] + dy2) as usize).min(PASOS_VINETA);
                        let peso = peso_vineta.unwrap()[i];
                        if oscurecer {
                            let k = 1.0 - peso;
                            r *= k;
                            g *= k;
                            b *= k;
                        } else {
                            r += (1.0 - r) * peso;
                            g += (1.0 - g) * peso;
                            b += (1.0 - b) * peso;
                        }
                    }
                    if amplitud > 0.0 {
                        let d = grano_en(region.x + col as u32, y_grano, LUMA[0] * r + LUMA[1] * g + LUMA[2] * b) * amplitud;
                        r += d;
                        g += d;
                        b += d;
                    }
                    q[0] = a_u8(r);
                    q[1] = a_u8(g);
                    q[2] = a_u8(b);
                }
                if C == 4 {
                    q[3] = 255;
                }
            }
        }
    }

    /// Atajo: aplica los ajustes a toda la imagen y devuelve una imagen nueva.
    pub fn aplicar_imagen(&self, origen: &RgbImage) -> RgbImage {
        let mut salida = vec![0u8; origen.as_raw().len()];
        self.aplicar::<3>(origen, Region::completa(origen), &mut salida, None);
        RgbImage::from_raw(origen.width(), origen.height(), salida).expect("tamaño exacto")
    }
}

/// Grano de película monocromo, sobre todo en los tonos medios. Es determinista
/// (depende solo de x, y), así que la vista previa y las pruebas se repiten igual.
/// Se aplica a la medida final, después del enfoque.
pub fn aplicar_grano(datos: &mut [u8], ancho: u32, canales: usize, grano: f32) {
    let amplitud = amplitud_grano(grano);
    if amplitud == 0.0 {
        return;
    }
    for (y, fila) in datos.chunks_exact_mut(ancho as usize * canales).enumerate() {
        for (x, p) in fila.chunks_exact_mut(canales).enumerate() {
            let luma = (LUMA[0] * f32::from(p[0]) + LUMA[1] * f32::from(p[1]) + LUMA[2] * f32::from(p[2])) / 255.0;
            let d = grano_en(x as u32, y as u32, luma) * amplitud + 0.5;
            p[0] = (f32::from(p[0]) + d) as u8;
            p[1] = (f32::from(p[1]) + d) as u8;
            p[2] = (f32::from(p[2]) + d) as u8;
        }
    }
}

/// Amplitud máxima del grano en niveles de 0 a 255.
fn amplitud_grano(grano: f32) -> f32 {
    let grano = if grano.is_finite() { grano.clamp(0.0, 100.0) } else { 0.0 };
    grano / 100.0 * 0.12 * 255.0
}

/// Ruido de grano en (x, y), de −1 a 1, atenuado en negros y blancos según la luma (0 a 1).
#[inline(always)]
fn grano_en(x: u32, y: u32, luma: f32) -> f32 {
    let h = mezclar(x, y);
    // Suma de dos uniformes: ruido triangular de −1 a 1, más natural.
    let ruido = ((h & 0xFFFF) as f32 + (h >> 16) as f32) * (1.0 / 65535.0) - 1.0;
    let l = luma * 2.0 - 1.0;
    ruido * (1.0 - l * l * 0.7)
}

/// Ganancias por canal para temperatura (`t`) y tinte (`m`), de −1 a 1.
/// La temperatura se normaliza para que el blanco conserve su luminancia; el
/// tinte no, porque la luma depende casi toda del verde y la normalización
/// anularía el efecto.
fn ganancias_balance(t: f32, m: f32) -> [f32; 3] {
    if t == 0.0 && m == 0.0 {
        return [1.0; 3];
    }
    let temperatura = [2f32.powf(0.30 * t), 1.0, 2f32.powf(-0.30 * t)];
    let y = LUMA[0] * temperatura[0] + LUMA[1] * temperatura[1] + LUMA[2] * temperatura[2];
    let tinte = [2f32.powf(0.15 * m), 2f32.powf(-0.35 * m), 2f32.powf(0.15 * m)];
    [0, 1, 2].map(|c| temperatura[c] / y * tinte[c])
}

/// Curva en S suave con pendiente 2 en el gris medio y 0 en los extremos.
fn curva_s(x: f32) -> f32 {
    if x < 0.5 { 2.0 * x * x } else { 1.0 - 2.0 * (1.0 - x) * (1.0 - x) }
}

#[inline(always)]
fn suave(desde: f32, hasta: f32, x: f32) -> f32 {
    let t = ((x - desde) / (hasta - desde)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn srgb_a_lineal(x: f32) -> f32 {
    if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
}

fn lineal_a_srgb(x: f32) -> f32 {
    if x <= 0.003_130_8 { x * 12.92 } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 }
}

/// Hash entero de dos coordenadas (para el grano).
#[inline(always)]
fn mezclar(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x8DA6_B343) ^ y.wrapping_mul(0xD816_3841);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 13)
}

/// De 0–1 a 0–255 con redondeo. `as u8` satura, y un NaN da 0.
#[inline(always)]
fn a_u8(v: f32) -> u8 {
    (v * 255.0 + 0.5) as u8
}

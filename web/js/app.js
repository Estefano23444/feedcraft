// Interfaz de FeedCraft: decide qué pedir al motor y muestra el resultado.
// Todo el trabajo de píxeles ocurre en el Web Worker (RNF-02).

import { crearPuente } from './puente.js';
import { AJUSTES_CERO, crearControles, pintarPista } from './ajustes.js';
import { FORMATOS, calcularMarco, crearMarco, formatoPorId } from './recorte.js';

const LADO_VISTA = 1600;
const MAX_BYTES = 50 * 1024 * 1024;
const TIPOS = ['image/jpeg', 'image/png', 'image/webp'];

const $ = (id) => document.getElementById(id);
const puente = crearPuente();

const estado = {
  info: null,
  nombre: 'foto',
  ajustes: { ...AJUSTES_CERO },
  original: false,
  formato: '4:5',
  centro: { cx: 0.5, cy: 0.5 },
  zoom: 1,
  ampliar: false,
  enfoque: 30,
  calidad: 92,
  /** Id del estilo aplicado (RF-17); vacío si no hay ninguno. */
  estilo: '',
  catalogo: [],
};

/** Marco actual en coordenadas normalizadas, derivado de formato, centro y zoom. */
function marcoActual() {
  const { ancho, alto } = estado.info;
  return calcularMarco(ancho, alto, estado.formato, estado.centro.cx, estado.centro.cy, estado.zoom);
}

function parametrosExportacion() {
  return {
    formato: estado.formato,
    marco: marcoActual(),
    ajustes: estado.ajustes,
    ampliar: estado.ampliar,
    enfoque: estado.enfoque,
    calidad: estado.calidad,
    nombre_base: estado.nombre,
    estilo: estado.estilo,
  };
}

// ---------- Carga (RF-01) ----------

$('archivo').addEventListener('change', (e) => {
  cargarArchivo(e.target.files[0]);
  e.target.value = '';
});
$('btn-nueva').addEventListener('click', () => $('archivo').click());

document.addEventListener('dragover', (e) => {
  e.preventDefault();
  document.body.classList.add('arrastrando');
});
document.addEventListener('dragleave', (e) => {
  if (!e.relatedTarget) document.body.classList.remove('arrastrando');
});
document.addEventListener('drop', (e) => {
  e.preventDefault();
  document.body.classList.remove('arrastrando');
  cargarArchivo(e.dataTransfer.files[0]);
});

async function cargarArchivo(archivo) {
  if (!archivo) return;
  if (archivo.size > MAX_BYTES) {
    avisar(`«${archivo.name}» pesa ${(archivo.size / 1048576).toFixed(1)} MB; el máximo es 50 MB.`, true);
    return;
  }
  if (archivo.type && !TIPOS.includes(archivo.type)) {
    avisar(`«${archivo.name}» no es JPEG, PNG ni WebP.`, true);
    return;
  }

  mostrarEditor(true);
  ocupado(true, 'Abriendo la foto…');
  try {
    const bytes = await archivo.arrayBuffer();
    const info = await puente.llamar('cargar', { bytes, lado: LADO_VISTA }, [bytes]);
    estado.info = info;
    estado.nombre = nombreBase(archivo.name);
    estado.ajustes = { ...AJUSTES_CERO };
    estado.estilo = '';
    marcarEstilo();
    estado.centro = { cx: 0.5, cy: 0.5 };
    estado.zoom = 1;
    estado.ampliar = false;
    estado.original = false;
    controles.mostrar(estado.ajustes);
    $('btn-original').setAttribute('aria-pressed', 'false');
    $('zoom').value = '100';
    $('zoom-valor').textContent = '100 %';
    pintarPista($('zoom'));
    prepararLienzo(info);
    actualizarRecorte();
    pedirVista();
  } catch (e) {
    avisar(e.message, true);
    if (!estado.info) mostrarEditor(false);
  } finally {
    ocupado(false);
  }
}

/** RF-15: nombre de archivo limpio, sin tildes ni espacios. */
function nombreBase(nombre) {
  const base = nombre
    .replace(/\.[^.]+$/, '')
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40);
  return base || 'foto';
}

function prepararLienzo(info) {
  const lienzo = $('lienzo');
  lienzo.width = info.vista_ancho;
  lienzo.height = info.vista_alto;
  const partes = [`${info.ancho} × ${info.alto} px`, info.formato.toUpperCase()];
  if (info.orientacion !== 1) partes.push('enderezada según EXIF');
  if (info.perfil_convertido) partes.push(`${info.perfil_convertido} → sRGB`);
  if (info.tenia_transparencia) partes.push('transparencia sobre blanco');
  $('info-foto').textContent = partes.join(' · ');
}

// ---------- Formato y recorte (RF-04, RF-06, RF-08) ----------

for (const f of FORMATOS) {
  const boton = document.createElement('button');
  boton.type = 'button';
  boton.dataset.formato = f.id;
  boton.setAttribute('aria-pressed', String(f.id === estado.formato));
  const escala = 18 / f.alto;
  boton.innerHTML = `<span class="forma" style="width:${Math.round(f.ancho * escala)}px;height:18px"></span>${f.id}<small>${f.ancho}×${f.alto}</small>`;
  boton.addEventListener('click', () => elegirFormato(f.id));
  $('formatos').append(boton);
}

function elegirFormato(id) {
  estado.formato = id;
  for (const b of $('formatos').children) b.setAttribute('aria-pressed', String(b.dataset.formato === id));
  if (estado.info) actualizarRecorte();
}

const marco = crearMarco($('lienzo-caja'), {
  alMover(centro) {
    estado.centro = centro;
    // La viñeta sigue al encuadre.
    if (estado.ajustes.vineta !== 0) pedirVista();
  },
  alSoltar() {
    actualizarRecorte();
  },
  alZoom(delta) {
    fijarZoom(estado.zoom + delta);
  },
});

$('zoom').addEventListener('input', (e) => fijarZoom(Number(e.target.value) / 100));

function fijarZoom(zoom) {
  estado.zoom = Math.min(Math.max(zoom, 1), 3);
  $('zoom').value = String(Math.round(estado.zoom * 100));
  $('zoom-valor').textContent = `${Math.round(estado.zoom * 100)} %`;
  pintarPista($('zoom'));
  if (estado.info) actualizarRecorte();
}

$('enfoque').addEventListener('input', (e) => {
  estado.enfoque = Number(e.target.value);
  $('enfoque-valor').textContent = e.target.value;
  pintarPista(e.target);
});
$('calidad').addEventListener('input', (e) => {
  estado.calidad = Number(e.target.value);
  $('calidad-valor').textContent = e.target.value;
  pintarPista(e.target);
});
for (const id of ['enfoque', 'calidad', 'zoom']) pintarPista($(id));

// RF-11: comparar con la foto sin ajustes.
$('btn-original').addEventListener('click', () => {
  estado.original = !estado.original;
  $('btn-original').setAttribute('aria-pressed', String(estado.original));
  pedirVista();
});

for (const boton of $('aviso-pequena').querySelectorAll('[data-ampliar]')) {
  boton.addEventListener('click', () => {
    estado.ampliar = boton.dataset.ampliar === 'true';
    evaluarSalida();
  });
}

/** Recalcula el marco con el centro, el formato y el zoom actuales. */
function actualizarRecorte() {
  const m = marcoActual();
  estado.centro = { cx: m.x + m.ancho / 2, cy: m.y + m.alto / 2 };
  marco.fijar(m);
  evaluarSalida();
  if (estado.ajustes.vineta !== 0) pedirVista();
}

/** Pregunta al motor qué medidas tendrá la exportación (RF-08). */
async function evaluarSalida() {
  try {
    const s = await puente.llamar('evaluar', parametrosExportacion());
    const f = formatoPorId(estado.formato);
    $('aviso-pequena').hidden = !s.pequena;
    if (s.pequena) {
      const recorte = Math.floor(marcoActual().ancho * estado.info.ancho);
      $('aviso-pequena-texto').textContent =
        `El recorte mide ${recorte} px de ancho y Instagram usa ${f.ancho} × ${f.alto}. ` +
        'Ampliarla no añade detalle: puede verse menos nítida.';
      for (const b of $('aviso-pequena').querySelectorAll('[data-ampliar]')) {
        b.setAttribute('aria-pressed', String((b.dataset.ampliar === 'true') === estado.ampliar));
      }
    }
    $('medidas-salida').textContent = `${s.ancho} × ${s.alto} px · JPEG${s.ampliada ? ' · ampliada' : ''}`;
  } catch (e) {
    avisar(e.message, true);
  }
}

// ---------- Ajustes (RF-09) ----------

const controles = crearControles($('ajustes'), (id, valor) => {
  estado.ajustes[id] = valor;
  pedirVista();
});

$('btn-restablecer').addEventListener('click', () => {
  estado.ajustes = { ...AJUSTES_CERO };
  estado.estilo = '';
  controles.mostrar(estado.ajustes);
  marcarEstilo();
  pedirVista();
});

// ---------- Estilos (RF-17) ----------

async function cargarCatalogo() {
  let json = null;
  try {
    const respuesta = await fetch('estilos.json', { cache: 'no-store' });
    if (respuesta.ok) json = await respuesta.text();
  } catch {
    // Sin red: el worker usa el catálogo incrustado.
  }
  try {
    const { estilos, aviso } = await puente.llamar('catalogo', { json });
    estado.catalogo = estilos;
    pintarEstilos();
    if (aviso) avisar(`${aviso.replace(/\.?$/, '.')} Se usa el catálogo incluido en la app.`, true);
  } catch (e) {
    avisar(e.message, true);
  }
}

function pintarEstilos() {
  const contenedor = $('estilos');
  contenedor.innerHTML = '';
  for (const e of estado.catalogo) {
    const chip = document.createElement('button');
    chip.type = 'button';
    chip.className = 'chip';
    chip.dataset.estilo = e.id;
    chip.textContent = e.nombre;
    chip.title = e.descripcion;
    chip.addEventListener('click', () => aplicarEstilo(e.id));
    contenedor.append(chip);
  }
  marcarEstilo();
}

function aplicarEstilo(id) {
  const e = estado.catalogo.find((x) => x.id === id);
  if (!e) return;
  estado.estilo = id;
  estado.ajustes = { ...AJUSTES_CERO, ...e.ajustes };
  controles.mostrar(estado.ajustes);
  marcarEstilo();
  pedirVista();
}

function marcarEstilo() {
  for (const chip of $('estilos').children) chip.setAttribute('aria-pressed', String(chip.dataset.estilo === estado.estilo));
  const e = estado.catalogo.find((x) => x.id === estado.estilo);
  $('descripcion-estilo').textContent = e ? e.descripcion : 'Aplica un estilo con un clic y ajústalo a tu gusto.';
}

cargarCatalogo();

// ---------- Vista previa en vivo (RF-10) ----------
// Si llegan cambios mientras el motor trabaja, solo se procesa el último.

let vistaEnCurso = false;
let vistaPedida = false;

function pedirVista() {
  if (!estado.info) return;
  vistaPedida = true;
  if (!vistaEnCurso) procesarVistas();
}

async function procesarVistas() {
  vistaEnCurso = true;
  while (vistaPedida) {
    vistaPedida = false;
    const inicio = performance.now();
    try {
      const r = await puente.llamar('vista', { ajustes: estado.ajustes, marco: marcoActual(), original: estado.original });
      const imagen = new ImageData(new Uint8ClampedArray(r.rgba), r.ancho, r.alto);
      $('lienzo').getContext('2d').putImageData(imagen, 0, 0);
      $('tiempo-vista').textContent =
        `Vista previa: ${Math.round(performance.now() - inicio)} ms (motor ${Math.round(r.ms)} ms)`;
    } catch (e) {
      avisar(e.message, true);
    }
  }
  vistaEnCurso = false;
}

// ---------- Exportación ----------

$('btn-exportar').addEventListener('click', async () => {
  if (!estado.info) return;
  const boton = $('btn-exportar');
  boton.disabled = true;
  ocupado(true, 'Exportando a resolución completa…');
  const inicio = performance.now();
  try {
    const { jpeg, nombre } = await puente.llamar('exportar', parametrosExportacion());
    descargar(new Blob([jpeg], { type: 'image/jpeg' }), nombre);
    avisar(`Listo: ${nombre} (${((performance.now() - inicio) / 1000).toFixed(1)} s)`);
  } catch (e) {
    avisar(e.message, true);
  } finally {
    ocupado(false);
    boton.disabled = false;
  }
});

function descargar(blob, nombre) {
  const url = URL.createObjectURL(blob);
  const enlace = Object.assign(document.createElement('a'), { href: url, download: nombre });
  document.body.append(enlace);
  enlace.click();
  enlace.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

// ---------- Utilidades de interfaz ----------

function mostrarEditor(visible) {
  $('inicio').hidden = visible;
  $('editor').hidden = !visible;
  $('btn-nueva').hidden = !visible;
}

function ocupado(activo, texto = '') {
  $('cargando').hidden = !activo;
  if (texto) $('cargando-texto').textContent = texto;
}

let temporizadorAviso;
function avisar(texto, error = false) {
  const toast = $('toast');
  toast.textContent = texto;
  toast.classList.toggle('error', error);
  toast.hidden = false;
  clearTimeout(temporizadorAviso);
  temporizadorAviso = setTimeout(() => (toast.hidden = true), error ? 8000 : 4000);
}

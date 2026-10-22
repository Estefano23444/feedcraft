// Interfaz de FeedCraft: decide qué pedir al motor y muestra el resultado.
// Todo el trabajo de píxeles ocurre en el Web Worker (RNF-02).

import { crearPuente } from './puente.js';
import { AJUSTES_CERO, crearControles } from './ajustes.js';

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
};

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
    controles.mostrar(estado.ajustes);
    prepararLienzo(info);
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

// ---------- Ajustes (RF-09) ----------

const controles = crearControles($('ajustes'), (id, valor) => {
  estado.ajustes[id] = valor;
  pedirVista();
});

$('btn-restablecer').addEventListener('click', () => {
  estado.ajustes = { ...AJUSTES_CERO };
  controles.mostrar(estado.ajustes);
  pedirVista();
});

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
      const r = await puente.llamar('vista', { ajustes: estado.ajustes, original: estado.original });
      const imagen = new ImageData(new Uint8ClampedArray(r.rgba), r.ancho, r.alto);
      $('lienzo').getContext('2d').putImageData(imagen, 0, 0);
      $('tiempo-vista').textContent = `Vista previa: ${Math.round(performance.now() - inicio)} ms`;
    } catch (e) {
      avisar(e.message, true);
    }
  }
  vistaEnCurso = false;
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

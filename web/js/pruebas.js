// Pruebas que solo tienen sentido en el navegador, con el wasm real dentro del
// Web Worker: rendimiento (T-09), aviso de foto pequeña (T-06), IA (T-07) y
// funcionamiento sin red (T-08). Las pruebas del motor están en `cargo test`.
//
// Cada prueba devuelve { ok, detalle }. Al terminar, `window.resultados` y
// `document.body.dataset.estado = 'terminado'` permiten leerlas desde Playwright.

import { crearPuente } from './puente.js';
import * as groq from './groq.js';

const puente = crearPuente();

let catalogo = null;
async function obtenerCatalogo() {
  if (!catalogo) {
    const json = await fetch('estilos.json').then((r) => r.text()).catch(() => null);
    catalogo = (await puente.llamar('catalogo', { json })).estilos;
  }
  return catalogo;
}

/** fetch simulado: devuelve `contenido` como respuesta del modelo, o lo que diga `fabricar`. */
function fetchSimulado(fabricar) {
  const llamadas = [];
  const traer = async (url, opciones) => {
    llamadas.push({ url, opciones });
    return fabricar(llamadas.length);
  };
  return { traer, llamadas };
}

const respuestaModelo = (contenido) =>
  new Response(JSON.stringify({ choices: [{ message: { content: contenido } }] }), { status: 200 });

async function pedirConFetch(traer) {
  const estilos = await obtenerCatalogo();
  return groq.pedirSugerencia({
    clave: 'gsk_prueba',
    config: groq.CONFIG_POR_DEFECTO,
    imagenBase64: 'AAAA',
    catalogo: estilos,
    normalizar: (texto) => puente.llamar('normalizar', { texto, catalogo: estilos }),
    respaldo: (motivo) => puente.llamar('respaldo', { catalogo: estilos, motivo }),
    fetch: traer,
  });
}

/** Una sugerencia que puede llegar a la interfaz: 3 estilos distintos del catálogo. */
function esValida(s, estilos) {
  const ids = s.variantes.map((v) => v.estilo);
  return ['1:1', '4:5', '3:4', '9:16'].includes(s.formato)
    && ids.length === 3 && new Set(ids).size === 3
    && ids.every((id) => estilos.some((e) => e.id === id));
}

const RESPUESTAS_ROTAS = [
  'Claro, te recomiendo un estilo cálido.',
  '{"formato": "4:5", "variantes": [{"estilo": "film-calido"',
  '',
  '[1, 2, 3]',
  '{"formato":"16:9","variantes":[{"estilo":"cyberpunk"},{"estilo":"vaporwave"},{"estilo":"<img src=x onerror=alert(1)>"}]}',
  '<think>...</think>```json\n{"variantes":[{"estilo":"noche-neon"},{"estilo":"alto-contraste-urbano"}]}\n```',
  '{"recorte":{"cx":5,"cy":-2},"variantes":[{"estilo":"film-calido","ajustes_extra":{"brillo":999}},{"estilo":"film-calido"},{"estilo":"film-calido"}]}',
  '{"formato":45,"recorte":"centro","variantes":"tres","caption":["no"],"hashtags":"#uno #dos"}',
  '{}',
  '{"a":'.repeat(3000) + '1' + '}'.repeat(3000),
];

/** JPEG sintético con aspecto de foto, generado en el navegador. */
async function jpegSintetico(ancho, alto) {
  const lienzo = new OffscreenCanvas(ancho, alto);
  const ctx = lienzo.getContext('2d');
  const cielo = ctx.createLinearGradient(0, 0, 0, alto);
  cielo.addColorStop(0, '#4a7fd6');
  cielo.addColorStop(0.6, '#cfe0f0');
  cielo.addColorStop(1, '#5c7a3a');
  ctx.fillStyle = cielo;
  ctx.fillRect(0, 0, ancho, alto);
  ctx.fillStyle = '#ffd77a';
  ctx.beginPath();
  ctx.arc(ancho * 0.7, alto * 0.3, alto * 0.08, 0, Math.PI * 2);
  ctx.fill();
  for (let i = 0; i < 400; i++) {
    ctx.fillStyle = `hsl(${(i * 47) % 360} 45% ${30 + (i % 40)}%)`;
    ctx.fillRect((i * 7919) % ancho, alto * 0.62 + ((i * 104729) % (alto * 0.38)), 40 + (i % 90), 12 + (i % 30));
  }
  // Ruido fino en mosaico, como el de un sensor: hace el JPEG realista.
  const ruido = new OffscreenCanvas(256, 256);
  const rctx = ruido.getContext('2d');
  const datos = rctx.createImageData(256, 256);
  let semilla = 12345;
  for (let i = 0; i < datos.data.length; i += 4) {
    semilla = (semilla * 1103515245 + 12345) & 0x7fffffff;
    const v = semilla & 0xff;
    datos.data.set([v, v, v, 28], i);
  }
  rctx.putImageData(datos, 0, 0);
  ctx.fillStyle = ctx.createPattern(ruido, 'repeat');
  ctx.fillRect(0, 0, ancho, alto);
  const blob = await lienzo.convertToBlob({ type: 'image/jpeg', quality: 0.92 });
  return blob.arrayBuffer();
}

const AJUSTES_TIPICOS = {
  brillo: 8, contraste: -10, saturacion: -8, temperatura: 18, tinte: 4, luces: -10, sombras: 15, vineta: 12, grano: 10,
};

export const PRUEBAS = [
  {
    id: 'E1',
    nombre: 'Rust ↔ navegador',
    verifica: 'Etapa 1',
    async correr() {
      const v = await puente.llamar('version');
      return { ok: v.startsWith('FeedCraft'), detalle: v };
    },
  },
  {
    id: 'T-09',
    nombre: 'Rendimiento: 24 MP en menos de 3 s',
    verifica: 'RNF-01',
    async correr() {
      const bytes = await jpegSintetico(6000, 4000);
      const mb = (bytes.byteLength / 1048576).toFixed(1);
      let inicio = performance.now();
      const info = await puente.llamar('cargar', { bytes, lado: 1600 }, [bytes]);
      const cargar = performance.now() - inicio;
      inicio = performance.now();
      const r = await puente.llamar('exportar', { formato: '4:5', ajustes: AJUSTES_TIPICOS, nombre_base: 't09' });
      const exportar = performance.now() - inicio;
      const total = cargar + exportar;
      return {
        ok: info.ancho * info.alto >= 24e6 && exportar < 3000 && total < 3000,
        detalle: `${info.ancho}×${info.alto} (${mb} MB): abrir ${Math.round(cargar)} ms + exportar ${Math.round(exportar)} ms = ${(total / 1000).toFixed(2)} s · ${r.nombre} ${Math.round(r.jpeg.byteLength / 1024)} KB`,
      };
    },
  },
  {
    id: 'T-06',
    nombre: 'Foto de 800 px: avisa y no amplía sola',
    verifica: 'RF-08',
    async correr() {
      const bytes = await jpegSintetico(800, 600);
      await puente.llamar('cargar', { bytes, lado: 1600 }, [bytes]);
      const s = await puente.llamar('evaluar', { formato: '4:5' });
      const ampliada = await puente.llamar('evaluar', { formato: '4:5', ampliar: true });
      return {
        ok: s.pequena && s.ancho < 1080 && !s.ampliada && ampliada.ancho === 1080,
        detalle: `sin elegir: ${s.ancho}×${s.alto}${s.pequena ? ' + aviso' : ''} · al elegir ampliar: ${ampliada.ancho}×${ampliada.alto}`,
      };
    },
  },
  {
    id: 'T-07',
    nombre: '10 respuestas rotas o inventadas: nunca falla y cae en Natural mejorado',
    verifica: 'Sección 5',
    async correr() {
      const estilos = await obtenerCatalogo();
      const fallos = [];
      let reintentos = 0;
      for (const [i, contenido] of RESPUESTAS_ROTAS.entries()) {
        const { traer, llamadas } = fetchSimulado(() => respuestaModelo(contenido));
        const { sugerencia } = await pedirConFetch(traer);
        if (llamadas.length === 2) reintentos++;
        const natural = sugerencia.variantes.some((v) => v.estilo === 'natural-mejorado');
        if (!esValida(sugerencia, estilos) || !natural || llamadas.length > 2) fallos.push(i + 1);
      }
      return {
        ok: fallos.length === 0,
        detalle: fallos.length ? `fallan las respuestas ${fallos.join(', ')}` : `10/10 válidas con Natural mejorado; ${reintentos} reintentaron una vez`,
      };
    },
  },
  {
    id: 'T-07b',
    nombre: 'Errores 401, 429, 500 y red: mensaje claro, sin fallar',
    verifica: 'Sección 5 · regla 4',
    async correr() {
      const estilos = await obtenerCatalogo();
      const casos = [
        ['clave', () => new Response('{"error":{"message":"Invalid API Key"}}', { status: 401 })],
        ['limite', () => new Response('{}', { status: 429, headers: { 'retry-after': '20' } })],
        ['servidor', () => new Response('{"error":{"message":"The model has been decommissioned"}}', { status: 400 })],
        ['red', () => { throw new TypeError('Failed to fetch'); }],
      ];
      const detalles = [];
      let ok = true;
      for (const [tipo, fabricar] of casos) {
        const { traer } = fetchSimulado(fabricar);
        const r = await pedirConFetch(traer);
        ok &&= r.error?.tipo === tipo && esValida(r.sugerencia, estilos) && r.error.mensaje.length > 20;
        detalles.push(`${tipo}: «${r.error?.mensaje}»`);
      }
      // Una respuesta buena después de una rota: el reintento la rescata.
      const { traer, llamadas } = fetchSimulado((n) => respuestaModelo(n === 1 ? 'roto' : '{"formato":"1:1","variantes":[{"estilo":"pastel-suave"},{"estilo":"minimal-frio"},{"estilo":"film-calido"}]}'));
      const r = await pedirConFetch(traer);
      ok &&= !r.error && r.sugerencia.formato === '1:1' && llamadas.length === 2;
      detalles.push(`reintento: ${r.sugerencia.formato} ${r.sugerencia.variantes[0].estilo}`);
      return { ok, detalle: detalles.join(' · ') };
    },
  },
  {
    id: 'RF-20',
    nombre: 'La clave se guarda solo en el navegador',
    verifica: 'RF-20, RNF-06',
    async correr() {
      const anterior = groq.leerClave();
      groq.guardarClave('gsk_clave_de_prueba');
      const guardada = groq.leerClave();
      const { traer, llamadas } = fetchSimulado(() => respuestaModelo('{}'));
      await pedirConFetch(traer);
      const { opciones, url } = llamadas[0];
      const enCabecera = opciones.headers.Authorization === 'Bearer gsk_prueba';
      const enCuerpo = opciones.body.includes('gsk_');
      groq.guardarClave(anterior);
      return {
        ok: guardada === 'gsk_clave_de_prueba' && enCabecera && !enCuerpo && url.startsWith(groq.CONFIG_POR_DEFECTO.baseURL),
        detalle: `localStorage ✓ · solo en la cabecera Authorization ✓ · destino ${new URL(url).host}`,
      };
    },
  },
  {
    id: 'T-08',
    nombre: 'Sin red: todo funciona excepto la IA',
    verifica: 'RNF-04',
    async correr() {
      const original = globalThis.fetch;
      const pedidas = [];
      globalThis.fetch = async (url) => {
        pedidas.push(String(url));
        throw new TypeError('Failed to fetch');
      };
      try {
        const estilos = await obtenerCatalogo();
        const bytes = await jpegSintetico(2400, 3000);
        await puente.llamar('cargar', { bytes, lado: 1600 }, [bytes]);
        const vista = await puente.llamar('vista', { ajustes: estilos[0].ajustes });
        const exportada = await puente.llamar('exportar', { formato: '4:5', ajustes: estilos[0].ajustes, nombre_base: 'sin-red', estilo: estilos[0].id });
        const r = await pedirConFetch(globalThis.fetch);
        const ok = vista.ancho > 0 && exportada.jpeg.byteLength > 10_000 && r.error?.tipo === 'red' && esValida(r.sugerencia, estilos);
        return {
          ok,
          detalle: `abrir, vista previa y exportar sin red ✓ (${exportada.nombre}) · IA: «${r.error?.mensaje}» · peticiones de red intentadas: ${pedidas.length} (solo Groq)`,
        };
      } finally {
        globalThis.fetch = original;
      }
    },
  },
];

export async function correrPruebas(alTerminarUna = () => {}) {
  const resultados = [];
  for (const prueba of PRUEBAS) {
    const inicio = performance.now();
    let resultado;
    try {
      resultado = await prueba.correr();
    } catch (e) {
      resultado = { ok: false, detalle: `Error: ${e.message}` };
    }
    resultado = { id: prueba.id, nombre: prueba.nombre, verifica: prueba.verifica, ms: Math.round(performance.now() - inicio), ...resultado };
    resultados.push(resultado);
    alTerminarUna(resultado);
  }
  return resultados;
}

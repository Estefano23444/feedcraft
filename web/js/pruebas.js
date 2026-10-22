// Pruebas que solo tienen sentido en el navegador, con el wasm real dentro del
// Web Worker: rendimiento (T-09), aviso de foto pequeña (T-06), IA (T-07) y
// funcionamiento sin red (T-08). Las pruebas del motor están en `cargo test`.
//
// Cada prueba devuelve { ok, detalle }. Al terminar, `window.resultados` y
// `document.body.dataset.estado = 'terminado'` permiten leerlas desde Playwright.

import { crearPuente } from './puente.js';

const puente = crearPuente();

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

// Web Worker: carga el núcleo Rust y hace todo el trabajo pesado fuera del hilo
// de la interfaz (RNF-02). Protocolo: {id, tipo, datos} → {id, ok, resultado|error}.

import iniciar, * as nucleo from '../pkg/nucleo.js';

const listo = iniciar();

/** La foto abierta. Vive aquí, en la memoria del wasm, mientras se edita. */
let editor = null;

function exigirFoto() {
  if (!editor) throw new Error('Primero abre una foto.');
  return editor;
}

function aBase64(bytes) {
  let binario = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binario += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  }
  return btoa(binario);
}

const manejadores = {
  version: () => ({ valor: nucleo.version() }),

  /** Valida estilos.json; si falta o no es válido, usa el catálogo incrustado. */
  catalogo({ json }) {
    let aviso = json ? null : 'No se pudo leer estilos.json.';
    if (json) {
      try {
        return { valor: { estilos: JSON.parse(nucleo.cargar_catalogo(json)), aviso } };
      } catch (e) {
        aviso = e.message;
      }
    }
    return { valor: { estilos: JSON.parse(nucleo.catalogo_incrustado()), aviso } };
  },

  cargar({ bytes, lado }) {
    const inicio = performance.now();
    editor?.free();
    editor = null;
    editor = new nucleo.Editor(new Uint8Array(bytes), lado);
    const info = JSON.parse(editor.info());
    info.ms = Math.round(performance.now() - inicio);
    return { valor: info };
  },

  vista(parametros) {
    const ed = exigirFoto();
    const inicio = performance.now();
    const rgba = ed.vista_previa(JSON.stringify(parametros));
    return {
      valor: { rgba: rgba.buffer, ancho: ed.vista_ancho(), alto: ed.vista_alto(), ms: performance.now() - inicio },
      transferir: [rgba.buffer],
    };
  },

  /** Copia de 512 px en base64 para la IA (sección 5). */
  copiaIa() {
    return { valor: aBase64(exigirFoto().copia_ia()) };
  },

  normalizar({ texto, catalogo }) {
    return { valor: JSON.parse(nucleo.normalizar_sugerencia(texto, JSON.stringify(catalogo))) };
  },

  respaldo({ catalogo, motivo }) {
    return { valor: JSON.parse(nucleo.sugerencia_respaldo(JSON.stringify(catalogo), motivo)) };
  },

  evaluar(parametros) {
    return { valor: JSON.parse(exigirFoto().evaluar(JSON.stringify(parametros))) };
  },

  /** `parametros` incluye `nombre_base` y `estilo` para el nombre del archivo (RF-15). */
  exportar(parametros) {
    const inicio = performance.now();
    const jpeg = exigirFoto().exportar(JSON.stringify(parametros));
    const nombre = nucleo.nombre_archivo(parametros.nombre_base ?? 'foto', parametros.estilo ?? '', parametros.formato);
    return { valor: { jpeg: jpeg.buffer, nombre, ms: performance.now() - inicio }, transferir: [jpeg.buffer] };
  },
};

self.onmessage = async (evento) => {
  const { id, tipo, datos } = evento.data;
  try {
    await listo;
    const manejador = manejadores[tipo];
    if (!manejador) throw new Error(`Operación desconocida: ${tipo}`);
    const { valor, transferir = [] } = await manejador(datos);
    self.postMessage({ id, ok: true, resultado: valor }, transferir);
  } catch (error) {
    self.postMessage({ id, ok: false, error: error?.message ?? String(error) });
  }
};

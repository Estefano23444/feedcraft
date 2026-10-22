// Web Worker: carga el núcleo Rust y hace todo el trabajo pesado fuera del hilo
// de la interfaz (RNF-02). Protocolo: {id, tipo, datos} → {id, ok, resultado|error}.

import iniciar, * as nucleo from '../pkg/nucleo.js';

const listo = iniciar();

const manejadores = {
  version: () => ({ valor: nucleo.version() }),
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

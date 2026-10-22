// Puente con el Web Worker: cada llamada devuelve una promesa.

export function crearPuente(url = new URL('./worker.js', import.meta.url)) {
  const worker = new Worker(url, { type: 'module' });
  const pendientes = new Map();
  let siguiente = 1;

  worker.onmessage = ({ data }) => {
    const p = pendientes.get(data.id);
    if (!p) return;
    pendientes.delete(data.id);
    if (data.ok) p.resolver(data.resultado);
    else p.rechazar(new Error(data.error));
  };

  worker.onerror = (e) => {
    const error = new Error(`El motor no pudo arrancar: ${e.message ?? 'error desconocido'}`);
    for (const p of pendientes.values()) p.rechazar(error);
    pendientes.clear();
  };

  return {
    llamar(tipo, datos, transferir = []) {
      const id = siguiente++;
      return new Promise((resolver, rechazar) => {
        pendientes.set(id, { resolver, rechazar });
        worker.postMessage({ id, tipo, datos }, transferir);
      });
    },
  };
}

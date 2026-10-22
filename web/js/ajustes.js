// Controles deslizantes de los ajustes de color (RF-09).

export const AJUSTES = [
  { id: 'brillo', nombre: 'Brillo', min: -100, max: 100 },
];

export const AJUSTES_CERO = Object.fromEntries(AJUSTES.map((a) => [a.id, 0]));

/**
 * Crea un deslizador por ajuste dentro de `contenedor`. `alCambiar(id, valor)`
 * se llama en cada movimiento. Doble clic sobre el valor lo devuelve a 0.
 */
export function crearControles(contenedor, alCambiar, definiciones = AJUSTES) {
  const entradas = new Map();

  for (const def of definiciones) {
    const control = document.createElement('div');
    control.className = 'control';
    const idEntrada = `ajuste-${def.id}`;
    control.innerHTML = `
      <label for="${idEntrada}">${def.nombre}</label>
      <output for="${idEntrada}" title="Doble clic: volver a 0">0</output>
      <input id="${idEntrada}" type="range" min="${def.min}" max="${def.max}" step="1" value="0">`;
    const entrada = control.querySelector('input');
    const salida = control.querySelector('output');

    const fijar = (valor) => {
      entrada.value = String(valor);
      salida.textContent = valor > 0 ? `+${valor}` : String(valor);
    };
    entrada.addEventListener('input', () => {
      const valor = Number(entrada.value);
      fijar(valor);
      alCambiar(def.id, valor);
    });
    salida.addEventListener('dblclick', () => {
      fijar(0);
      alCambiar(def.id, 0);
    });

    entradas.set(def.id, fijar);
    contenedor.append(control);
  }

  return {
    /** Muestra `valores` sin disparar `alCambiar`. */
    mostrar(valores) {
      for (const [id, fijar] of entradas) fijar(Math.round(valores[id] ?? 0));
    },
  };
}

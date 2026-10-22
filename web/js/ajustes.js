// Controles deslizantes de los ajustes de color (RF-09).

export const AJUSTES = [
  { id: 'brillo', nombre: 'Brillo', grupo: 'Luz', min: -100, max: 100 },
  { id: 'contraste', nombre: 'Contraste', grupo: 'Luz', min: -100, max: 100 },
  { id: 'luces', nombre: 'Luces', grupo: 'Luz', min: -100, max: 100 },
  { id: 'sombras', nombre: 'Sombras', grupo: 'Luz', min: -100, max: 100 },
  { id: 'temperatura', nombre: 'Temperatura', grupo: 'Color', min: -100, max: 100 },
  { id: 'tinte', nombre: 'Tinte', grupo: 'Color', min: -100, max: 100 },
  { id: 'saturacion', nombre: 'Saturación', grupo: 'Color', min: -100, max: 100 },
  { id: 'vineta', nombre: 'Viñeta', grupo: 'Efectos', min: -100, max: 100 },
  { id: 'grano', nombre: 'Grano', grupo: 'Efectos', min: 0, max: 100 },
];

export const AJUSTES_CERO = Object.fromEntries(AJUSTES.map((a) => [a.id, 0]));

/**
 * Pinta la parte activa de la pista: desde el centro si el rango tiene
 * negativos (se ve el signo de un vistazo) o desde el mínimo si no.
 */
export function pintarPista(entrada) {
  const min = Number(entrada.min);
  const max = Number(entrada.max);
  const pos = ((Number(entrada.value) - min) / (max - min)) * 100;
  const origen = min < 0 ? ((0 - min) / (max - min)) * 100 : 0;
  const [a, b] = [Math.min(origen, pos), Math.max(origen, pos)];
  entrada.style.setProperty(
    '--pista',
    `linear-gradient(to right, var(--borde) ${a}%, var(--acento) ${a}% ${b}%, var(--borde) ${b}%)`,
  );
}

/**
 * Crea un deslizador por ajuste dentro de `contenedor`, agrupados. `alCambiar(id, valor)`
 * se llama en cada movimiento. Doble clic sobre el valor lo devuelve a 0.
 */
export function crearControles(contenedor, alCambiar, definiciones = AJUSTES) {
  const entradas = new Map();
  let grupoActual = null;

  for (const def of definiciones) {
    if (def.grupo && def.grupo !== grupoActual) {
      grupoActual = def.grupo;
      const titulo = document.createElement('h3');
      titulo.className = 'subgrupo';
      titulo.textContent = def.grupo;
      contenedor.append(titulo);
    }

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
      salida.textContent = valor > 0 && def.min < 0 ? `+${valor}` : String(valor);
      pintarPista(entrada);
    };
    entrada.addEventListener('input', () => {
      const valor = Number(entrada.value);
      fijar(valor);
      alCambiar(def.id, valor);
    });
    const volverACero = () => {
      fijar(0);
      alCambiar(def.id, 0);
    };
    salida.addEventListener('dblclick', volverACero);
    control.querySelector('label').addEventListener('dblclick', volverACero);

    fijar(0);
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

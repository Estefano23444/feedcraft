// Formatos de Instagram (RF-04) y marco de recorte arrastrable (RF-06) que
// funciona igual con ratón y con el dedo (RNF-09): usa Pointer Events.

export const FORMATOS = [
  { id: '1:1', etiqueta: '1x1', ancho: 1080, alto: 1080 },
  { id: '4:5', etiqueta: '4x5', ancho: 1080, alto: 1350 },
  { id: '3:4', etiqueta: '3x4', ancho: 1080, alto: 1440 },
  { id: '9:16', etiqueta: '9x16', ancho: 1080, alto: 1920 },
];

export const formatoPorId = (id) => FORMATOS.find((f) => f.id === id) ?? FORMATOS[1];

const limitar = (v, min, max) => Math.min(Math.max(v, min), max);

/**
 * El mayor marco del formato que cabe en la foto, reducido por `zoom` y
 * centrado en (cx, cy) sin salirse. Coordenadas normalizadas de 0 a 1.
 * Es la misma regla que `recorte::marco_inicial` en Rust.
 */
export function calcularMarco(anchoFoto, altoFoto, formatoId, cx, cy, zoom) {
  const f = formatoPorId(formatoId);
  const proporcion = f.ancho / f.alto;
  let [mw, mh] = anchoFoto / altoFoto > proporcion
    ? [altoFoto * proporcion, altoFoto]
    : [anchoFoto, anchoFoto / proporcion];
  mw /= Math.max(zoom, 1);
  mh /= Math.max(zoom, 1);
  const x = limitar(cx * anchoFoto - mw / 2, 0, anchoFoto - mw);
  const y = limitar(cy * altoFoto - mh / 2, 0, altoFoto - mh);
  return { x: x / anchoFoto, y: y / altoFoto, ancho: mw / anchoFoto, alto: mh / altoFoto };
}

/**
 * Dibuja el marco sobre `caja` y lo deja mover. `alMover(centro)` se llama al
 * arrastrar con el centro nuevo {cx, cy}; `alSoltar()` al terminar.
 * `alZoom(delta)` recibe la rueda del ratón.
 */
export function crearMarco(caja, { alMover, alSoltar, alZoom }) {
  const marco = document.createElement('div');
  marco.className = 'marco';
  marco.tabIndex = 0;
  marco.setAttribute('role', 'group');
  marco.setAttribute('aria-label', 'Marco de recorte: arrástralo o usa las flechas del teclado');
  caja.append(marco);

  let actual = { x: 0, y: 0, ancho: 1, alto: 1 };
  let arrastre = null;

  function dibujar() {
    marco.style.left = `${actual.x * 100}%`;
    marco.style.top = `${actual.y * 100}%`;
    marco.style.width = `${actual.ancho * 100}%`;
    marco.style.height = `${actual.alto * 100}%`;
  }

  function mover(x, y) {
    actual = { ...actual, x: limitar(x, 0, 1 - actual.ancho), y: limitar(y, 0, 1 - actual.alto) };
    dibujar();
    alMover({ cx: actual.x + actual.ancho / 2, cy: actual.y + actual.alto / 2 });
  }

  marco.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return;
    marco.setPointerCapture(e.pointerId);
    const caja_ = caja.getBoundingClientRect();
    arrastre = { id: e.pointerId, px: e.clientX, py: e.clientY, x: actual.x, y: actual.y, w: caja_.width, h: caja_.height };
    marco.classList.add('moviendo');
  });

  marco.addEventListener('pointermove', (e) => {
    if (!arrastre || e.pointerId !== arrastre.id) return;
    mover(arrastre.x + (e.clientX - arrastre.px) / arrastre.w, arrastre.y + (e.clientY - arrastre.py) / arrastre.h);
  });

  const terminar = (e) => {
    if (!arrastre || e.pointerId !== arrastre.id) return;
    arrastre = null;
    marco.classList.remove('moviendo');
    alSoltar();
  };
  marco.addEventListener('pointerup', terminar);
  marco.addEventListener('pointercancel', terminar);

  marco.addEventListener('keydown', (e) => {
    const paso = e.shiftKey ? 0.05 : 0.01;
    const d = { ArrowLeft: [-paso, 0], ArrowRight: [paso, 0], ArrowUp: [0, -paso], ArrowDown: [0, paso] }[e.key];
    if (!d) return;
    e.preventDefault();
    mover(actual.x + d[0], actual.y + d[1]);
    alSoltar();
  });

  caja.addEventListener('wheel', (e) => {
    e.preventDefault();
    alZoom(e.deltaY < 0 ? 0.1 : -0.1);
  }, { passive: false });

  return {
    fijar(m) {
      actual = m;
      dibujar();
    },
    mostrar(visible) {
      marco.hidden = !visible;
    },
  };
}

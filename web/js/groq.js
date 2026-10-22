// Cliente de Groq (sección 5). Solo la interfaz habla con Groq; el núcleo Rust
// nunca sale a la red. La IA recomienda; Rust ejecuta y valida.

export const CONFIG_POR_DEFECTO = {
  // Groq es compatible con el formato de OpenAI. Si la app se publica para
  // otros usuarios, aquí va la URL del intermediario que guarda la clave (RNF-07).
  baseURL: 'https://api.groq.com/openai/v1',
  // Groq retira modelos: por eso es un ajuste editable. qwen3.6-27b (el de la
  // especificación) ya no está disponible; qwen3.8-27b sí, con visión.
  modelo: 'qwen/qwen3.8-27b',
  idioma: 'español',
};

export const IDIOMAS = ['español', 'English', 'português', 'français', 'italiano', 'Deutsch'];

const CLAVE_CONFIG = 'feedcraft.ia.config';
const CLAVE_GROQ = 'feedcraft.ia.clave';
const CLAVE_AVISO = 'feedcraft.ia.aviso-aceptado';
const TIEMPO_MAXIMO_MS = 45_000;

// ---------- Almacenamiento solo en este navegador (RF-20, RNF-06) ----------

function leer(clave) {
  try {
    return localStorage.getItem(clave);
  } catch {
    return null;
  }
}

function escribir(clave, valor) {
  try {
    if (valor === null) localStorage.removeItem(clave);
    else localStorage.setItem(clave, valor);
    return true;
  } catch {
    return false;
  }
}

export function leerConfig() {
  try {
    return { ...CONFIG_POR_DEFECTO, ...JSON.parse(leer(CLAVE_CONFIG) ?? '{}') };
  } catch {
    return { ...CONFIG_POR_DEFECTO };
  }
}

export const guardarConfig = (config) => escribir(CLAVE_CONFIG, JSON.stringify(config));
export const leerClave = () => leer(CLAVE_GROQ) ?? '';
export const guardarClave = (clave) => escribir(CLAVE_GROQ, clave.trim() || null);
export const avisoAceptado = () => leer(CLAVE_AVISO) === '1';
export const aceptarAviso = () => escribir(CLAVE_AVISO, '1');

// ---------- Petición ----------

const SISTEMA = `Eres el asistente de estilo de FeedCraft, un editor de fotos para Instagram.
Miras la foto y recomiendas cómo publicarla. No editas píxeles: solo devuelves parámetros.

Responde SOLO con un objeto JSON, sin texto antes ni después, con esta forma exacta:
{"formato":"4:5","recorte":{"cx":0.5,"cy":0.4},"variantes":[{"estilo":"id-del-catalogo","ajustes_extra":{"brillo":4},"motivo":"..."}],"caption":"...","hashtags":["#..."]}

Reglas:
- formato: uno de "1:1", "4:5", "3:4", "9:16". 4:5 aprovecha más pantalla en el feed y es la mejor opción si dudas; 1:1 para composiciones centradas; 3:4 para fotos verticales que conviene recortar poco; 9:16 solo si la foto es muy vertical.
- recorte: centro de interés (rostro o sujeto principal) en coordenadas relativas a la foto completa; cx y cy van de 0 a 1, con 0,0 arriba a la izquierda.
- variantes: exactamente 3, con estilos DISTINTOS tomados del catálogo por su id, de la más recomendada a la menos.
- ajustes_extra: correcciones pequeñas para esta foto concreta sobre el estilo elegido. Son números enteros entre -20 y 20 en una escala de -100 a 100, donde 0 es sin cambio. Claves posibles: brillo, contraste, saturacion, temperatura, tinte, luces, sombras, vineta, grano. Usa {} si no hace falta.
- motivo: una frase corta, de 8 palabras como máximo, en español.
- caption: un texto breve y natural para la publicación, sin hashtags.
- hashtags: entre 5 y 12, relevantes, cada uno empieza por #.`;

/** Cuerpo de la petición a /chat/completions. */
export function construirPeticion({ modelo, imagenBase64, catalogo, idioma }) {
  const estilos = catalogo.map(({ id, descripcion }) => ({ id, descripcion }));
  return {
    model: modelo,
    messages: [
      { role: 'system', content: SISTEMA },
      {
        role: 'user',
        content: [
          {
            type: 'text',
            text: `Catálogo de estilos (elige solo de aquí, por su id):\n${JSON.stringify(estilos)}\n\nEscribe el caption y los hashtags en ${idioma}.`,
          },
          { type: 'image_url', image_url: { url: `data:image/jpeg;base64,${imagenBase64}` } },
        ],
      },
    ],
    response_format: { type: 'json_object' },
    temperature: 0.4,
    max_completion_tokens: 2048,
  };
}

/**
 * Pide una sugerencia y la valida. Nunca lanza: si algo falla, devuelve la
 * opción segura y un error con un mensaje claro (sección 5, reglas 1 y 4).
 *
 * `normalizar(texto)` y `respaldo(motivo)` los hace el núcleo Rust; `fetch` se
 * puede sustituir en las pruebas.
 * Devuelve { sugerencia, error: null | {tipo, mensaje}, intentos }.
 */
export async function pedirSugerencia({ clave, config, imagenBase64, catalogo, normalizar, respaldo, fetch: traer = globalThis.fetch }) {
  const conRespaldo = async (tipo, mensaje, intentos) => ({
    sugerencia: await respaldo(mensaje),
    error: { tipo, mensaje },
    intentos,
  });
  const url = `${config.baseURL.replace(/\/+$/, '')}/chat/completions`;
  const cuerpo = JSON.stringify(construirPeticion({ modelo: config.modelo, imagenBase64, catalogo, idioma: config.idioma }));

  for (let intento = 1; intento <= 2; intento++) {
    let respuesta;
    const control = new AbortController();
    const temporizador = setTimeout(() => control.abort(), TIEMPO_MAXIMO_MS);
    try {
      respuesta = await traer(url, {
        method: 'POST',
        headers: { Authorization: `Bearer ${clave}`, 'Content-Type': 'application/json' },
        body: cuerpo,
        signal: control.signal,
      });
    } catch (e) {
      const mensaje = e?.name === 'AbortError'
        ? 'Groq tardó demasiado en responder. Puedes seguir editando sin IA.'
        : 'No hay conexión con Groq. Revisa tu internet: puedes seguir editando sin IA.';
      return conRespaldo('red', mensaje, intento);
    } finally {
      clearTimeout(temporizador);
    }

    if (respuesta.status === 401) {
      return conRespaldo('clave', 'La clave de Groq no es válida. Revísala en Configurar IA.', intento);
    }
    if (respuesta.status === 429) {
      const espera = respuesta.headers?.get?.('retry-after');
      return conRespaldo('limite', `Se alcanzó el límite de solicitudes de Groq${espera ? ` (reintenta en ${espera} s)` : ''}. Puedes seguir sin IA.`, intento);
    }
    if (!respuesta.ok) {
      const detalle = await detalleError(respuesta);
      return conRespaldo('servidor', `Groq respondió con un error ${respuesta.status}${detalle ? `: ${detalle}` : ''}. Puedes seguir sin IA.`, intento);
    }

    let texto = '';
    try {
      const datos = await respuesta.json();
      texto = datos?.choices?.[0]?.message?.content ?? '';
    } catch {
      texto = '';
    }
    try {
      return { sugerencia: await normalizar(texto), error: null, intentos: intento };
    } catch {
      // JSON no válido: se reintenta una vez (regla 1).
    }
  }
  return conRespaldo('json', 'La IA no devolvió una respuesta válida: se usa Natural mejorado en 4:5.', 2);
}

async function detalleError(respuesta) {
  try {
    const datos = await respuesta.json();
    const mensaje = String(datos?.error?.message ?? '').slice(0, 160);
    if (/model|decommission/i.test(mensaje)) return `${mensaje} (cambia el modelo en Configurar IA)`;
    return mensaje;
  } catch {
    return '';
  }
}

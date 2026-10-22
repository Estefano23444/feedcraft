# FeedCraft

Editor de imágenes para Instagram que corre entero en el navegador. Subes una foto y en unos segundos tienes **3 versiones listas para publicar**, cada una a la medida exacta que espera Instagram. Solo tienes que elegir cuál descargar.

- **La IA recomienda; Rust ejecuta.** Groq propone estilo, formato, encuadre, caption y hashtags. Un núcleo en Rust compilado a WebAssembly hace toda la edición. La IA nunca toca píxeles.
- **Privado.** Todo el procesamiento ocurre en tu navegador. La foto solo sale del equipo si pulsas «Sugerir con IA», y aun así solo sale una copia de 512 px.
- **Sin pérdida de calidad.** Decodificación a resolución completa, color sobre la foto original, una sola reducción Lanczos3 y un solo JPEG (sRGB, 4:4:4).

## Requisitos

| Herramienta | Para qué |
|---|---|
| [Rust](https://rustup.rs) estable + `rustup target add wasm32-unknown-unknown` | Compilar el núcleo |
| [wasm-pack](https://rustwasm.github.io/wasm-pack/) (`npm install -g wasm-pack`) | Empaquetar el wasm para la web |
| Node.js 20 o superior | Servidor de desarrollo y medición del wasm |

En Windows, Rust necesita las herramientas de compilación de C++ de Visual Studio.

## Uso

```bash
npm run build        # compila nucleo/ a web/pkg/ (wasm optimizado)
npm run dev          # sirve web/ en http://localhost:8080
```

Abre `http://localhost:8080`, arrastra una foto y listo. Para usar la IA, pulsa **Configurar IA** y pega una clave de Groq ([console.groq.com/keys](https://console.groq.com/keys)). La clave se guarda solo en el `localStorage` de tu navegador.

## Pruebas

```bash
npm test             # cargo test (motor) + tamaño del wasm (RNF-03)
npm run sinteticas   # genera imágenes de prueba en pruebas/sinteticas/
```

Con `npm run dev` en marcha, `http://localhost:8080/pruebas.html` ejecuta las pruebas que necesitan el navegador, con el wasm real dentro del Web Worker: rendimiento, IA de punta a punta y funcionamiento sin red.

Si copias fotos reales a `pruebas/corpus/` (carpeta excluida de git), `cargo test` también las pasa por el proceso completo y mide su calidad.

| Prueba | Verifica | Dónde |
|---|---|---|
| T-01 Medidas exactas | RF-04, RF-12 | `nucleo/tests/t01_medidas.rs` |
| T-02 Orientación | RF-02 | `nucleo/tests/t02_orientacion.rs` (las 8 orientaciones EXIF, Display P3, PNG con alfa) |
| T-03 Ajuste neutro · T-04 Simetría | RF-09 | `nucleo/tests/t03_t04_ajustes.rs` |
| T-05 Calidad (SSIM ≥ 0,97) | Sección 6 | `nucleo/tests/t05_calidad.rs`, contra un Lanczos3 independiente, en fotos sintéticas y en el corpus |
| T-06 Foto pequeña | RF-08 | `nucleo/tests/t06_foto_pequena.rs` y `pruebas.html` |
| T-07 IA robusta | Sección 5 | `nucleo/tests/t07_ia.rs` (10 respuestas rotas) y `pruebas.html` (de punta a punta, con errores 401, 429 y de red) |
| T-08 Sin red | RNF-04 | `pruebas.html` |
| T-09 Rendimiento (24 MP < 3 s) | RNF-01 | `pruebas.html` |
| Cada estilo exporta | RF-17 | `nucleo/tests/estilos.rs` |
| RF-14, RF-15 | JPEG sRGB 4:4:4, nombre del archivo | `nucleo/tests/t05_calidad.rs` |
| Corpus completo | Sección 9 | `nucleo/tests/corpus.rs` |

## Catálogo de estilos

Las tendencias viven en [`web/estilos.json`](web/estilos.json), no en la memoria de la IA: la IA solo elige del catálogo. Cada estilo tiene esta forma:

```json
{
  "id": "film-calido",
  "nombre": "Film cálido",
  "descripcion": "Tonos dorados, negros levantados, grano suave. Retratos, atardeceres y comida.",
  "ajustes": { "brillo": 5, "contraste": -10, "saturacion": -8, "temperatura": 18, "tinte": 4, "sombras": 15, "vineta": 12, "grano": 10 },
  "vigente_desde": "2026-10-01"
}
```

- Entre 5 y 20 estilos. Con más opciones, la IA elige peor.
- `natural-mejorado` debe existir siempre: es la opción segura.
- Ajustes posibles, de −100 a 100: `brillo`, `contraste`, `saturacion`, `temperatura`, `tinte`, `luces`, `sombras`, `vineta`. `grano` va de 0 a 100. Los que no se indican valen 0.
- La IA lee la `descripcion` para decidir: describe el aspecto y para qué fotos sirve.

El catálogo se valida al arrancar. Si el archivo no es válido o no carga, la app avisa y usa la copia incrustada en el wasm al compilar.

## Arquitectura

```
web/                  interfaz en HTML + JS, sin framework
  js/app.js           estado de la interfaz: carga, recorte, ajustes, estilos, IA, versiones
  js/worker.js        Web Worker: carga el wasm y guarda la foto (la interfaz no se congela)
  js/groq.js          cliente de Groq: la única pieza que sale a la red
  js/recorte.js       marco de recorte (ratón, dedo y teclado)
  estilos.json        catálogo de estilos
  pruebas.html        pruebas en el navegador
nucleo/src/           núcleo Rust → WebAssembly
  decodificar.rs      JPEG/PNG/WebP, EXIF, perfil ICC → sRGB, transparencia sobre blanco
  color.rs            los 9 ajustes, con tablas por canal y una pasada por píxel
  recorte.rs          formatos, marco y aviso de foto pequeña
  redimension.rs      recorte + Lanczos3 en un solo paso (fast_image_resize)
  enfoque.rs          máscara de enfoque sobre la luminancia
  exportar.rs         JPEG sRGB 4:4:4 y nombre de archivo
  estilos.rs          catálogo y mezcla estilo + ajustes de la IA
  ia.rs               validación de la respuesta de la IA
  proceso.rs          sesión de edición y orden de procesamiento de la sección 6
```

Orden de exportación: decodificar → orientación EXIF → sRGB → ajustes de color a resolución completa → recorte y Lanczos3 a la medida exacta → enfoque → grano → un solo JPEG.

## Decisiones respecto a la especificación

| Tema | Especificación | Implementación | Por qué |
|---|---|---|---|
| Modelo de IA | `qwen/qwen3.6-27b` | `qwen/qwen3.8-27b` (editable) | El 3.6 ya no está disponible en Groq; el 3.8 tiene visión y responde JSON en ~1,5 s |
| Calidad JPEG por defecto | 92 | 95 (control de 80 a 100) | En fotos nocturnas con ruido, la calidad 92 deja el SSIM en 0,968 (< 0,97); con 95 todo el corpus queda ≥ 0,98 |
| Ajustes de color | Sobre la foto completa | Sobre la región del recorte, a resolución completa | Mismo resultado y menos trabajo; la viñeta queda centrada en el encuadre final |
| Grano | Parte de los ajustes de color | Se añade a la medida final, después del enfoque | Lanczos lo borraría y el enfoque lo exageraría |
| Enfoque | Máscara de enfoque | Solo sobre la luminancia, con umbral | Realza el detalle sin realzar el ruido de color |
| Foto pequeña (RF-08) | Avisar | Aviso y elección explícita: tamaño real (por defecto) o ampliar | Nada se amplía en silencio |
| EXIF | `kamadak-exif` | Lector EXIF de `image` | Una dependencia menos y un wasm más pequeño |
| RF-07 y RF-21 (P1) | Después | Incluidos | El flujo de un clic los necesita: recorte centrado en el sujeto y caption visible |

## Privacidad y seguridad

- Sin IA, nada sale del navegador. No hay analítica ni rastreadores (RNF-04).
- Antes del primer uso de la IA, un aviso explica que se envía una copia de 512 px a Groq (RNF-05).
- La clave de Groq nunca va en el código ni en el repositorio (RNF-06). Si publicas la app para otras personas, cambia la **URL base** en *Configurar IA* para apuntar a un intermediario, por ejemplo un webhook de n8n que guarde la clave (RNF-07).
- La respuesta de la IA se valida en Rust antes de aplicarse. Un estilo inventado se cambia por «Natural mejorado», los valores se recortan a su rango y un JSON roto se reintenta una vez antes de usar la opción segura.

## Fuera de alcance de esta versión

Carrusel (RF-03, RF-05), descarga en ZIP (RF-16), búsqueda web de tendencias, video y reels, publicar directo en Instagram, cuentas de usuario, edición por zonas y generar o borrar objetos con IA.

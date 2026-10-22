// Servidor estático de desarrollo para web/. Sin dependencias: solo Node.
// Uso: node scripts/servidor.mjs [puerto]   (por defecto 8080)

import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const RAIZ = resolve(fileURLToPath(new URL('../web', import.meta.url)));
const PUERTO = Number(process.argv[2] ?? process.env.PORT ?? 8080);

const TIPOS = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.webp': 'image/webp',
  '.ico': 'image/x-icon',
};

async function servirArchivo(req, res) {
  const ruta = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  let archivo = normalize(join(RAIZ, ruta));
  if (archivo !== RAIZ && !archivo.startsWith(RAIZ + sep)) {
    res.writeHead(403).end('Prohibido');
    return;
  }
  try {
    if ((await stat(archivo)).isDirectory()) archivo = join(archivo, 'index.html');
    const datos = await readFile(archivo);
    res.writeHead(200, {
      'Content-Type': TIPOS[extname(archivo).toLowerCase()] ?? 'application/octet-stream',
      'Cache-Control': 'no-store',
    });
    res.end(datos);
  } catch {
    res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' }).end('No encontrado');
  }
}

createServer((req, res) => {
  servirArchivo(req, res).catch((e) => {
    res.writeHead(500).end(String(e));
  });
}).listen(PUERTO, () => {
  console.log(`FeedCraft en http://localhost:${PUERTO}`);
});

// RNF-03: el módulo Wasm pesa menos de 2 MB comprimido.
// Uso: node scripts/medir-wasm.mjs   (después de `npm run build`)

import { readFile } from 'node:fs/promises';
import { gzipSync, brotliCompressSync } from 'node:zlib';

const LIMITE = 2 * 1024 * 1024;
const ruta = new URL('../web/pkg/nucleo_bg.wasm', import.meta.url);

let datos;
try {
  datos = await readFile(ruta);
} catch {
  console.error('No existe web/pkg/nucleo_bg.wasm. Ejecuta antes: npm run build');
  process.exit(1);
}

const kb = (n) => `${(n / 1024).toFixed(0)} KB`;
const gzip = gzipSync(datos, { level: 9 }).length;
const brotli = brotliCompressSync(datos).length;

console.log(`Wasm sin comprimir: ${kb(datos.length)}`);
console.log(`Wasm gzip:          ${kb(gzip)}`);
console.log(`Wasm brotli:        ${kb(brotli)}`);

if (gzip >= LIMITE) {
  console.error(`RNF-03 FALLA: ${kb(gzip)} ≥ 2048 KB comprimido`);
  process.exit(1);
}
console.log('RNF-03 OK: menos de 2 MB comprimido');

import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const runtimeSo = join(root, 'apps', 'client', 'src-tauri', 'ndi-runtime', 'libndi.so.6');

if (process.platform !== 'linux') {
  process.exit(0);
}

if (!existsSync(runtimeSo)) {
  console.warn(`Linux NDI runtime missing at ${runtimeSo}; AppImage will ship without bundled NDI`);
  process.exit(0);
}

console.log(`Linux NDI runtime ready at ${runtimeSo}`);

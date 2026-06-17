import { spawn } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { applyNdiEnv } from './with-ndi-env.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const child = spawn('pnpm', ['--filter', '@tandem/client', 'dev'], {
  cwd: root,
  env: applyNdiEnv(),
  stdio: 'inherit',
  shell: true,
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});

import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const ndiSdk = join(root, 'apps', 'client', 'ndi-sdk');
const clientDir = join(root, 'apps', 'client');

export function applyNdiEnv(env = process.env) {
  if (!existsSync(ndiSdk)) {
    console.error(`NDI SDK not found at ${ndiSdk}`);
    process.exit(1);
  }

  const previous = env.NDI_SDK_DIR;
  if (previous && previous !== ndiSdk) {
    console.log(`Using NDI_SDK_DIR=${ndiSdk} (was ${previous})`);
  }

  return { ...env, NDI_SDK_DIR: ndiSdk };
}

const isMain = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isMain) {
  const [, , ...cmd] = process.argv;
  if (cmd.length === 0) {
    console.error('Usage: node scripts/with-ndi-env.mjs <command> [args...]');
    process.exit(1);
  }

  const child = spawn(cmd[0], cmd.slice(1), {
    cwd: clientDir,
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
}

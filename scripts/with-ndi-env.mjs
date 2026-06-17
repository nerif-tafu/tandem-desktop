import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const vendoredNdiSdk = join(root, 'apps', 'client', 'ndi-sdk');
const clientDir = join(root, 'apps', 'client');

const MAC_NDI_SDK_CANDIDATES = [
  process.env.HOME ? join(process.env.HOME, 'ndi-sdk') : null,
  '/Library/NDI SDK for Apple',
  '/Library/NDI 6 SDK',
  '/Library/NDI SDK for macOS',
  '/Library/NDI SDK',
];

function hasNdiHeaders(sdkDir) {
  return ['Include/Processing.NDI.Lib.h', 'include/Processing.NDI.Lib.h'].some((relativePath) =>
    existsSync(join(sdkDir, relativePath)),
  );
}

function hasNdiMacRuntime(sdkDir) {
  return ['lib/macOS/libndi.dylib', 'lib/macOS/libndi.4.dylib', 'lib/libndi.dylib'].some(
    (relativePath) => existsSync(join(sdkDir, relativePath)),
  );
}

export function resolveNdiSdkDir(env = process.env) {
  if (env.NDI_SDK_DIR && existsSync(env.NDI_SDK_DIR)) {
    return env.NDI_SDK_DIR;
  }

  if (process.platform === 'darwin') {
    for (const candidate of MAC_NDI_SDK_CANDIDATES.filter(Boolean)) {
      if (hasNdiHeaders(candidate) || hasNdiMacRuntime(candidate)) {
        return candidate;
      }
    }

    console.error(
      'NDI SDK for Apple not found. Install it from https://ndi.video/tools/ or set NDI_SDK_DIR.',
    );
    process.exit(1);
  }

  if (!existsSync(vendoredNdiSdk)) {
    console.error(`NDI SDK not found at ${vendoredNdiSdk}`);
    process.exit(1);
  }

  return vendoredNdiSdk;
}

export function applyNdiEnv(env = process.env) {
  const resolved = resolveNdiSdkDir(env);
  const previous = env.NDI_SDK_DIR;

  if (previous && previous !== resolved) {
    console.log(`Using NDI_SDK_DIR=${resolved} (was ${previous})`);
  }

  return { ...env, NDI_SDK_DIR: resolved };
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

#!/usr/bin/env node
import { cpSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const sourceScript = join(root, 'apps/client/scripts/linux-livekit-publisher.mjs');
const bundleDir = join(root, 'apps/client/src-tauri/livekit-sidecar');

if (!existsSync(sourceScript)) {
  console.error(`Missing sidecar script at ${sourceScript}`);
  process.exit(1);
}

rmSync(bundleDir, { recursive: true, force: true });
mkdirSync(bundleDir, { recursive: true });

writeFileSync(
  join(bundleDir, 'package.json'),
  `${JSON.stringify(
    {
      name: 'tandem-livekit-sidecar',
      private: true,
      type: 'module',
      dependencies: {
        '@livekit/rtc-node': '^0.13.30',
      },
    },
    null,
    2,
  )}\n`,
);

cpSync(sourceScript, join(bundleDir, 'linux-livekit-publisher.mjs'));

console.log('Installing LiveKit sidecar dependencies...');
execSync('npm install --omit=dev --no-package-lock', {
  cwd: bundleDir,
  stdio: 'inherit',
});

console.log(`LiveKit sidecar bundle ready at ${bundleDir}`);

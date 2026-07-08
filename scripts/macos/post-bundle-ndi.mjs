import { execFileSync } from 'node:child_process';
import { cpSync, existsSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const bundleRoot = join(root, 'apps', 'client', 'src-tauri', 'target', 'release', 'bundle', 'macos');
const dmgDir = join(root, 'apps', 'client', 'src-tauri', 'target', 'release', 'bundle', 'dmg');

if (process.platform !== 'darwin') {
  process.exit(0);
}

function run(command, args, inherit = true) {
  return execFileSync(command, args, {
    stdio: inherit ? 'inherit' : ['inherit', 'pipe', 'inherit'],
    encoding: inherit ? undefined : 'utf8',
  });
}

function findDmgPath() {
  if (!existsSync(dmgDir)) {
    return null;
  }

  const dmgName = readdirSync(dmgDir).find((name) => name.endsWith('.dmg'));
  return dmgName ? join(dmgDir, dmgName) : null;
}

function findBundledAppPath() {
  if (!existsSync(bundleRoot)) {
    return null;
  }

  const appName = readdirSync(bundleRoot).find((name) => name.endsWith('.app'));
  return appName ? join(bundleRoot, appName) : null;
}

function extractAppFromDmg(dmgPath) {
  const attachOutput = String(run('hdiutil', ['attach', dmgPath, '-nobrowse', '-noverify', '-noautoopen'], false));
  const mountPoint = attachOutput
    .split('\n')
    .map((line) => line.split('\t').at(-1)?.trim())
    .find((line) => line?.startsWith('/Volumes/'));

  if (!mountPoint) {
    throw new Error(`Could not mount dmg at ${dmgPath}`);
  }

  try {
    const appName = readdirSync(mountPoint).find((name) => name.endsWith('.app'));
    if (!appName) {
      throw new Error(`No .app bundle found inside ${mountPoint}`);
    }

    const staging = mkdtempSync(join(tmpdir(), 'tandem-macos-'));
    const appPath = join(staging, appName);
    cpSync(join(mountPoint, appName), appPath, { recursive: true });
    return { appPath, appName, staging };
  } finally {
    run('hdiutil', ['detach', mountPoint, '-quiet']);
  }
}

function fixNdiAndSign(appPath) {
  const macOsDir = join(appPath, 'Contents', 'MacOS');
  const frameworksDir = join(appPath, 'Contents', 'Frameworks');
  const dylibPath = join(frameworksDir, 'libndi.dylib');
  const executablePath = join(macOsDir, 'tandem-client');

  if (!existsSync(dylibPath)) {
    console.warn(`Skipping codesign: libndi.dylib not found in ${frameworksDir}`);
    return;
  }

  run('install_name_tool', ['-id', '@rpath/libndi.dylib', dylibPath]);

  if (existsSync(executablePath)) {
    try {
      run('install_name_tool', ['-add_rpath', '@executable_path/../Frameworks', executablePath]);
    } catch {
      // rpath may already exist
    }
  }

  // install_name_tool invalidates signatures; sign inside-out so Gatekeeper accepts the bundle.
  run('codesign', ['--force', '--sign', '-', dylibPath]);
  if (existsSync(executablePath)) {
    run('codesign', ['--force', '--sign', '-', executablePath]);
  }
  run('codesign', ['--force', '--sign', '-', appPath]);
}

function rebuildDmg(appPath, dmgPath) {
  rmSync(dmgPath, { force: true });
  run('hdiutil', [
    'create',
    '-volname',
    'Tandem',
    '-srcfolder',
    appPath,
    '-ov',
    '-format',
    'UDZO',
    dmgPath,
  ]);
}

function resolveAppBundle() {
  const bundledAppPath = findBundledAppPath();
  if (bundledAppPath) {
    return { appPath: bundledAppPath, cleanup: null };
  }

  const dmgPath = findDmgPath();
  if (!dmgPath) {
    console.warn('No macOS .app or .dmg found to post-process');
    process.exit(0);
  }

  // Tauri deletes bundle/macos after creating the dmg, so extract the app from the dmg.
  const extracted = extractAppFromDmg(dmgPath);
  return {
    appPath: extracted.appPath,
    cleanup: () => rmSync(extracted.staging, { recursive: true, force: true }),
  };
}

const dmgPath = findDmgPath();
if (!dmgPath) {
  console.warn(`No macOS dmg found in ${dmgDir}`);
  process.exit(0);
}

const { appPath, cleanup } = resolveAppBundle();

try {
  fixNdiAndSign(appPath);
  rebuildDmg(appPath, dmgPath);
  console.log(`Re-signed ${appPath} and rebuilt ${dmgPath}`);
} finally {
  cleanup?.();
}

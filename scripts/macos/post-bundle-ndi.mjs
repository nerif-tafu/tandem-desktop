import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const bundleRoot = join(root, 'apps', 'client', 'src-tauri', 'target', 'release', 'bundle', 'macos');
const dmgDir = join(root, 'apps', 'client', 'src-tauri', 'target', 'release', 'bundle', 'dmg');

if (process.platform !== 'darwin') {
  process.exit(0);
}

if (!existsSync(bundleRoot)) {
  console.warn(`macOS bundle directory not found: ${bundleRoot}`);
  process.exit(0);
}

function run(command, args) {
  execFileSync(command, args, { stdio: 'inherit' });
}

for (const appName of readdirSync(bundleRoot).filter((name) => name.endsWith('.app'))) {
  const appPath = join(bundleRoot, appName);
  const macOsDir = join(appPath, 'Contents', 'MacOS');
  const frameworksDir = join(appPath, 'Contents', 'Frameworks');
  const dylibPath = join(frameworksDir, 'libndi.dylib');

  if (!existsSync(dylibPath)) {
    console.warn(`Skipping ${appName}: libndi.dylib not found in Frameworks`);
    continue;
  }

  run('install_name_tool', ['-id', '@rpath/libndi.dylib', dylibPath]);

  for (const binaryName of readdirSync(macOsDir)) {
    const binaryPath = join(macOsDir, binaryName);
    try {
      run('install_name_tool', ['-add_rpath', '@executable_path/../Frameworks', binaryPath]);
    } catch {
      // rpath may already exist
    }
  }

  // install_name_tool invalidates the linker signature; re-sign so Gatekeeper
  // does not report the app as "damaged" after download.
  run('codesign', ['--force', '--deep', '--sign', '-', appPath]);

  // tauri build creates the dmg before this script runs; rebuild it from the
  // fixed .app so release artifacts contain the re-signed bundle.
  if (existsSync(dmgDir)) {
    for (const dmgName of readdirSync(dmgDir).filter((name) => name.endsWith('.dmg'))) {
      rmSync(join(dmgDir, dmgName));
    }

    const dmgPath = join(dmgDir, 'Tandem-macos.dmg');
    run('hdiutil', ['create', '-volname', 'Tandem', '-srcfolder', appPath, '-ov', '-format', 'UDZO', dmgPath]);
    console.log(`Rebuilt ${dmgPath} from re-signed ${appName}`);
  }

  console.log(`Fixed NDI dylib linking for ${appName}`);
}

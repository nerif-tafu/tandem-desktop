# Tandem Desktop

Tauri desktop publisher for [Tandem](https://github.com/nerif-tafu/tandem-server). Capture screen, webcam, or NDI sources and stream to remote viewers.

## Development

```bash
pnpm install
pnpm dev
```

The app expects a running Tandem server (default `http://127.0.0.1:3841`). See [tandem-server](https://github.com/nerif-tafu/tandem-server) for local setup.

## Build

```bash
pnpm --filter @tandem/shared build
pnpm --filter @tandem/client build
```

Platform bundles are produced under `apps/client/src-tauri/target/release/bundle/`.

## Releases

Push a version tag to publish installers to GitHub Releases:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The `Release` workflow uploads:

| Platform | Asset |
|----------|-------|
| Windows | `Tandem-windows-x64.exe` |
| macOS | `Tandem-macos.dmg` |
| Linux | `Tandem-linux-x86_64.AppImage` |

## Related repo

- [tandem-server](https://github.com/nerif-tafu/tandem-server) — Web viewer, API, and deployment

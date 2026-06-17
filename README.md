# Tandem Desktop

Desktop publisher for [Tandem](https://github.com/nerif-tafu/tandem-server). Capture your screen, webcam, or NDI feeds and stream them to remote viewers in a room.

## Development

```bash
pnpm install
pnpm dev
```

`pnpm dev` builds `@tandem/shared` and sets `NDI_SDK_DIR` to `apps/client/ndi-sdk`. If you have an old system-wide `NDI_SDK_DIR` from a previous install, delete it or always start via `pnpm dev` so the path stays correct.

You need a running Tandem server. In dev the app talks to `http://127.0.0.1:3841` by default. See [tandem-server](https://github.com/nerif-tafu/tandem-server) for how to run that locally.

## Build

```bash
pnpm --filter @tandem/shared build
pnpm --filter @tandem/client build
```

Installers land in `apps/client/src-tauri/target/release/bundle/`.

## Releases

Tag a version and push to publish installers on GitHub Releases:

```bash
git tag v1.0.0
git push origin v1.0.0
```

The Release workflow uploads:

| Platform | Asset |
|----------|-------|
| Windows | `Tandem-windows-x64-setup.exe` (NSIS installer, includes NDI runtime) |
| macOS | `Tandem-macos.dmg` (includes NDI when built with the NDI SDK for Apple) |
| Linux | `Tandem-linux-x86_64.AppImage` |

## Related repo

- [tandem-server](https://github.com/nerif-tafu/tandem-server): web viewer, API, and deployment

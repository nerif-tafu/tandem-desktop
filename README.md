# Tandem Desktop

Desktop publisher for [Tandem](https://github.com/nerif-tafu/tandem-server). Capture your screen, webcam, or NDI feeds and stream them to remote viewers in a room.

## Development

```bash
pnpm install
pnpm dev
```

`pnpm dev` builds `@tandem/shared` and sets `NDI_SDK_DIR` to `apps/client/ndi-sdk`. If you have an old system-wide `NDI_SDK_DIR` from a previous install, delete it or always start via `pnpm dev` so the path stays correct.

You need a running Tandem server. In dev the app talks to `http://127.0.0.1:3841` by default. See [tandem-server](https://github.com/nerif-tafu/tandem-server) for how to run that locally.

### Linux (Wayland / screen capture)

Use the dev launcher instead of building a release package while iterating — `tauri dev` only recompiles changed Rust crates:

```bash
bash scripts/linux/dev.sh
```

Logs: `~/tandem-dev.log` (tail with `tail -f ~/tandem-dev.log`).

Release builds target **Ubuntu 22.04+ / Debian 12+** as a `.deb`. Tauri links against system WebKit/GTK; `apt` installs runtime dependencies (GStreamer WebRTC, PipeWire, desktop portal) when you install the package:

```bash
sudo apt install ./Tandem-linux-amd64.deb
```

To test portal capture without the full UI:

```bash
bash scripts/linux/test-portal-capture.sh
```

That runs `cargo run --example portal-capture-probe` (seconds, not a release build).

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
| Linux | `Tandem-linux-amd64.deb` (Ubuntu 22.04+ / Debian 12+) |

## Related repo

- [tandem-server](https://github.com/nerif-tafu/tandem-server): web viewer, API, and deployment

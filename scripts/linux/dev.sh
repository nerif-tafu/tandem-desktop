#!/usr/bin/env bash
# Start Tandem desktop in dev mode (tauri dev — incremental Rust, no AppImage build).
exec "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/remote-dev.sh" "$@"

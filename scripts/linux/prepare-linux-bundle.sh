#!/usr/bin/env bash
# Patch Tauri's cached linuxdeploy GTK plugin so AppImages preload host libwayland-client.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PLUGIN_DIR="${HOME}/.cache/tauri"
PLUGIN="${PLUGIN_DIR}/linuxdeploy-plugin-gtk.sh"
MARKER='Tandem: use host libwayland-client'

mkdir -p "$PLUGIN_DIR"
if [[ ! -f "$PLUGIN" ]]; then
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$PLUGIN" \
      https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh
  else
    wget -q -O "$PLUGIN" \
      https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh
  fi
  chmod +x "$PLUGIN"
fi

if grep -q "$MARKER" "$PLUGIN"; then
  echo "linuxdeploy GTK plugin already patched"
  exit 0
fi

python3 - "$PLUGIN" "$MARKER" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
marker = sys.argv[2]
text = path.read_text()
needle = 'export GDK_BACKEND=x11 # Crash with Wayland backend on Wayland - We tested it without it and ended up with this: https://github.com/tauri-apps/tauri/issues/8541'
insert = '''export GDK_BACKEND=x11 # Crash with Wayland backend on Wayland - We tested it without it and ended up with this: https://github.com/tauri-apps/tauri/issues/8541
# Tandem: use host libwayland-client (fixes blank WebKit in AppImage)
if [ -z "${LD_PRELOAD:-}" ]; then
  for lib in /usr/lib/x86_64-linux-gnu/libwayland-client.so.0 /lib/x86_64-linux-gnu/libwayland-client.so.0 /usr/lib64/libwayland-client.so.0; do
    if [ -f "$lib" ]; then export LD_PRELOAD="$lib"; break; fi
  done
fi
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"'''
if needle not in text:
    raise SystemExit(f"Could not find GDK_BACKEND hook in {path}")
path.write_text(text.replace(needle, insert, 1))
print(f"Patched {path}")
PY

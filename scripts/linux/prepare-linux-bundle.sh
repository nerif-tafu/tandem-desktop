#!/usr/bin/env bash
# Patch Tauri's cached linuxdeploy GTK plugin for Tandem AppImages.
set -euo pipefail

PLUGIN_DIR="${HOME}/.cache/tauri"
PLUGIN="${PLUGIN_DIR}/linuxdeploy-plugin-gtk.sh"
WAYLAND_MARKER='Tandem: use host libwayland-client'
EXCLUDE_MARKER='Tandem: exclude glibc-sensitive bundled libs'

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

python3 - "$PLUGIN" "$WAYLAND_MARKER" "$EXCLUDE_MARKER" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
wayland_marker = sys.argv[2]
exclude_marker = sys.argv[3]
text = path.read_text()

if wayland_marker not in text:
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
    text = text.replace(needle, insert, 1)
    print("Patched AppRun hook for host libwayland-client")

if exclude_marker not in text:
    needle = 'env LINUXDEPLOY_PLUGIN_MODE=1 "$LINUXDEPLOY" --appdir="$APPDIR" "${LIBRARIES[@]}"'
    insert = '''# Tandem: exclude glibc-sensitive bundled libs (use host copies on Ubuntu 22.04+)
EXCLUDE_LIBRARIES=(
  "libxslt.so.1"
  "libgcrypt.so.20"
  "libgstreamer-1.0.so.0"
  "libtasn1.so.6"
  "libatk-bridge-2.0.so.0"
  "libgssapi_krb5.so.2"
  "libmount.so.1"
  "libselinux.so.1"
  "libbsd.so.0"
  "libcap.so.2"
  "libdw.so.1"
  "liborc-0.4.so.0"
  "libkrb5.so.3"
  "libk5crypto.so.3"
  "libkrb5support.so.0"
  "libblkid.so.1"
  "libelf.so.1"
  "libudev.so.1"
)
EXCLUDE_ARGS=()
for lib in "${EXCLUDE_LIBRARIES[@]}"; do
  EXCLUDE_ARGS+=( "--exclude-library=$lib" )
done
env LINUXDEPLOY_PLUGIN_MODE=1 "$LINUXDEPLOY" --appdir="$APPDIR" "${EXCLUDE_ARGS[@]}" "${LIBRARIES[@]}"'''
    if needle not in text:
        raise SystemExit(f"Could not find linuxdeploy invocation in {path}")
    text = text.replace(needle, insert, 1)
    print("Patched linuxdeploy excludes for glibc compatibility")

path.write_text(text)
print(f"Ready: {path}")
PY

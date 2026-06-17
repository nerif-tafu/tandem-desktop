#!/usr/bin/env bash
set -euo pipefail
APPIMAGE="${1:-$HOME/tandem-test/Tandem-linux-x86_64.orig.AppImage}"
WORKDIR="/tmp/tandem-extract-test"
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"
cd "$WORKDIR"
"$APPIMAGE" --appimage-extract >/dev/null

cat > squashfs-root/apprun-hooks/tandem-env.sh <<'EOF'
#! /usr/bin/env bash
if [[ -z "${LD_PRELOAD:-}" ]]; then
  for lib in \
    /usr/lib/x86_64-linux-gnu/libwayland-client.so.0 \
    /lib/x86_64-linux-gnu/libwayland-client.so.0 \
    /usr/lib64/libwayland-client.so.0; do
    if [[ -f "$lib" ]]; then
      export LD_PRELOAD="$lib"
      break
    fi
  done
fi
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
EOF
chmod +x squashfs-root/apprun-hooks/tandem-env.sh

if ! grep -q tandem-env.sh squashfs-root/AppRun; then
  sed -i 's|source "$this_dir"/apprun-hooks/"linuxdeploy-plugin-gtk.sh"|source "$this_dir"/apprun-hooks/tandem-env.sh\nsource "$this_dir"/apprun-hooks/"linuxdeploy-plugin-gtk.sh"|' squashfs-root/AppRun
fi

export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
export DISPLAY="${DISPLAY:-:0}"
for auth in "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.*; do
  [[ -f "$auth" ]] && export XAUTHORITY="$auth" && break
done
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"

pkill -x tandem-client 2>/dev/null || true
sleep 1
./squashfs-root/AppRun >/tmp/patched-run.log 2>&1 &
sleep 14

python3 - <<'PY'
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
Atspi.init()
desktop = Atspi.get_desktop(0)
chunks = []

def walk(obj):
    try:
        name = obj.get_name() or ""
        if name.strip():
            chunks.append(name)
        for i in range(obj.get_child_count()):
            walk(obj.get_child_at_index(i))
    except Exception:
        pass

for i in range(desktop.get_child_count()):
    walk(desktop.get_child_at_index(i))

blob = "\n".join(chunks)
if "Create a" in blob or "Join" in blob:
    print("PASS: patched AppRun shows UI")
else:
    print("FAIL: no landing UI")
    raise SystemExit(1)
PY

tail -3 /tmp/patched-run.log
pkill -x tandem-client 2>/dev/null || true

#!/usr/bin/env bash
set -euo pipefail
APPIMAGE="${1:-$HOME/tandem-test/Tandem-v1.1.5-orig.AppImage}"
WORKDIR=/tmp/tandem-host-webkit-test
rm -rf "$WORKDIR"
mkdir -p "$WORKDIR"
cd "$WORKDIR"
"$APPIMAGE" --appimage-extract
cd squashfs-root/usr/lib
rm -f libwebkit* libjavascript* libgtk-3* libgio-2.0* libicui18n* libicuuc* libicudata* \
  libsoup-3* libxml2* libsqlite3* libxslt* libmount* libselinux* libsystemd* libdbus-1* \
  libxkbcommon* libwayland-server* libpango* libXcursor* 2>/dev/null || true

export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
export DISPLAY="${DISPLAY:-:0}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
export GDK_BACKEND=x11
for auth in "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.* "$HOME/.Xauthority"; do
  [[ -f "$auth" ]] && export XAUTHORITY="$auth" && break
done
export LD_PRELOAD="${LD_PRELOAD:-/usr/lib/x86_64-linux-gnu/libwayland-client.so.0}"
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1

pkill -x tandem-client 2>/dev/null || true
sleep 1
"$WORKDIR/squashfs-root/AppRun" >/tmp/host-webkit.log 2>&1 &
sleep 18

if ! pgrep -x tandem-client >/dev/null; then
  echo "FAIL: process exited"
  tail -15 /tmp/host-webkit.log
  exit 1
fi

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
    print("PASS: landing UI visible")
else:
    print("FAIL: no landing UI")
    print(blob[:600])
    raise SystemExit(1)
PY

tail -3 /tmp/host-webkit.log
pkill -x tandem-client 2>/dev/null || true

#!/usr/bin/env bash
set -euo pipefail
APPIMAGE="${1:-$HOME/tandem-test/Tandem-linux-x86_64.AppImage}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
export DISPLAY="${DISPLAY:-:0}"
for auth in "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.*; do
  [[ -f "$auth" ]] && export XAUTHORITY="$auth" && break
done
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
chmod +x "$APPIMAGE"

check_ui() {
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
for needle in ("Create a", "Join", "Room"):
    if needle in blob:
        print("HIT:" + needle)
        raise SystemExit(0)
raise SystemExit(1)
PY
}

run_case() {
  name="$1"
  shift
  pkill -x tandem-client 2>/dev/null || true
  sleep 1
  env "$@" "$APPIMAGE" >"/tmp/case-$name.log" 2>&1 &
  sleep 14
  running=no
  pgrep -x tandem-client >/dev/null && running=yes
  ui=no
  if check_ui; then ui=yes; fi
  egl=$(grep -c EGL /tmp/case-"$name".log || true)
  echo "$name running=$running ui=$ui egl_lines=$egl"
  tail -1 "/tmp/case-$name.log" || true
  pkill -x tandem-client 2>/dev/null || true
  echo
}

run_case default
run_case preload LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libwayland-client.so.0
run_case llvmpipe LIBGL_ALWAYS_SOFTWARE=1 MESA_LOADER_DRIVER_OVERRIDE=llvmpipe GALLIUM_DRIVER=llvmpipe WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1
run_case preload_llvmpipe LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libwayland-client.so.0 LIBGL_ALWAYS_SOFTWARE=1 MESA_LOADER_DRIVER_OVERRIDE=llvmpipe GALLIUM_DRIVER=llvmpipe WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1
run_case xwayland WAYLAND_DISPLAY= WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1

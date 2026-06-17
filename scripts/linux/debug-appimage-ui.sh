#!/usr/bin/env bash
set -euo pipefail
APPIMAGE="${1:-$HOME/tandem-test/Tandem-v1.1.3.AppImage}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
export DISPLAY="${DISPLAY:-:0}"
for auth in "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.*; do
  [[ -f "$auth" ]] && export XAUTHORITY="$auth" && break
done
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"

pkill -x tandem-client 2>/dev/null || true
sleep 1
"$APPIMAGE" >/tmp/v113-debug.log 2>&1 &
sleep 3
PID="$(pgrep -x tandem-client || true)"
echo "PID=$PID"
if [[ -n "$PID" ]]; then
  tr '\0' '\n' <"/proc/$PID/environ" | grep -E 'LD_PRELOAD|WEBKIT' || true
fi
sleep 22

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
print("Create a:", "Create a" in blob)
print("Join:", "Join" in blob)
print(blob[:1000])
if "Create a" not in blob and "Join" not in blob:
    raise SystemExit(1)
PY

tail -5 /tmp/v113-debug.log
pkill -x tandem-client 2>/dev/null || true

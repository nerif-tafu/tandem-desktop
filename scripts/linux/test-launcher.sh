#!/usr/bin/env bash
set -euo pipefail
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
export DISPLAY="${DISPLAY:-:0}"
for auth in "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.*; do
  [[ -f "$auth" ]] && export XAUTHORITY="$auth" && break
done
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"

cp ~/tandem-test/Tandem-linux-x86_64.AppImage ~/Downloads/Tandem-linux-x86_64.AppImage
sed -i 's/\r$//' ~/Downloads/launch-tandem-appimage.sh
chmod +x ~/Downloads/launch-tandem-appimage.sh

pkill -x tandem-client 2>/dev/null || true
sleep 1
~/Downloads/launch-tandem-appimage.sh >/tmp/launcher.log 2>&1 &
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
    print("PASS: launcher shows landing UI")
else:
    print("FAIL: launcher still blank")
    raise SystemExit(1)
PY

pkill -x tandem-client 2>/dev/null || true

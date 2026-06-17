#!/usr/bin/env bash
# Verify a Tandem AppImage actually renders UI (not a blank white window).
# Exit 0 when accessibility tree contains expected landing-page copy.
set -euo pipefail

APPIMAGE="${1:-$HOME/tandem-test/Tandem-linux-x86_64.AppImage}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
export DISPLAY="${DISPLAY:-:0}"
export GDK_BACKEND="${GDK_BACKEND:-x11}"
for auth in "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.*; do
  [[ -f "$auth" ]] && export XAUTHORITY="$auth" && break
done
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"

pkill -x tandem-client 2>/dev/null || true
sleep 1
chmod +x "$APPIMAGE"

echo "Launching $APPIMAGE"
"$APPIMAGE" >"/tmp/tandem-verify.log" 2>&1 &
sleep 14

if ! pgrep -x tandem-client >/dev/null; then
  echo "FAIL: process exited"
  tail -20 /tmp/tandem-verify.log
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
needles = ["Create a", "Join", "Tandem"]
found = [n for n in needles if n in blob]
print("FOUND:", ", ".join(found) if found else "(none)")
if "Create a" not in blob and "Join" not in blob:
    raise SystemExit(1)
PY

echo "PASS: UI text detected"
tail -5 /tmp/tandem-verify.log
pkill -x tandem-client 2>/dev/null || true

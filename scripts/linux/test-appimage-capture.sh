#!/usr/bin/env bash
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

test_one() {
  local name="$1"
  shift
  pkill -x tandem-client 2>/dev/null || true
  sleep 1
  env "$@" "$APPIMAGE" >"/tmp/tandem-$name.log" 2>&1 &
  sleep 12
  local pid
  pid="$(pgrep -x tandem-client || true)"
  printf '%s running=%s ' "$name" "${pid:-no}"
  if [[ -n "$pid" ]]; then
  python3 - <<PY
import subprocess, sys
from PIL import Image
import io
win = subprocess.check_output(["xwininfo", "-name", "Tandem"], text=True)
wid = [l.split()[-1] for l in win.splitlines() if "Window id" in l][0]
raw = subprocess.check_output(["xwd", "-silent", "-id", wid])
proc = subprocess.Popen(["xwdtopnm"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
out, _ = proc.communicate(raw)
proc2 = subprocess.Popen(["pnmtopnm"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
# fallback write via pillow from ppm bytes in pipe
import tempfile, os
with tempfile.NamedTemporaryFile(suffix=".xwd", delete=False) as f:
    f.write(raw)
    xwd_path = f.name
subprocess.run(["convert", xwd_path, f"/tmp/tandem-{name}.png"], check=False)
PY
  fi
  if [[ -f "/tmp/tandem-$name.png" ]]; then
    python3 - "/tmp/tandem-$name.png" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert('RGB')
px = list(img.getdata())
white = sum(1 for r,g,b in px if r > 245 and g > 245 and b > 245)
dark = sum(1 for r,g,b in px if r < 50 and g < 50 and b < 50)
print(f"white={white/len(px):.2f} dark={dark/len(px):.2f}")
PY
  else
    echo "no_png"
  fi
  grep -E 'EGL|Aborting|panic' "/tmp/tandem-$name.log" | tail -1 || true
  pkill -x tandem-client 2>/dev/null || true
  echo
}

chmod +x "$APPIMAGE"
test_one baseline
test_one llvmpipe LIBGL_ALWAYS_SOFTWARE=1 MESA_LOADER_DRIVER_OVERRIDE=llvmpipe GALLIUM_DRIVER=llvmpipe WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1
test_one cairo_soft LIBGL_ALWAYS_SOFTWARE=1 GSK_RENDERER=cairo WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1
test_one wayland GDK_BACKEND=wayland WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1
test_one xwayland_only WAYLAND_DISPLAY= WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1

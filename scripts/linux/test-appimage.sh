#!/usr/bin/env bash
# Run on the Linux test machine from repo: bash scripts/linux/test-appimage.sh [AppImage path]
set -euo pipefail

APPIMAGE="${1:-$HOME/tandem-test/Tandem-linux-x86_64.AppImage}"
OUT_DIR="${2:-$HOME/tandem-test/test-runs}"
mkdir -p "$OUT_DIR"

export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"

if [[ -z "${DISPLAY:-}" ]]; then
  export DISPLAY=:0
  export GDK_BACKEND=x11
  for auth in "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.*; do
    [[ -f "$auth" ]] && export XAUTHORITY="$auth" && break
  done
fi

run_case() {
  local name="$1"
  shift
  local log="$OUT_DIR/${name}.log"
  local shot="$OUT_DIR/${name}.png"

  echo "=== $name ==="
  pkill -x tandem-client 2>/dev/null || true
  sleep 1

  rm -f "$log"
  env "$@" "$APPIMAGE" >"$log" 2>&1 &
  local pid=$!
  sleep 10

  if ! kill -0 "$pid" 2>/dev/null && ! pgrep -x tandem-client >/dev/null; then
    echo "RESULT=crashed"
    tail -5 "$log"
    echo
    return
  fi

  if command -v gnome-screenshot >/dev/null; then
    gnome-screenshot -w -f "$shot" 2>/dev/null || gnome-screenshot -f "$shot" 2>/dev/null || true
  fi

  if [[ -f "$shot" ]]; then
  python3 - "$shot" <<'PY'
import sys
from pathlib import Path
try:
    from PIL import Image
except ImportError:
    print("RESULT=running no_pillow")
    sys.exit(0)
img = Image.open(sys.argv[1]).convert("RGB")
px = list(img.getdata())
white = sum(1 for r, g, b in px if r > 245 and g > 245 and b > 245)
ratio = white / len(px)
dark = sum(1 for r, g, b in px if r < 40 and g < 40 and b < 40)
print(f"RESULT=running white_ratio={ratio:.3f} dark_ratio={dark/len(px):.3f} size={img.size}")
PY
  else
    echo "RESULT=running no_screenshot"
  fi

  tail -3 "$log" | sed 's/^/  /'
  echo

  pkill -x tandem-client 2>/dev/null || true
  sleep 1
}

chmod +x "$APPIMAGE"

run_case direct
run_case webkit_compositing WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1
run_case webkit_wayland_empty WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 WAYLAND_DISPLAY=
run_case webkit_gl WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 GSK_RENDERER=gl LIBGL_ALWAYS_SOFTWARE=0

echo "Logs and screenshots in $OUT_DIR"

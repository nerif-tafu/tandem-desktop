#!/usr/bin/env bash
# Launch Tandem AppImage in the active desktop session (from SSH).
set -euo pipefail

APPIMAGE="${1:-$HOME/tandem-test/Tandem-linux-x86_64.AppImage}"
LOG="${2:-/tmp/tandem-launch.log}"

if [[ ! -f "$APPIMAGE" ]]; then
  echo "AppImage not found: $APPIMAGE" >&2
  exit 1
fi

chmod +x "$APPIMAGE"

export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
export DISPLAY="${DISPLAY:-:0}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
export GDK_BACKEND="${GDK_BACKEND:-x11}"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"

for auth in "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.* "$HOME/.Xauthority"; do
  if [[ -f "$auth" ]]; then
    export XAUTHORITY="$auth"
    break
  fi
done

if [[ -z "${LD_PRELOAD:-}" ]]; then
  for lib in /usr/lib/x86_64-linux-gnu/libwayland-client.so.0 /lib/x86_64-linux-gnu/libwayland-client.so.0; do
    if [[ -f "$lib" ]]; then
      export LD_PRELOAD="$lib"
      break
    fi
  done
fi

pkill -x tandem-client 2>/dev/null || true
sleep 1

nohup "$APPIMAGE" >"$LOG" 2>&1 &
sleep 3

if pgrep -x tandem-client >/dev/null; then
  echo "Tandem running (pid $(pgrep -x tandem-client))"
  echo "Log: $LOG"
  exit 0
fi

echo "Tandem failed to start. Log tail:"
tail -40 "$LOG" >&2
exit 1

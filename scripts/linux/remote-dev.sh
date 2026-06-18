#!/usr/bin/env bash
# Run Tandem from source on the Linux test machine (fast iteration, live logs).
set -euo pipefail

ROOT="${TANDEM_ROOT:-$HOME/tandem-desktop-build}"
LOG="${TANDEM_DEV_LOG:-$HOME/tandem-dev.log}"

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

export NDI_SDK_DIR="${NDI_SDK_DIR:-$HOME/ndi-sdk}"
if [[ -d "$NDI_SDK_DIR/lib/x86_64-linux-gnu" ]]; then
  export LD_LIBRARY_PATH="$NDI_SDK_DIR/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}"
elif [[ -d "$NDI_SDK_DIR/lib" ]]; then
  export LD_LIBRARY_PATH="$NDI_SDK_DIR/lib:${LD_LIBRARY_PATH:-}"
fi

export RUST_LOG="${RUST_LOG:-tandem_client_lib=debug,xcap=debug,grafton_ndi=info}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

if [[ ! -d "$ROOT" ]]; then
  echo "Source tree not found at $ROOT" >&2
  exit 1
fi

pkill -x tandem-client 2>/dev/null || true
pkill -f 'vite.*3842' 2>/dev/null || true
pkill -f '@tauri-apps/cli.*dev' 2>/dev/null || true
sleep 1

cd "$ROOT"
echo "=== tandem dev $(date -Is) ===" | tee "$LOG"
echo "Root: $ROOT" | tee -a "$LOG"
echo "RUST_LOG=$RUST_LOG" | tee -a "$LOG"
echo "Log: $LOG" | tee -a "$LOG"
echo "Tail with: tail -f $LOG" | tee -a "$LOG"

pnpm --filter @tandem/shared build >>"$LOG" 2>&1
exec pnpm --filter @tandem/client dev >>"$LOG" 2>&1

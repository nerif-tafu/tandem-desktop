#!/usr/bin/env bash
# End-to-end portal + PipeWire screen capture probe (Wayland).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
export DISPLAY="${DISPLAY:-:0}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
export GDK_BACKEND="${GDK_BACKEND:-x11}"
export RUST_LOG="${RUST_LOG:-tandem_client_lib=debug}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

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

# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

cd "$ROOT/apps/client/src-tauri"
echo "=== portal-capture-probe $(date -Is) ==="
cargo run --quiet --example portal-capture-probe

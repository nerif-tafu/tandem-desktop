#!/usr/bin/env bash
# Verify capture source enumeration on the Linux test machine.
set -euo pipefail

ROOT="${TANDEM_ROOT:-$HOME/tandem-desktop-build}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
export DISPLAY="${DISPLAY:-:0}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"

for auth in "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.* "$HOME/.Xauthority"; do
  if [[ -f "$auth" ]]; then
    export XAUTHORITY="$auth"
    break
  fi
done

export NDI_SDK_DIR="${NDI_SDK_DIR:-$HOME/ndi-sdk}"

# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

cd "$ROOT/apps/client/src-tauri"
echo "=== list-sources-probe $(date -Is) ==="
cargo run --quiet --example list-sources-probe --features ndi

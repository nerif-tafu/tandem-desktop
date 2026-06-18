#!/usr/bin/env bash
# Quick NDI + capture sanity checks on a Linux desktop test machine.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

echo "=== session ==="
echo "XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-unknown}"
echo "DISPLAY=${DISPLAY:-unset}"
echo "WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-unset}"

echo "=== portal / pipewire ==="
pgrep -a xdg-desktop-portal || echo "xdg-desktop-portal not running"
pgrep -a pipewire || echo "pipewire not running"

echo "=== v4l2 ==="
if command -v v4l2-ctl >/dev/null; then
  v4l2-ctl --list-devices || true
else
  ls -la /dev/video* 2>/dev/null || echo "no /dev/video*"
fi

echo "=== ndi runtime ==="
if ldconfig -p 2>/dev/null | grep -q libndi.so.6; then
  ldconfig -p | grep libndi.so.6
else
  echo "libndi.so.6 not on system library path"
fi

APPIMAGE="${1:-$HOME/tandem-test/Tandem-linux-x86_64.AppImage}"
if [[ -f "$APPIMAGE" ]]; then
  echo "=== appimage ndi strings ==="
  if strings "$APPIMAGE" | grep -q 'NDI support was not compiled'; then
    echo "FAIL: AppImage built without NDI feature"
  else
    echo "PASS: no 'NDI not compiled' marker in AppImage"
  fi
fi

if [[ -n "${NDI_SDK_DIR:-}" ]] && command -v cargo >/dev/null; then
  echo "=== rust ndi probe ==="
  (cd apps/client/src-tauri && cargo run --quiet --features ndi --example ndi-probe)
else
  echo "Skipping ndi-probe (set NDI_SDK_DIR and install Rust to run)"
fi

echo "Done."

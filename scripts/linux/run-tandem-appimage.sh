#!/usr/bin/env bash
# Optional debug launcher for Tandem AppImage on Linux.
set -euo pipefail

APPIMAGE="${1:-${TANDEM_APPIMAGE:-$HOME/tandem-test/Tandem-linux-x86_64.AppImage}}"

if [[ -z "${LD_PRELOAD:-}" ]]; then
  for lib in \
    /usr/lib/x86_64-linux-gnu/libwayland-client.so.0 \
    /lib/x86_64-linux-gnu/libwayland-client.so.0; do
    if [[ -f "$lib" ]]; then
      export LD_PRELOAD="$lib"
      break
    fi
  done
fi

export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"

exec "$APPIMAGE" "${@:2}"

#!/usr/bin/env bash
# Optional debug launcher. The AppImage should work directly after v1.1.2.
set -euo pipefail

APPIMAGE="${1:-${TANDEM_APPIMAGE:-$HOME/tandem-test/Tandem-linux-x86_64.AppImage}}"

export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"

exec "$APPIMAGE" "${@:2}"

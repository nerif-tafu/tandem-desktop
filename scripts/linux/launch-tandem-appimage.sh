#!/usr/bin/env bash
# Temporary launcher until v1.1.3 AppImage is installed. Use from the same folder as the AppImage.
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
export LD_PRELOAD="${LD_PRELOAD:-/usr/lib/x86_64-linux-gnu/libwayland-client.so.0}"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
exec "$DIR/Tandem-linux-x86_64.AppImage" "$@"

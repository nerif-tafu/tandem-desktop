#!/usr/bin/env bash
# Launcher for Tandem AppImage on Linux (WebKit + PipeWire compatibility).
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
export LD_PRELOAD="${LD_PRELOAD:-/usr/lib/x86_64-linux-gnu/libwayland-client.so.0}"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
exec "$DIR/Tandem-linux-x86_64.AppImage" "$@"

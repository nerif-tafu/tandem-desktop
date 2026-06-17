#!/usr/bin/env bash
# Optional debug launcher for AppImage EGL issues. Normal users should run the AppImage directly.
set -euo pipefail

APPIMAGE="${1:-${TANDEM_APPIMAGE:-$HOME/tandem-test/Tandem-linux-x86_64.AppImage}}"

export GSK_RENDERER="${GSK_RENDERER:-cairo}"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"

exec "$APPIMAGE" "${@:2}"

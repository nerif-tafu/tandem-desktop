#!/usr/bin/env bash
# End-to-end: patch AppImage, launch without extra env, verify UI via AT-SPI.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APPIMAGE="${1:-$HOME/tandem-test/Tandem-linux-x86_64.AppImage}"

bash "$ROOT/scripts/linux/patch-appimage.sh" "$APPIMAGE" "$APPIMAGE.patched"
bash "$ROOT/scripts/linux/verify-appimage-ui.sh" "$APPIMAGE.patched"

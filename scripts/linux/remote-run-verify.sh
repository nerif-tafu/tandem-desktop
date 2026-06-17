#!/usr/bin/env bash
set -euo pipefail

RELEASE_TAG="${1:-v1.1.3}"
APPIMAGE_URL="https://github.com/nerif-tafu/tandem-desktop/releases/download/${RELEASE_TAG}/Tandem-linux-x86_64.AppImage"
APPIMAGE=~/tandem-test/Tandem-linux-x86_64.AppImage

mkdir -p ~/tandem-test ~/Downloads
bash ~/tandem-test/install-remote-test-deps.sh

if command -v wget >/dev/null; then
  wget -q -O "$APPIMAGE" "$APPIMAGE_URL"
else
  curl -fsSL -o "$APPIMAGE" "$APPIMAGE_URL"
fi
chmod +x "$APPIMAGE"
cp "$APPIMAGE" ~/Downloads/Tandem-linux-x86_64.AppImage

bash ~/tandem-test/verify-appimage-ui.sh "$APPIMAGE"

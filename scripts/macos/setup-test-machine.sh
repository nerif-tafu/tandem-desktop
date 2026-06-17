#!/usr/bin/env bash
set -euo pipefail

export NDI_SDK_HOME="$HOME/ndi-sdk"
export PATH="$HOME/.cargo/bin:$HOME/.local/node/bin:$PATH"

echo "==> Install Rust (user)"
if ! command -v rustc >/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
# shellcheck disable=SC1091
source "$HOME/.cargo/env"
rustc -V

echo "==> Install Node 22 (user)"
NODE_DIR="$HOME/.local/node"
if ! command -v node >/dev/null; then
  mkdir -p "$HOME/.local"
  ARCH="$(uname -m)"
  case "$ARCH" in
    arm64) NODE_ARCH=arm64 ;;
    x86_64) NODE_ARCH=x64 ;;
    *) echo "unsupported arch $ARCH"; exit 1 ;;
  esac
  NODE_VERSION=22.14.0
  TARBALL="node-v${NODE_VERSION}-darwin-${NODE_ARCH}.tar.gz"
  curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/${TARBALL}" -o "/tmp/${TARBALL}"
  rm -rf "$NODE_DIR"
  tar -xzf "/tmp/${TARBALL}" -C "$HOME/.local"
  mv "$HOME/.local/node-v${NODE_VERSION}-darwin-${NODE_ARCH}" "$NODE_DIR"
fi
export PATH="$NODE_DIR/bin:$PATH"
node -v
corepack enable
pnpm -v 2>/dev/null || npm install -g pnpm
pnpm -v

echo "==> Extract NDI SDK (user-local)"
if [[ ! -f "$NDI_SDK_HOME/include/Processing.NDI.Lib.h" && ! -f "$NDI_SDK_HOME/Include/Processing.NDI.Lib.h" ]]; then
  WORK="$HOME/ndi-sdk-install"
  rm -rf "$WORK"
  mkdir -p "$WORK"
  curl -fsSL "https://downloads.ndi.tv/SDK/NDI_SDK_Mac/Install_NDI_SDK_v6_Apple.pkg" -o "$WORK/ndi.pkg"
  pkgutil --expand "$WORK/ndi.pkg" "$WORK/expanded"
  PAYLOAD="$(find "$WORK/expanded" -name Payload -print -quit)"
  if [[ -z "$PAYLOAD" ]]; then
    echo "Could not find Payload in NDI pkg"
    exit 1
  fi
  mkdir -p "$WORK/root"
  (cd "$WORK/root" && cat "$PAYLOAD" | gunzip -dc | cpio -idm 2>/dev/null)
  SDK_SRC="$(find "$WORK/root" -maxdepth 5 -type d -name 'NDI SDK for Apple' -print -quit)"
  if [[ -z "$SDK_SRC" ]]; then
    SDK_SRC="$(find "$WORK/root" -maxdepth 5 -type d -name '*NDI*SDK*' -print -quit || true)"
  fi
  if [[ -z "$SDK_SRC" || ! -d "$SDK_SRC" ]]; then
    echo "NDI SDK tree not found after extraction"
    find "$WORK/root" -maxdepth 5 -type d | head -40
    exit 1
  fi
  rm -rf "$NDI_SDK_HOME"
  mkdir -p "$NDI_SDK_HOME"
  cp -R "$SDK_SRC/." "$NDI_SDK_HOME/"
fi
ls -la "$NDI_SDK_HOME" | head -10
ls "$NDI_SDK_HOME/lib/macOS" 2>/dev/null || ls "$NDI_SDK_HOME/lib" 2>/dev/null

echo "==> Clone tandem-desktop"
if [[ ! -d "$HOME/tandem-desktop/.git" ]]; then
  git clone https://github.com/nerif-tafu/tandem-desktop.git "$HOME/tandem-desktop"
fi

echo "SETUP_DONE"

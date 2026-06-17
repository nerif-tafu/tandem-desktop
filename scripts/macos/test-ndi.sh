#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "==> Tandem macOS NDI smoke test"
echo "Host: $(hostname)"
echo "OS: $(sw_vers -productName) $(sw_vers -productVersion) ($(uname -m))"

SDK_DIR="${NDI_SDK_DIR:-}"
if [[ -z "$SDK_DIR" ]]; then
  for candidate in \
    "/Library/NDI SDK for Apple" \
    "/Library/NDI 6 SDK" \
    "/Library/NDI SDK for macOS" \
    "/Library/NDI SDK"; do
    if [[ -d "$candidate" ]]; then
      SDK_DIR="$candidate"
      break
    fi
  done
fi

if [[ -z "$SDK_DIR" || ! -d "$SDK_DIR" ]]; then
  echo "ERROR: NDI SDK for Apple not found."
  echo "Install from https://ndi.video/tools/ or set NDI_SDK_DIR."
  exit 1
fi

echo "NDI_SDK_DIR=$SDK_DIR"

for candidate in \
  "$SDK_DIR/lib/macOS/libndi.dylib" \
  "$SDK_DIR/lib/macOS/libndi.4.dylib" \
  "$SDK_DIR/lib/libndi.dylib"; do
  if [[ -f "$candidate" ]]; then
    echo "Found runtime: $candidate"
    RUNTIME="$candidate"
    break
  fi
done

if [[ -z "${RUNTIME:-}" ]]; then
  echo "ERROR: libndi dylib not found under $SDK_DIR"
  exit 1
fi

command -v pnpm >/dev/null || { echo "ERROR: pnpm not installed"; exit 1; }
command -v rustc >/dev/null || { echo "ERROR: rustc not installed"; exit 1; }

if [[ ! -d "$ROOT/node_modules" ]]; then
  echo "==> pnpm install"
  pnpm install
fi

echo "==> Build shared package"
pnpm --filter @tandem/shared build

echo "==> cargo check (NDI enabled)"
export NDI_SDK_DIR="$SDK_DIR"
(
  cd apps/client/src-tauri
  cargo check --features ndi
)

echo "==> NDI availability probe"
export DYLD_LIBRARY_PATH="$(dirname "$RUNTIME"):${DYLD_LIBRARY_PATH:-}"
cargo run --quiet --manifest-path apps/client/src-tauri/Cargo.toml --features ndi --example ndi-probe

echo
echo "PASS: NDI SDK and Rust bindings look healthy."
echo "Next: run 'pnpm dev' from tandem-desktop, open a slot, choose NDI, and pick a source."

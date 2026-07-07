#!/usr/bin/env bash
set -euo pipefail
source "$HOME/.cargo/env"
export NDI_SDK_DIR="${NDI_SDK_DIR:-$HOME/ndi-sdk}"
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=4
cd "$HOME/tandem-desktop-build"
pnpm --filter @tandem/client exec tauri build
node scripts/linux/post-bundle-ndi.mjs
cp -f apps/client/src-tauri/target/release/bundle/deb/*.deb ~/tandem-test/Tandem-linux-amd64.deb
cp -f ~/tandem-test/Tandem-linux-amd64.deb ~/Downloads/
echo BUILD_OK

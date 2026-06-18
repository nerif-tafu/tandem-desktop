#!/usr/bin/env bash
# Build Tandem AppImage on this Linux host (e.g. finn-rm test machine).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

NDI_SDK_DIR="${NDI_SDK_DIR:-$HOME/ndi-sdk}"
export CARGO_PROFILE_RELEASE_LTO="${CARGO_PROFILE_RELEASE_LTO:-false}"
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS="${CARGO_PROFILE_RELEASE_CODEGEN_UNITS:-4}"

ensure_swap() {
  if [[ "$(swapon --show | wc -l)" -le 0 ]] && [[ "$(free -m | awk '/^Mem:/{print $2}')" -lt 6144 ]]; then
    echo "Adding 4G swap for release build..."
    sudo fallocate -l 4G /swapfile-tandem-build 2>/dev/null || sudo dd if=/dev/zero of=/swapfile-tandem-build bs=1M count=4096
    sudo chmod 600 /swapfile-tandem-build
    sudo mkswap /swapfile-tandem-build
    sudo swapon /swapfile-tandem-build
  fi
}

install_system_deps() {
  if ! dpkg -s libwebkit2gtk-4.1-dev >/dev/null 2>&1; then
    echo "Installing system build dependencies..."
    sudo apt-get update
    sudo apt-get install -y curl wget pkg-config build-essential \
      libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libxdo-dev \
      libpipewire-0.3-dev libgbm-dev libxcb1-dev libxrandr-dev libdbus-1-dev \
      libwayland-dev libegl-dev libclang-dev fuse libfuse2 file
  fi
}

install_rust() {
  # shellcheck disable=SC1091
  [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

  if ! command -v rustup >/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --no-modify-path
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi

  if ! rustup show active-toolchain >/dev/null 2>&1; then
    rustup default stable
  fi

  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
  cargo --version
}

install_node() {
  local need_node=0
  if ! command -v node >/dev/null; then
    need_node=1
  else
    local major
    major="$(node -v | sed 's/^v//' | cut -d. -f1)"
    if [[ "${major:-0}" -lt 22 ]]; then
      need_node=1
    fi
  fi

  if [[ "$need_node" -eq 1 ]]; then
    curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
    sudo apt-get install -y nodejs
  fi
  if ! command -v pnpm >/dev/null; then
    sudo npm install -g pnpm
  fi
}

install_ndi_sdk() {
  if [[ -f "$NDI_SDK_DIR/include/Processing.NDI.Lib.h" ]]; then
    echo "NDI SDK already at $NDI_SDK_DIR"
    return
  fi

  echo "Downloading NDI SDK for Linux..."
  local work="$HOME/ndi-sdk-install"
  local archive="$work/ndi_sdk_installer.tar.gz"
  mkdir -p "$work"
  curl -fsSL -o "$archive" "https://downloads.ndi.tv/SDK/NDI_SDK_Linux/Install_NDI_SDK_v6_Linux.tar.gz"

  tar -xzf "$archive" -C "$work"
  local installer
  installer="$(find "$work" -name 'Install_NDI_SDK_*.sh' -type f | head -1)"
  local archive_line
  archive_line="$(awk '/^__NDI_ARCHIVE_BEGIN__/ { print NR+1; exit 0; }' "$installer")"
  mkdir -p "$work/extracted"
  tail -n+"$archive_line" "$installer" | tar -xz -C "$work/extracted"
  local extracted
  extracted="$(find "$work/extracted" -maxdepth 1 -type d -name '*NDI*' | head -1)"
  if [[ -z "$extracted" && -d "$work/extracted/include" ]]; then
    extracted="$work/extracted"
  fi

  mkdir -p "$NDI_SDK_DIR"
  cp -R "$extracted"/* "$NDI_SDK_DIR/"

  for libdir in "$NDI_SDK_DIR/lib" "$NDI_SDK_DIR/lib/x86_64-linux-gnu"; do
    [[ -d "$libdir" ]] || continue
    (
      cd "$libdir"
      for lib in lib*.so.*.*.*; do
        [[ -f "$lib" ]] || continue
        base="${lib%%.so*}.so"
        major="${lib#*.so.}"
        major="${major%%.*}"
        ln -sf "$lib" "${base}.${major}" 2>/dev/null || true
        ln -sf "$lib" "$base" 2>/dev/null || true
      done
    )
  done

  echo "NDI SDK installed to $NDI_SDK_DIR"
}

build_appimage() {
  # shellcheck disable=SC1091
  [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

  export NDI_SDK_DIR
  export PATH="$NDI_SDK_DIR/lib/x86_64-linux-gnu:$NDI_SDK_DIR/lib:${PATH:-}"
  export LD_LIBRARY_PATH="$NDI_SDK_DIR/lib/x86_64-linux-gnu:$NDI_SDK_DIR/lib:${LD_LIBRARY_PATH:-}"

  pnpm install
  pnpm --filter @tandem/shared build
  bash scripts/linux/prepare-linux-bundle.sh
  pnpm --filter @tandem/client exec tauri build
  node scripts/linux/post-bundle-ndi.mjs

  local built
  built="$(ls -1 apps/client/src-tauri/target/release/bundle/appimage/*.AppImage | head -1)"
  mkdir -p "$HOME/tandem-test" "$HOME/Downloads"
  cp -f "$built" "$HOME/tandem-test/Tandem-linux-x86_64.AppImage"
  cp -f "$built" "$HOME/Downloads/Tandem-linux-x86_64.AppImage"
  chmod +x "$HOME/tandem-test/Tandem-linux-x86_64.AppImage" "$HOME/Downloads/Tandem-linux-x86_64.AppImage"
  echo "Built: $built"
  echo "Copied to ~/tandem-test and ~/Downloads"
}

ensure_swap
install_system_deps
install_rust
install_node
install_ndi_sdk
build_appimage

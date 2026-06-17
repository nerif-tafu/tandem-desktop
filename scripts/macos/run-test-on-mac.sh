#!/usr/bin/env bash
export PATH="$HOME/.cargo/bin:$HOME/.local/node/bin:$PATH"
export NDI_SDK_DIR="$HOME/ndi-sdk"
sed -i '' 's/\r$//' "$HOME/tandem-desktop/scripts/macos/test-ndi.sh"
bash "$HOME/tandem-desktop/scripts/macos/test-ndi.sh"

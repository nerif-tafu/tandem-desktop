#!/usr/bin/env bash
# One-time setup on the Linux test machine (desktop session required for UI verify).
set -euo pipefail

if ! command -v python3 >/dev/null; then
  echo "python3 is required" >&2
  exit 1
fi

if ! python3 -c "import gi; gi.require_version('Atspi','2.0')" 2>/dev/null; then
  echo "Installing python3-gi (AT-SPI) for UI verification..."
  if command -v apt-get >/dev/null; then
    sudo apt-get update
    sudo apt-get install -y python3-gi gir1.2-atspi-2.0
  else
    echo "Install python3-gi / gir1.2-atspi-2.0 manually, then re-run." >&2
    exit 1
  fi
fi

mkdir -p ~/tandem-test ~/Downloads
echo "Remote test deps OK on $(hostname)"

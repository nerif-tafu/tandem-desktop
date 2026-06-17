#!/usr/bin/env bash
# Patch a Tandem AppImage so WebKit renders on Linux (host libwayland-client preload).
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <Tandem-linux-x86_64.AppImage> [output.AppImage]" >&2
  exit 1
fi

INPUT="$(readlink -f "$1")"
FINAL="$(readlink -f "${2:-$1}")"
WORKDIR="$(mktemp -d)"
BUILT="$WORKDIR/Tandem-linux-x86_64.AppImage"
trap 'rm -rf "$WORKDIR"' EXIT

cd "$WORKDIR"
"$INPUT" --appimage-extract >/dev/null

HOOK="squashfs-root/apprun-hooks/tandem-env.sh"
mkdir -p squashfs-root/apprun-hooks
cat >"$HOOK" <<'EOF'
#! /usr/bin/env bash
# Prefer the host libwayland-client over the AppImage copy (blank WebKit otherwise).
if [[ -z "${LD_PRELOAD:-}" ]]; then
  for lib in \
    /usr/lib/x86_64-linux-gnu/libwayland-client.so.0 \
    /lib/x86_64-linux-gnu/libwayland-client.so.0 \
    /usr/lib64/libwayland-client.so.0; do
    if [[ -f "$lib" ]]; then
      export LD_PRELOAD="$lib"
      break
    fi
  done
fi
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
EOF
chmod +x "$HOOK"

if ! grep -q 'tandem-env.sh' squashfs-root/AppRun; then
  sed -i 's|source "$this_dir"/apprun-hooks/"linuxdeploy-plugin-gtk.sh"|source "$this_dir"/apprun-hooks/tandem-env.sh\nsource "$this_dir"/apprun-hooks/"linuxdeploy-plugin-gtk.sh"|' squashfs-root/AppRun
fi

OFFSET="$("$INPUT" --appimage-offset)"
RUNTIME="$WORKDIR/runtime"
head -c "$OFFSET" "$INPUT" >"$RUNTIME"

APPIMAGETOOL="${APPIMAGETOOL:-}"
if [[ -z "$APPIMAGETOOL" ]]; then
  for candidate in \
    appimagetool-x86_64.AppImage \
    "$HOME/.cache/tauri/appimagetool-x86_64.AppImage" \
    /tmp/appimagetool-x86_64.AppImage; do
    if [[ -f "$candidate" ]]; then
      APPIMAGETOOL="$candidate"
      break
    fi
  done
fi

if [[ -z "$APPIMAGETOOL" ]]; then
  echo "Downloading appimagetool..." >&2
  APPIMAGETOOL="$WORKDIR/appimagetool-x86_64.AppImage"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$APPIMAGETOOL" \
      https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
  else
    wget -q -O "$APPIMAGETOOL" \
      https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
  fi
  chmod +x "$APPIMAGETOOL"
fi

chmod +x "$APPIMAGETOOL"

run_appimagetool() {
  local args=(--runtime-file "$RUNTIME" --no-appstream squashfs-root "$BUILT")
  if ARCH=x86_64 "$APPIMAGETOOL" "${args[@]}" 2>"$WORKDIR/tool.log"; then
    return 0
  fi
  if grep -qi 'libfuse' "$WORKDIR/tool.log"; then
    echo "FUSE unavailable; running extracted appimagetool..." >&2
    TOOLDIR="$WORKDIR/atool"
    mkdir -p "$TOOLDIR"
    cp "$APPIMAGETOOL" "$TOOLDIR/"
    (
      cd "$TOOLDIR"
      "./$(basename "$APPIMAGETOOL")" --appimage-extract >/dev/null
      ARCH=x86_64 ./squashfs-root/usr/bin/appimagetool \
        --runtime-file "$RUNTIME" --no-appstream "$WORKDIR/squashfs-root" "$BUILT"
    )
    return $?
  fi
  cat "$WORKDIR/tool.log" >&2
  return 1
}

run_appimagetool
mv -f "$BUILT" "$FINAL"
chmod +x "$FINAL"

echo "Patched AppImage: $FINAL"

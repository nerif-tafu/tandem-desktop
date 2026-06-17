#!/usr/bin/env bash
# Remove glibc-sensitive libraries from a built AppImage (use host copies on Ubuntu 22.04).
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <Tandem-linux-x86_64.AppImage>" >&2
  exit 1
fi

INPUT="$(readlink -f "$1")"
FINAL="$INPUT"
WORKDIR="$(mktemp -d)"
BUILT="$WORKDIR/Tandem-linux-x86_64.AppImage"
trap 'rm -rf "$WORKDIR"' EXIT

cd "$WORKDIR"
"$INPUT" --appimage-extract >/dev/null

LIBS=(
  libxslt.so.1
  libgcrypt.so.20
  libgstreamer-1.0.so.0
  libtasn1.so.6
  libatk-bridge-2.0.so.0
  libgssapi_krb5.so.2
  libmount.so.1
  libselinux.so.1
  libbsd.so.0
  libcap.so.2
  libdw.so.1
  liborc-0.4.so.0
  libkrb5.so.3
  libk5crypto.so.3
  libkrb5support.so.0
  libblkid.so.1
  libelf.so.1
  libudev.so.1
)

for lib in "${LIBS[@]}"; do
  find squashfs-root/usr/lib -name "$lib" -delete 2>/dev/null || true
done

OFFSET="$("$INPUT" --appimage-offset)"
RUNTIME="$WORKDIR/runtime"
head -c "$OFFSET" "$INPUT" >"$RUNTIME"

APPIMAGETOOL="${APPIMAGETOOL:-$HOME/.cache/tauri/appimagetool-x86_64.AppImage}"
if [[ ! -f "$APPIMAGETOOL" ]]; then
  APPIMAGETOOL="$WORKDIR/appimagetool-x86_64.AppImage"
  if command -v curl >/dev/null; then
    curl -fsSL -o "$APPIMAGETOOL" \
      https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
  else
    wget -q -O "$APPIMAGETOOL" \
      https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
  fi
  chmod +x "$APPIMAGETOOL"
fi

run_appimagetool() {
  if ARCH=x86_64 "$APPIMAGETOOL" --runtime-file "$RUNTIME" --no-appstream squashfs-root "$BUILT" 2>"$WORKDIR/tool.log"; then
    return 0
  fi
  if grep -qi libfuse "$WORKDIR/tool.log"; then
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
echo "Stripped glibc-sensitive libs from $FINAL"

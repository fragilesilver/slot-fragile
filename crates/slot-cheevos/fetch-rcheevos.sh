#!/bin/sh
set -eu

TAG="${1:-v12.5.0}"
HERE="$(cd "$(dirname "$0")" && pwd)"
DEST="$HERE/rcheevos"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

curl -fsSL "https://github.com/RetroAchievements/rcheevos/archive/refs/tags/$TAG.tar.gz" | tar -xz -C "$WORK"
SRC="$(find "$WORK" -mindepth 1 -maxdepth 1 -type d)"

rm -rf "$DEST"
mkdir -p "$DEST/libretro"
cp -R "$SRC/include" "$DEST/include"
cp -R "$SRC/src" "$DEST/src"
rm -f "$DEST/src/rc_client_raintegration.c" "$DEST/src/rc_client_raintegration_internal.h" "$DEST/src/rc_client_types.natvis"
cp "$SRC/LICENSE" "$DEST/LICENSE"
cp "$SRC/test/libretro.h" "$DEST/libretro/libretro.h"
for p in "$HERE"/patches/*.patch; do
	patch -d "$DEST" -p1 -s < "$p"
done
echo "$TAG" > "$DEST/VERSION"

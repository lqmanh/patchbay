#!/usr/bin/env bash
# Export AppIcon.icns from patchbay.svg and the menu bar PNGs from menubar-*.svg.
# Requires resvg (`cargo install resvg`, the renderer GPUI uses) and iconutil (macOS).
set -euo pipefail

cd "$(dirname "$0")"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
iconset="$work/AppIcon.iconset"
mkdir "$iconset"

for size in 16 32 128 256 512; do
  resvg -w "$size" -h "$size" patchbay.svg "$iconset/icon_${size}x${size}.png"
  resvg -w "$((size * 2))" -h "$((size * 2))" patchbay.svg "$iconset/icon_${size}x${size}@2x.png"
done

iconutil -c icns -o AppIcon.icns "$iconset"
echo "wrote $(pwd)/AppIcon.icns"

# Menu bar template images: @1x at the SVG's own 22 x 16 pt size, plus @2x
for glyph in menubar-*.svg; do
  name="${glyph%.svg}"
  resvg "$glyph" "$name.png"
  resvg -z 2 "$glyph" "$name@2x.png"
  echo "wrote $(pwd)/$name.png, $name@2x.png"
done

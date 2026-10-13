#!/usr/bin/env bash
# Package dist/Patchbay.app as a drag-to-Applications disk image.
set -euo pipefail

cd "$(dirname "$0")/.."

app="dist/Patchbay.app"
version="$(plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")"
dmg="dist/Patchbay-$version.dmg"

staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT

ditto "$app" "$staging/Patchbay.app"
ln -s /Applications "$staging/Applications"

rm -f "$dmg"
diskutil image create from --volumeName Patchbay --format ULMO "$staging" "$dmg"

echo "wrote $(pwd)/$dmg"

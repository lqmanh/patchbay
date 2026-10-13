#!/usr/bin/env bash
# Build dist/Patchbay.app from the release binaries. The GUI looks for the CLI
# beside its own executable, so both live in Contents/MacOS.
set -euo pipefail

cd "$(dirname "$0")/.."

app="dist/Patchbay.app"
contents="$app/Contents"
version="$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)"

cargo build --release -p patchbay-gui -p patchbay-cli

rm -rf "$app"
mkdir -p "$contents/MacOS" "$contents/Resources"
cp target/release/patchbay-gui "$contents/MacOS/patchbay-gui"
cp target/release/patchbay "$contents/MacOS/patchbay"
cp assets/icon/AppIcon.icns "$contents/Resources/AppIcon.icns"

# Keep LSMinimumSystemVersion in sync with .cargo/config.toml.
cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>Patchbay</string>
  <key>CFBundleExecutable</key>
  <string>patchbay-gui</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>
  <string>io.github.lqmanh.patchbay</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Patchbay</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$version</string>
  <key>CFBundleVersion</key>
  <string>$version</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.utilities</string>
  <key>LSMinimumSystemVersion</key>
  <string>26.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

# Ad-hoc, to seal Info.plist and Resources. Nested code first; the bundle
# signature covers the main executable.
codesign --force --sign - "$contents/MacOS/patchbay"
codesign --force --sign - "$app"

echo "wrote $(pwd)/$app ($version)"

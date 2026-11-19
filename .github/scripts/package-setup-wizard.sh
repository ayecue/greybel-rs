#!/usr/bin/env bash
# Packs a build of greyhack-setup (apps/rs/greyhack-setup-wizard) for one Rust target into the output folder:
# greyhack-setup-wizard-<version>-<target>.zip for Windows, .tar.gz otherwise; on macOS the program is in
# `Grey Hack Setup.app`. Usage: package-setup-wizard.sh <version> <rust target> <program> <output folder>
set -euo pipefail

version=$1
target=$2
program=$3
output=$(mkdir -p "$4" && cd "$4" && pwd)
archive="greyhack-setup-wizard-$version-$target"
work=$(mktemp -d)
pack="$work/$archive"
app_name="Grey Hack Setup"

mkdir -p "$pack"
cp LICENSE "$pack/"

case $target in
  *-windows-*)
    cp "$program" "$pack/greyhack-setup.exe"
    (cd "$work" && 7z a -tzip -bso0 "$output/$archive.zip" "$archive")
    ;;
  *-apple-darwin)
    contents="$pack/$app_name.app/Contents"
    mkdir -p "$contents/MacOS"
    cp "$program" "$contents/MacOS/greyhack-setup"
    chmod +x "$contents/MacOS/greyhack-setup"
    cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$app_name</string>
  <key>CFBundleDisplayName</key><string>$app_name</string>
  <key>CFBundleIdentifier</key><string>com.ayecue.greyhack-setup-wizard</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleExecutable</key><string>greyhack-setup</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
    tar -czf "$output/$archive.tar.gz" -C "$work" "$archive"
    ;;
  *)
    cp "$program" "$pack/greyhack-setup"
    chmod +x "$pack/greyhack-setup"
    tar -czf "$output/$archive.tar.gz" -C "$work" "$archive"
    ;;
esac

rm -rf "$work"
echo "$output/$archive"

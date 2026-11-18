#!/usr/bin/env bash
# Publishes the setup wizard's window and command-line tool as trimmed, self-contained single files for each runtime
# and packs them into one archive per runtime in the output folder: .zip for Windows, .tar.gz otherwise, the window as
# an .app bundle on macOS. Usage: package-setup-wizard.sh <version> <output folder>
set -euo pipefail

version=$1
output=$(mkdir -p "$2" && cd "$2" && pwd)
project=apps/dotnet/greyhack-setup-wizard
work=$(mktemp -d)
app_name="Grey Hack Setup Wizard"
host_rid=$(dotnet --info | sed -n 's/^ *RID: *//p' | head -n 1)

for rid in win-x64 win-arm64 linux-x64 linux-arm64 osx-x64 osx-arm64; do
  folder="$work/$rid"

  for part in gui cli; do
    dotnet publish "$project/$part" -c Release -r "$rid" --self-contained \
      -p:PublishSingleFile=true -p:IncludeNativeLibrariesForSelfExtract=true -p:DebugType=none -p:PublishTrimmed=true \
      -o "$folder/$part"
  done

  # Smoke-test the trimmed CLI on the host runtime: startup, command parsing, and text resources.
  if [[ $rid == "$host_rid" ]]; then
    "$folder/cli/greyhack-setup-wizard" --help > /dev/null
    located=$("$folder/cli/greyhack-setup-wizard" locate --game "$work" --language en 2>&1 || true)
    grep -q "wasn't found" <<< "$located"
  fi

  archive="greyhack-setup-wizard-$version-$rid"
  pack="$work/pack/$archive"
  mkdir -p "$pack"
  cp LICENSE "$pack/"

  case $rid in
    win-*)
      cp "$folder/gui/greyhack-setup-wizard-gui.exe" "$folder/cli/greyhack-setup-wizard.exe" "$pack/"
      (cd "$work/pack" && zip -qr "$output/$archive.zip" "$archive")
      ;;
    osx-*)
      contents="$pack/$app_name.app/Contents"
      mkdir -p "$contents/MacOS"
      cp "$folder/gui/"* "$contents/MacOS/"
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
  <key>CFBundleExecutable</key><string>greyhack-setup-wizard-gui</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
      chmod +x "$contents/MacOS/greyhack-setup-wizard-gui"
      cp "$folder/cli/greyhack-setup-wizard" "$pack/"
      chmod +x "$pack/greyhack-setup-wizard"
      tar -czf "$output/$archive.tar.gz" -C "$work/pack" "$archive"
      ;;
    *)
      cp "$folder/gui/greyhack-setup-wizard-gui" "$folder/cli/greyhack-setup-wizard" "$pack/"
      chmod +x "$pack/greyhack-setup-wizard-gui" "$pack/greyhack-setup-wizard"
      tar -czf "$output/$archive.tar.gz" -C "$work/pack" "$archive"
      ;;
  esac
done

ls -l "$output"

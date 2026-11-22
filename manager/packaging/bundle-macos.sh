#!/bin/zsh
# Build a release and package it as "dist/Camoufox Manager.app".
# Data is NOT inside the app: the app uses ~/Library/Application Support/camoufox-manager/
# (camoufox-manager.db, .env, camoufox/...). manager/ is the Cargo workspace; the binary is in apps/camoufox-manager.
set -euo pipefail
cd "${0:A:h}/.."

APP="dist/Camoufox Manager.app"
# workspace-wide version ([workspace.package] in Cargo.toml)
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)

cargo build --release -p camoufox-manager
[ -f packaging/AppIcon.icns ] || python3 packaging/make_icon.py

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/camoufox-manager "$APP/Contents/MacOS/Camoufox Manager"
cp packaging/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Camoufox Manager</string>
  <key>CFBundleDisplayName</key><string>Camoufox Manager</string>
  <key>CFBundleIdentifier</key><string>local.camoufox-manager</string>
  <key>CFBundleExecutable</key><string>Camoufox Manager</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <!-- the webview only talks to the app's own server on 127.0.0.1 -->
  <key>NSAppTransportSecurity</key><dict><key>NSAllowsLocalNetworking</key><true/></dict>
</dict></plist>
PLIST
# ad-hoc signing so Gatekeeper does not report the app as damaged on this machine
codesign --force --deep --sign - "$APP"
echo "Xong: $APP"

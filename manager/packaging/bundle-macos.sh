#!/bin/zsh
# Build a release and package it as "dist/Camoufox Manager.app".
# Data is NOT inside the app: the app uses ~/Library/Application Support/camoufox-manager/
# (camoufox-manager.db, .env, camoufox/...). manager/ is the Cargo workspace; the binary is in apps/camoufox-manager.
set -euo pipefail
cd "${0:A:h}/.."

APP="dist/Camoufox Manager.app"
# uv is bundled so the in-app Camoufox install works with nothing preinstalled
UV_VERSION="0.12.17"
UV_SHA256_AARCH64="85f00cbdc6dd3e97eba4c31b4d014375a9fdfe8f570023b84e5102fc3456896b"
UV_SHA256_X86_64="8dcf05a8c809bb3c471d2b614788ba27a6e41298fc8c31ac84b5f4339fd468e5"
# workspace-wide version ([workspace.package] in Cargo.toml)
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)

cargo build --release -p camoufox-manager
[ -f packaging/AppIcon.icns ] || python3 packaging/make_icon.py

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/camoufox-manager "$APP/Contents/MacOS/Camoufox Manager"
cp packaging/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

# bundle uv: UV_BIN (local builds/tests, no download) or the pinned official release
if [ -n "${UV_BIN:-}" ]; then
  cp "$UV_BIN" "$APP/Contents/MacOS/uv"
else
  case "$(uname -m)" in
    arm64) UV_ARCH=aarch64; UV_SHA256="$UV_SHA256_AARCH64" ;;
    x86_64) UV_ARCH=x86_64; UV_SHA256="$UV_SHA256_X86_64" ;;
    *) echo "unsupported architecture: $(uname -m)" >&2; exit 1 ;;
  esac
  UV_TGZ="packaging/.cache/uv-$UV_VERSION-$UV_ARCH-apple-darwin.tar.gz"
  mkdir -p packaging/.cache
  [ -f "$UV_TGZ" ] || curl -fsSL -o "$UV_TGZ" \
    "https://github.com/astral-sh/uv/releases/download/$UV_VERSION/uv-$UV_ARCH-apple-darwin.tar.gz"
  echo "$UV_SHA256  $UV_TGZ" | shasum -a 256 -c - >/dev/null || { echo "uv sha256 mismatch: $UV_TGZ" >&2; exit 1; }
  UV_TMP=$(mktemp -d)
  tar -xzf "$UV_TGZ" -C "$UV_TMP" --strip-components=1 "uv-$UV_ARCH-apple-darwin/uv"
  cp "$UV_TMP/uv" "$APP/Contents/MacOS/uv"
  rm -rf "$UV_TMP"
fi
chmod +x "$APP/Contents/MacOS/uv"
# third-party licenses (uv is distributed under its MIT option)
mkdir -p "$APP/Contents/Resources/licenses"
cp packaging/licenses/uv-LICENSE-MIT "$APP/Contents/Resources/licenses/"
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
# For notarization, uv and the app must both be signed with the Developer ID identity and the
# hardened runtime (--options runtime), instead of the ad-hoc identity used here.
codesign --force --sign - --options runtime "$APP/Contents/MacOS/uv"
codesign --force --deep --sign - "$APP"
echo "Done: $APP"

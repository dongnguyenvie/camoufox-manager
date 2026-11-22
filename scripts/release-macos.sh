#!/bin/zsh
# Release build for macOS: .app (via manager/packaging/bundle-macos.sh), .dmg, .zip, SHA256SUMS.txt
# in release/<version>/. See scripts/README.md.
# Usage: scripts/release-macos.sh [--arch arm64|x86_64|universal] [--allow-dirty]
set -euo pipefail
ROOT="${0:A:h:h}"
cd "$ROOT"

ARCH=universal
ALLOW_DIRTY=0
while [ $# -gt 0 ]; do
  case "$1" in
    --arch) ARCH="${2:?--arch needs a value}"; shift 2 ;;
    --allow-dirty) ALLOW_DIRTY=1; shift ;;
    -h|--help) sed -n '2,4p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
case "$ARCH" in arm64|x86_64|universal) ;; *) echo "--arch must be arm64, x86_64 or universal" >&2; exit 2 ;; esac

VERSION=$(grep -m1 '^version' manager/Cargo.toml | cut -d'"' -f2)
[ -n "$VERSION" ] || { echo "version not found in manager/Cargo.toml" >&2; exit 1; }

if [ -n "$(git status --porcelain)" ] && [ "$ALLOW_DIRTY" -ne 1 ]; then
  echo "git tree is dirty; commit or stash, or pass --allow-dirty" >&2
  exit 1
fi

OUT="$ROOT/release/$VERSION"
NAME="Camoufox-Manager-$VERSION-macos-$ARCH"
APP="$ROOT/manager/dist/Camoufox Manager.app"
mkdir -p "$OUT"
rm -f "$OUT/$NAME.dmg" "$OUT/$NAME.zip"

# --- build the binary per target ---
build() { # $1 = rust target
  (cd manager && cargo build --release -p camoufox-manager --target "$1")
}
BIN_DIR="$ROOT/release/.build-$VERSION"
rm -rf "$BIN_DIR"; mkdir -p "$BIN_DIR"
A64="$ROOT/manager/target/aarch64-apple-darwin/release/camoufox-manager"
X64="$ROOT/manager/target/x86_64-apple-darwin/release/camoufox-manager"
case "$ARCH" in
  arm64)     build aarch64-apple-darwin; cp "$A64" "$BIN_DIR/camoufox-manager"; UV_ARCH=aarch64 ;;
  x86_64)    build x86_64-apple-darwin;  cp "$X64" "$BIN_DIR/camoufox-manager"; UV_ARCH=x86_64 ;;
  universal) build aarch64-apple-darwin; build x86_64-apple-darwin
             lipo -create "$A64" "$X64" -output "$BIN_DIR/camoufox-manager"; UV_ARCH=universal ;;
esac

# --- bundle (reuses the existing script; it ad-hoc signs) ---
CM_BINARY="$BIN_DIR/camoufox-manager" CM_UV_ARCH="$UV_ARCH" "$ROOT/manager/packaging/bundle-macos.sh"
rm -rf "$BIN_DIR"

# --- signing ---
if [ -n "${CM_SIGN_IDENTITY:-}" ]; then
  echo "Signing with: $CM_SIGN_IDENTITY"
  codesign --force --options runtime --timestamp --sign "$CM_SIGN_IDENTITY" "$APP/Contents/MacOS/uv"
  codesign --deep --force --options runtime --timestamp --sign "$CM_SIGN_IDENTITY" "$APP"
  codesign --verify --deep --strict "$APP"
else
  echo "NOTE: CM_SIGN_IDENTITY not set; the app is only ad-hoc signed (users must right-click > Open)."
fi

# --- notarize the app (zip is used only for submission), then staple before the final archives ---
if [ -n "${CM_NOTARY_PROFILE:-}" ]; then
  if [ -z "${CM_SIGN_IDENTITY:-}" ]; then
    echo "CM_NOTARY_PROFILE requires CM_SIGN_IDENTITY" >&2; exit 1
  fi
  NOTARY_ZIP="$(mktemp -d)/notary.zip"
  ditto -c -k --keepParent "$APP" "$NOTARY_ZIP"
  xcrun notarytool submit "$NOTARY_ZIP" --keychain-profile "$CM_NOTARY_PROFILE" --wait
  xcrun stapler staple "$APP"
else
  echo "NOTE: CM_NOTARY_PROFILE not set; skipping notarization."
fi

# --- zip and dmg ---
ditto -c -k --keepParent "$APP" "$OUT/$NAME.zip"
STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Camoufox Manager $VERSION" -srcfolder "$STAGE" -ov -format UDZO "$OUT/$NAME.dmg" >/dev/null
rm -rf "$STAGE"
if [ -n "${CM_SIGN_IDENTITY:-}" ]; then
  codesign --force --timestamp --sign "$CM_SIGN_IDENTITY" "$OUT/$NAME.dmg"
  if [ -n "${CM_NOTARY_PROFILE:-}" ]; then
    xcrun notarytool submit "$OUT/$NAME.dmg" --keychain-profile "$CM_NOTARY_PROFILE" --wait
    xcrun stapler staple "$OUT/$NAME.dmg"
  fi
fi

# --- checksums (keep other platforms' lines, replace ours) ---
SUMS="$OUT/SHA256SUMS.txt"
touch "$SUMS"
grep -v -F -e "$NAME.dmg" -e "$NAME.zip" "$SUMS" > "$SUMS.tmp" || true
(cd "$OUT" && shasum -a 256 "$NAME.dmg" "$NAME.zip") >> "$SUMS.tmp"
mv "$SUMS.tmp" "$SUMS"

echo
echo "Release artifacts in $OUT:"
ls -lh "$OUT"

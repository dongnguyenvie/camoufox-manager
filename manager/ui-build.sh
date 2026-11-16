#!/bin/sh
# Compile Tailwind v4 -> crates/cm-ui/assets/app.css (needs Node + network, development only;
# the app itself runs offline). Tailwind is installed in a temp dir so no node_modules lands in the repo.
set -e
UI="$(cd "$(dirname "$0")/crates/cm-ui/assets" && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
(cd "$TMP" && npm init -y >/dev/null && npm i --silent tailwindcss@4.1.11 @tailwindcss/cli@4.1.11)
# Same layout as assets/ so the @source globs in input.css resolve (vendor/ is not scanned).
mkdir -p "$TMP/src/components" "$TMP/src/pages"
cp "$UI/input.css" "$UI/index.html" "$UI"/*.js "$TMP/src/"
cp "$UI"/components/*.js "$TMP/src/components/"
cp "$UI"/pages/*.js "$TMP/src/pages/"
(cd "$TMP" && npx tailwindcss -i src/input.css -o out.css --minify)
cp "$TMP/out.css" "$UI/app.css"
echo "wrote $UI/app.css"

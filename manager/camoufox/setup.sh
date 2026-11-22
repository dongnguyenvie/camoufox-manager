#!/bin/sh
# Set up the environment for the Camoufox engine: a dedicated venv (uv) + the camoufox package + the Camoufox browser.
#   manager/camoufox/setup.sh            # install into <data_dir>/camoufox/venv
#   CAMOUFOX_VENV=/path/venv setup.sh    # install elsewhere (then set CAMOUFOX_PYTHON=/path/venv/bin/python)
# The app's Python helper lives in crates/cm-engine-camoufox/python/helper/ (embedded in the binary; the app
# writes it to <data_dir>/camoufox/helper/). This script only installs the venv, the camoufox package and the browser.
# `camoufox fetch` downloads the browser (~200 MB) and the font bundle (~2 GB), about 2.4 GB in total, into the camoufox cache (see `python -m camoufox path`).
set -e
CAMOUFOX_VERSION="0.5.9"
DATA_DIR="$HOME/Library/Application Support/camoufox-manager"
VENV="${CAMOUFOX_VENV:-$DATA_DIR/camoufox/venv}"
UV="$(command -v uv || echo /opt/homebrew/bin/uv)"
[ -x "$UV" ] || { echo "uv is required: https://docs.astral.sh/uv/ (brew install uv)"; exit 1; }

mkdir -p "$(dirname "$VENV")"
[ -x "$VENV/bin/python" ] || "$UV" venv --python 3.12 "$VENV"
"$UV" pip install --python "$VENV/bin/python" "camoufox==$CAMOUFOX_VERSION"
"$VENV/bin/python" -m camoufox fetch
echo "Done. Python: $VENV/bin/python"
echo "Browser: $("$VENV/bin/python" -m camoufox path)"

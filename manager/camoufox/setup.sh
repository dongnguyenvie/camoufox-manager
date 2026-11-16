#!/bin/sh
# Cài môi trường cho engine Camoufox: venv riêng (uv) + gói camoufox + trình duyệt Camoufox.
#   manager/camoufox/setup.sh            # cài vào <data_dir>/camoufox/venv
#   CAMOUFOX_VENV=/path/venv setup.sh    # cài chỗ khác (nhớ đặt CAMOUFOX_PYTHON=/path/venv/bin/python)
# Helper Python của app nằm ở crates/cm-engine-camoufox/python/helper/ (nhúng vào binary, app tự ghi
# ra <data_dir>/camoufox/helper/) — script này chỉ cài venv + gói camoufox + trình duyệt.
# `camoufox fetch` tải trình duyệt (~200 MB) + bộ font (~2 GB), tổng ~2,4 GB, vào cache của camoufox (xem `python -m camoufox path`).
set -e
CAMOUFOX_VERSION="0.5.9"
DATA_DIR="$HOME/Library/Application Support/camoufox-manager"
VENV="${CAMOUFOX_VENV:-$DATA_DIR/camoufox/venv}"
UV="$(command -v uv || echo /opt/homebrew/bin/uv)"
[ -x "$UV" ] || { echo "Cần uv: https://docs.astral.sh/uv/ (brew install uv)"; exit 1; }

mkdir -p "$(dirname "$VENV")"
[ -x "$VENV/bin/python" ] || "$UV" venv --python 3.12 "$VENV"
"$UV" pip install --python "$VENV/bin/python" "camoufox==$CAMOUFOX_VERSION"
"$VENV/bin/python" -m camoufox fetch
echo "Xong. Python: $VENV/bin/python"
echo "Trình duyệt: $("$VENV/bin/python" -m camoufox path)"

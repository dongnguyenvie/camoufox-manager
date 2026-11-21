#!/bin/sh
# End-to-end harness (plan CM-1.13). Not part of `cargo test` / gates.sh.
#   ./e2e/run.sh            all scenarios      ./e2e/run.sh 1 3 5   selected ones
#   ./e2e/run.sh flaky [N]  run `cargo test --workspace` N times (default 10), report flaky tests
# Every run uses private temp HOMEs (cme2e.*) with the Camoufox browser cache symlinked from the real
# one, headless, ports 7921/47921/51000 (+ 7922/47922/52000 for the sync peer). Apps are stopped by
# PID only. Needs ./target/debug/camoufox-manager (built here when missing) and the real Camoufox venv.
set -eu
cd "$(dirname "$0")/.."
[ -x target/debug/camoufox-manager ] || cargo build --workspace
if [ "${1:-}" = "flaky" ]; then
  exec python3 e2e/flaky.py "${2:-10}"
fi
PY=""
for n in camoufox-manager; do
  c="$HOME/Library/Application Support/$n/camoufox/venv/bin/python"
  if [ -x "$c" ]; then PY="$c"; break; fi
done
[ -n "$PY" ] || { echo "no Camoufox venv found (run manager/camoufox/setup.sh)"; exit 2; }
exec "$PY" e2e/main.py "$@"

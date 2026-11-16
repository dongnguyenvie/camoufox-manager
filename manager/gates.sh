#!/bin/sh
# Gate for every phase (plan CM-1): format, lint, build, test. Run from anywhere.
set -eu
cd "$(dirname "$0")"
step() { printf '\n== %s\n' "$*"; }
step "cargo fmt --check";   cargo fmt --all --check
step "cargo clippy";        cargo clippy --workspace --all-targets -- -D warnings
step "cargo build";         cargo build --workspace
step "cargo test";          cargo test --workspace
if command -v node >/dev/null 2>&1; then
  step "node --check (UI scripts)"
  for f in crates/cm-ui/assets/*.js crates/cm-ui/assets/components/*.js crates/cm-ui/assets/pages/*.js; do node --check "$f"; done
fi
printf '\ngates: OK\n'

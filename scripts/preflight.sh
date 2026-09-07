#!/usr/bin/env bash
set -eu
missing=0
for t in git python3; do command -v "$t" >/dev/null 2>&1 || { echo "preflight missing REQUIRED_NOW tool: $t" >&2; missing=1; }; done
# Rust/Node/pnpm become REQUIRED_NOW once repository bootstrap pins them in EP-000.
if [ -f rust-toolchain.toml ]; then command -v cargo >/dev/null || missing=1; fi
if [ -f package.json ]; then command -v node >/dev/null || missing=1; command -v pnpm >/dev/null || missing=1; fi
[ "$missing" -eq 0 ] || exit 1
echo "preflight: ok"

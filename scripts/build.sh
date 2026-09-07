#!/usr/bin/env bash
set -eu
[ -f package.json ] && [ -f Cargo.toml ] || { echo "build: product not bootstrapped" >&2; exit 2; }
pnpm --filter @linchpin/desktop tauri build

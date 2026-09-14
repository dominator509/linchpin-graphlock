#!/usr/bin/env bash
set -eu
if [ -f Cargo.toml ]; then
  cargo clippy --workspace --all-targets --locked -- -D warnings
fi
if [ -f package.json ]; then
  pnpm -r lint
fi

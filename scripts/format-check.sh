#!/usr/bin/env bash
set -eu
if [ -f Cargo.toml ]; then
  cargo fmt --all -- --check
fi
if [ -f package.json ]; then
  pnpm -r format:check
fi

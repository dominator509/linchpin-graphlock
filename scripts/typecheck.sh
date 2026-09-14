#!/usr/bin/env bash
set -eu
if [ -f Cargo.toml ]; then
  cargo check --workspace --all-targets --locked
fi
if [ -f package.json ]; then
  pnpm -r typecheck
fi

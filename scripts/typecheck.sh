#!/usr/bin/env bash
set -eu
[ -f Cargo.toml ] && cargo check --workspace --all-targets --locked
[ -f package.json ] && pnpm -r typecheck

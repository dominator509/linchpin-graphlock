#!/usr/bin/env bash
set -eu
[ -f Cargo.toml ] && cargo clippy --workspace --all-targets --locked -- -D warnings
[ -f package.json ] && pnpm -r lint

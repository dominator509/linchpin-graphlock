#!/usr/bin/env bash
set -eu
[ -f Cargo.toml ] && cargo fmt --all -- --check
[ -f package.json ] && pnpm -r format:check

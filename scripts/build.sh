#!/usr/bin/env bash
set -eu
[ -f package.json ] && [ -f Cargo.toml ] || { echo "build: product not bootstrapped" >&2; exit 2; }
# `-- --locked` forwards to cargo. REQ-LIC-003 requires dependency versions to be
# pinned exactly and never floated, and this is the PRIMARY production build, yet
# it was the one path not passing --locked: without it `tauri build` can silently
# re-resolve and rewrite Cargo.lock. Found by the licensing-policy acceptance
# test, which audits every build invocation under scripts/ rather than one.
pnpm --filter @linchpin/desktop tauri build -- --locked

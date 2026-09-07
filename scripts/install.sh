#!/usr/bin/env bash
set -eu
[ -f Cargo.toml ] || [ -f package.json ] || { echo "install: product repository has not been bootstrapped; execute EP-000/EP-001" >&2; exit 2; }
[ -f Cargo.toml ] && cargo fetch --locked
[ -f package.json ] && pnpm install --frozen-lockfile
[ -f pyproject.toml ] && uv sync --frozen

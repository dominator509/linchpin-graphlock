#!/usr/bin/env bash
set -eu
[ -f Cargo.toml ] || [ -f package.json ] || { echo "install: product repository has not been bootstrapped; execute EP-000/EP-001" >&2; exit 2; }
if [ -f Cargo.toml ]; then
  cargo fetch --locked
fi
if [ -f package.json ]; then
  pnpm install --frozen-lockfile
fi
if [ -f pyproject.toml ]; then
  uv sync --frozen
fi

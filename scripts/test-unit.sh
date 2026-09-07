#!/usr/bin/env bash
set -eu
[ -f Cargo.toml ] && cargo test --workspace --locked
[ -f package.json ] && pnpm -r test:unit
[ -f pyproject.toml ] && uv run pytest -q tests/unit

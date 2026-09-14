#!/usr/bin/env bash
set -eu
# Rust: run the suite, then verify collection against the committed manifest.
# DOD-007 requires the harness to fail when zero or too few tests are collected,
# so the guard runs even though `cargo test` already exits non-zero on failure.
if [ -f Cargo.toml ]; then
  cargo test --workspace --locked
  python3 scripts/test-collection-guard.py
fi
# JS: vitest is invoked with --passWithNoTests=false in every package so an
# empty suite exits non-zero instead of reporting a green run with zero tests.
[ -f package.json ] && pnpm -r test:unit
[ -f pyproject.toml ] && uv run pytest -q tests/unit

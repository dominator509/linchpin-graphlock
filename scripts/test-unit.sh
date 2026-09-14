#!/usr/bin/env bash
# Unit-test gate: Rust, JavaScript and (if present) Python lanes.
#
# DOD-007: the harness must fail when zero or too few tests are collected, so
# the Rust lane runs the collection guard in addition to `cargo test`.
#
# BUG FIXED: the final line was
#     [ -f pyproject.toml ] && uv run pytest -q tests/unit
# Under `set -eu` a false test in an `&&` list is the command's exit status, so
# the script exited 1 whenever pyproject.toml was ABSENT -- i.e. it reported
# failure even though every lane that ran had passed. That is a gate that lies,
# and it would have masked a genuine pass. Each lane now uses an explicit `if`,
# and the script reports which lanes ran.
set -eu

ran=""

if [ -f Cargo.toml ]; then
  cargo test --workspace --locked
  python3 scripts/test-collection-guard.py
  ran="$ran rust"
fi

if [ -f package.json ]; then
  # Every package invokes vitest with --passWithNoTests=false, so an empty
  # suite exits non-zero instead of reporting a green run with zero tests.
  pnpm -r test:unit
  ran="$ran js"
fi

if [ -f pyproject.toml ]; then
  uv run pytest -q tests/unit
  ran="$ran python"
fi

if [ -z "$ran" ]; then
  echo "test-unit: FAIL -- no test lane present (no Cargo.toml, package.json or pyproject.toml)" >&2
  exit 2
fi

echo "test-unit: ok (lanes:$ran)"

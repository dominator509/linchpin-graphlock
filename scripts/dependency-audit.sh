#!/usr/bin/env bash
set -eu
if [ -f Cargo.toml ]; then
  cargo deny check
fi
if [ -f package.json ]; then
  pnpm audit --audit-level high
fi
if [ -f pyproject.toml ]; then
  uv run pip-audit
fi

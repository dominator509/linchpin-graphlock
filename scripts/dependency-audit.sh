#!/usr/bin/env bash
set -eu
[ -f Cargo.toml ] && cargo deny check
[ -f package.json ] && pnpm audit --audit-level high
[ -f pyproject.toml ] && uv run pip-audit

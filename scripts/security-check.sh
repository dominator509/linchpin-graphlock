#!/usr/bin/env bash
set -eu
python3 scripts/anti-gaming-scan.py .
[ -f Cargo.toml ] && cargo deny check
[ -f package.json ] && pnpm audit --audit-level high

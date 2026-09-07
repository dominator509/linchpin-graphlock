#!/usr/bin/env bash
set -eu
[ -f package.json ] || { echo "e2e: desktop product not bootstrapped" >&2; exit 2; }
pnpm -r test:e2e

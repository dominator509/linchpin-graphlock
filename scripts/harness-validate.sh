#!/usr/bin/env bash
set -eu
python3 scripts/validate-generated-pack.py .
python3 scripts/validate-hash-ledger.py . 2>/dev/null || true

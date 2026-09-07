#!/usr/bin/env bash
set -eu
python3 scripts/validate-generated-pack.py .
sh scripts/lint.sh
sh scripts/format-check.sh
sh scripts/typecheck.sh
sh scripts/test-unit.sh
sh scripts/security-check.sh

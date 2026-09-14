#!/usr/bin/env bash
set -eu
python3 scripts/validate-generated-pack.py .
# Materialize the per-ID accounting so the count invariant in
# harness-accounting.sh is satisfiable without synthesizing PASS.
python3 scripts/build-accounting.py
echo "harness-init: blueprint valid; per-ID accounting materialized (statuses are honest, not synthesized)"

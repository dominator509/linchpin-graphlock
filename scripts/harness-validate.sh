#!/usr/bin/env bash
set -eu
# Full harness validation.
#
# Previously this script ended with `python3 scripts/validate-hash-ledger.py .
# 2>/dev/null || true`, which discarded BOTH the exit code and stderr. A
# tampered or corrupt .agent/state/LEDGER.jsonl therefore validated "cleanly"
# (demonstrated: a file containing `garbage {"not":"valid"}` makes the validator
# exit 1, but `|| true` forces the overall result to 0). That is a DOD-024
# failure-masking pattern and, for a hash-chained evidence ledger, a
# tamper-detection bypass. The mask is removed here.
python3 scripts/validate-generated-pack.py .
python3 scripts/validate-hash-ledger.py .agent/state/LEDGER.jsonl
python3 scripts/build-accounting.py --check
python3 scripts/build-traceability.py --check
python3 scripts/build-dod-status.py --check
python3 scripts/test-collection-guard.py --check
python3 scripts/ship-gate.py --check
echo "harness-validate: ok"

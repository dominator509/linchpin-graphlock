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
python3 scripts/build-accounting.py --check --structure-only
python3 scripts/build-traceability.py --check
python3 scripts/build-dod-status.py --check --structure-only
python3 scripts/generate-evidence-index.py --check --structure-only
python3 scripts/test-collection-guard.py --check
# Structure only: this script is stage V-000 and runs BEFORE the settle path refreshes
# the derived state, so an equality check against the recorded gate is unsatisfiable
# here. MEASURED: it failed with "ship-gate check: FAIL (recorded INCONCLUSIVE,
# recomputed CONDITIONAL_EXTERNAL_GATES)" for a state that was merely not refreshed yet
# -- the same circularity the other derived artifacts above already solve with
# --structure-only. The strict identity check now runs in scripts/verify.sh, after the
# refresh, so the comparison happens where it can hold instead of being dropped.
python3 scripts/ship-gate.py --check --structure-only
python3 scripts/build-applicability.py --check
python3 scripts/generate-sbom.py --check
python3 scripts/change-invalidation.py --check
python3 scripts/bind-requirements.py --check
python3 scripts/secret-scan.py --check
python3 scripts/collect-skip-report.py --check
python3 scripts/reachability.py --check
python3 scripts/fix-trailing-and-lanes.py --check
echo "harness-validate: ok"

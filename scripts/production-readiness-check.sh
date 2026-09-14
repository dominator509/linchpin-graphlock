#!/usr/bin/env bash
# Production readiness gate.
#
# Previously this read RELEASE_GATE.json and failed unless the verdict was
# already GO/CONDITIONAL_EXTERNAL_GATES. Nothing ever computed that file, so the
# gate could only ever fail -- it could not distinguish "not ready" from "not
# evaluated".
#
# It now recomputes the verdict from measured state and compares, so a stale or
# hand-edited gate file is detected rather than trusted (DOD-042 requires the
# verdict be machine-validated).
set -eu

[ -f .agent/verification/reports/RELEASE_GATE.json ] || {
  echo "production-readiness: FAIL -- no RELEASE_GATE.json" >&2
  exit 2
}

echo "production-readiness: recomputing verdict from measured state"
python3 scripts/ship-gate.py --check || {
  echo "production-readiness: FAIL -- recorded verdict does not match a fresh computation" >&2
  exit 1
}

verdict="$(python3 -c "import json;print(json.load(open('.agent/verification/reports/RELEASE_GATE.json'))['verdict'])")"
echo "production-readiness: recorded verdict = $verdict"

case "$verdict" in
  GO|CONDITIONAL_EXTERNAL_GATES)
    echo "production-readiness: ok"
    ;;
  NO_GO|INCONCLUSIVE)
    # Correct and expected while mandatory clauses are unmet. Report it as a
    # readiness failure rather than an error, so the exit code still signals
    # "not shippable" to any caller that ignores the message.
    echo "production-readiness: NOT READY (verdict $verdict)" >&2
    exit 1
    ;;
  *)
    echo "production-readiness: FAIL -- unknown verdict $verdict" >&2
    exit 1
    ;;
esac

#!/usr/bin/env bash
# Full verification ladder.
#
# Ordered so that the epoch-sensitive steps come LAST: running any build or test
# gate can change the epoch, so the change-invalidation and rerun checks are only
# meaningful once everything before them has settled.
#
# BUG FIXED: the previous version ended with `sh scripts/security-check.sh`,
# whose own `set -eu` trailing-`&&` bug (since fixed) made this script's exit
# status depend on whether package.json existed rather than on the gates. Each
# lane now reports its own status and the script fails if any lane fails.
set -eu

status=0
run() {
  name="$1"; shift
  echo "=== verify: $name"
  if "$@"; then
    echo "    $name: ok"
  else
    code=$?
    echo "    $name: FAIL (exit $code)" >&2
    status=1
  fi
}

run "pack validation" python3 scripts/validate-generated-pack.py .
run "lint" sh scripts/lint.sh
run "format-check" sh scripts/format-check.sh
run "typecheck" sh scripts/typecheck.sh
run "unit tests" sh scripts/test-unit.sh
run "integration tests" sh scripts/test-integration.sh
run "security" sh scripts/security-check.sh

# Epoch-sensitive checks, LAST and in this order.
#
# `change-invalidation.py` (without --check) must run first: it RECOMPUTES and
# records the epoch. Only then can --check pass and a rerun be meaningful.
# Without this, every lane above that touches a tracked input advances the epoch
# and leaves the recorded record stale, which is what happened when the checks
# were simply appended.
run "change invalidation (recompute)" python3 scripts/change-invalidation.py
run "change invalidation (verify)" python3 scripts/change-invalidation.py --check
run "rerun obligation" python3 scripts/rerun-invalidated.py --check

if [ "$status" -eq 0 ]; then
  echo "verify: ok"
else
  echo "verify: FAIL -- see the lanes above" >&2
fi
exit "$status"

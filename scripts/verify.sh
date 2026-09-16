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

# Evidence-currency checks, AFTER the epoch has settled.
#
# These live here rather than in harness-validate.sh because harness-validate is
# itself an invalidated stage: a gate change reruns it, and the stages that
# rewrite evidence run before it, so a currency check inside it can never pass
# on a rerun. That is the same circularity that forced the rerun-obligation
# check out of harness-validate earlier; harness-validate now runs these with
# --structure-only and the currency assertion happens here, on the settled state.
run "ledger evidence currency" python3 scripts/build-accounting.py --check
run "DOD evidence currency" python3 scripts/build-dod-status.py --check
run "evidence index currency" python3 scripts/generate-evidence-index.py --check

# Identity check LAST, after the epoch has settled: DOD-029 requires the
# candidate commit, base revision, epoch and artifact digests to belong to the
# SAME run, and this lane fails when any of them has moved since the manifest was
# written. Re-derive with `python3 scripts/run-manifest.py` after settling.
run "run manifest identity" python3 scripts/run-manifest.py --check
run "completion report" python3 scripts/completion-report.py --check

# DOD-008 evidence currency: the coverage report records the EPOCH it was measured
# at, so a source change invalidates the number instead of leaving a stale one in
# place. Re-measure with `python3 scripts/coverage-gate.py` (the settle path does
# this automatically).
run "coverage currency" python3 scripts/coverage-gate.py --check

if [ "$status" -eq 0 ]; then
  echo "verify: ok"
else
  echo "verify: FAIL -- see the lanes above" >&2
fi
exit "$status"

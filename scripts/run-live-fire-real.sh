#!/usr/bin/env bash
# The real production-path live-fire runner.
#
# `scripts/live-fire.sh` refused with "real production-path runner absent until
# EP-007" (exit 2), which is one of the FAILING commands published in
# COMMANDS.md, and `scripts/reality-gate.sh` skips its final proof while this
# file is missing. AGENTS.md is explicit that this gate "must genuinely pass":
#
#   Reality law (section 9): proof crosses "the actual desktop/UI or declared
#   service boundary, reaches real persistence or a valid sandbox/external
#   dependency, independently reads the effect back, exercises a negative case,
#   and binds the result to the exact artifact digest."
#
# This runner composes the three gates that together satisfy that definition.
# It does NOT invent a fourth proof, and it does not re-implement any of them:
# each already exists, is independently runnable, and carries its own evidence.
#
#   1. scripts/artifact-e2e.sh          -- crosses the packaged desktop/UI
#      boundary over CDP, invokes real Tauri commands, WRITES to the durable
#      vault, reads the effect back, and binds the run to a pinned executable
#      digest. (Reality-law items: real boundary, real persistence, digest.)
#   2. scripts/live-fire-local-provider.sh -- reaches a real external-ish
#      dependency (a local inference server) at the product's own command
#      boundary, exercises BOTH negative cases (non-loopback refused, unreachable
#      port fails closed without fabricating text), and proves the production
#      artifact keeps its debug port shut. (Negative cases, real dependency.)
#   3. scripts/vault-preservation-e2e.sh -- real installer, real persistent
#      state, and reconciliation across update / uninstall / rollback.
#
# PREREQUISITE POLICY: step 2 requires a served loopback provider (PF-011). It
# exits 2 naming that prerequisite rather than skipping, so on a host without one
# this runner fails loudly. That is deliberate: a "live fire" that quietly skips
# its live dependency would be exactly the fabrication AG-006 recorded.
set -eu

echo "=== live-fire: production-path proof suite ==="
echo "live-fire: host = $(powershell -NoProfile -Command '[Environment]::OSVersion.VersionString' 2>/dev/null | tr -d '\r')"

failed=0
run_step() {
  name="$1"; shift
  echo ""
  echo "--- live-fire: $name"
  if "$@"; then
    echo "--- live-fire: $name PASSED"
  else
    code=$?
    echo "--- live-fire: $name FAILED (exit $code)" >&2
    failed=1
  fi
}

run_step "1/3 packaged desktop boundary, real persistence, digest-bound" \
  sh scripts/artifact-e2e.sh

run_step "2/3 real provider boundary with negative cases (PF-011)" \
  sh scripts/live-fire-local-provider.sh

run_step "3/3 installer state preservation across update/uninstall/rollback" \
  sh scripts/vault-preservation-e2e.sh

echo ""
if [ "$failed" -ne 0 ]; then
  echo "live-fire: FAIL -- at least one production-path proof did not pass" >&2
  exit 1
fi
echo "live-fire: ok (real boundary, real persistence, real dependency, negatives, digest-bound)"

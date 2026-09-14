#!/usr/bin/env bash
# Security gate (DOD-021).
#
# DOD-021 requires "formatting, linting, static analysis, type checking, secret
# scanning, dependency/license/SBOM scanning, IaC/container checks, and security
# tests" to pass under enforced thresholds.
#
# TWO BUGS FIXED HERE:
#  1. The script ended with `[ -f package.json ] && pnpm audit ...`. Under
#     `set -eu` a false test in an `&&` list IS the command's exit status, so the
#     gate would have exited non-zero for a reason unrelated to security. Same
#     defect class as scripts/test-unit.sh. Lanes now use explicit `if`s.
#  2. Secret scanning was absent entirely, though DOD-021 names it explicitly.
#     It is now a lane.
#
# Each lane reports its own exit status, so one failing lane cannot be hidden by
# a later passing one (DOD-024).
set -eu

status=0
run() {
  name="$1"; shift
  echo "--- security: $name"
  if "$@"; then
    echo "    $name: ok"
  else
    code=$?
    echo "    $name: FAIL (exit $code)" >&2
    status=1
  fi
}

run "anti-gaming scan" python3 scripts/anti-gaming-scan.py .
run "secret scan" python3 scripts/secret-scan.py --check
run "sbom currency" python3 scripts/generate-sbom.py --check

if [ -f Cargo.toml ]; then
  run "cargo-deny advisories" cargo deny check advisories
  run "cargo-deny bans" cargo deny check bans
  run "cargo-deny sources" cargo deny check sources
  # Licences is expected to FAIL while ADR-002 (MPL-2.0) is open. It is run and
  # reported rather than skipped: DOD-021 forbids silent continuation.
  echo "--- security: cargo-deny licenses (FAIL expected pending ADR-002)"
  if cargo deny check licenses; then
    echo "    cargo-deny licenses: ok"
  else
    echo "    cargo-deny licenses: FAIL -- see ADR-002" >&2
    status=1
  fi
fi

if [ -f package.json ]; then
  run "pnpm audit" pnpm audit --audit-level high
fi

if [ "$status" -eq 0 ]; then
  echo "security-check: ok"
else
  echo "security-check: FAIL -- see the lanes above" >&2
fi
exit "$status"

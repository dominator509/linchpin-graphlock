#!/usr/bin/env bash
# Stress / exhaustion lane (E2E-009, DOD-038's stress term).
#
# WHY THIS EXISTS. The concurrent-workload trial lives in a cargo test target, so
# the DOD-041 applicability probe for E2E-009 recorded "no systemic stress/exhaustion
# harness exists" long after it did. Naming a real command in the applicability
# matrix requires a command that can be run on its own; this is it.
#
# SCOPE, stated rather than implied. The trial runs 8 writers x 250 events against
# one vault with 4 concurrent readers and a backup thread, and enforces encoded
# floors (p95 write latency, throughput, no hang, no lost events, no failed reads or
# backups, and non-vacuity of both reads and backups) that FAIL the trial. No stress
# concurrency level, duration or throughput target is specified for this clause in
# the repository or in the pack's suite library, so the trial defines and labels its
# own workload as ABBREVIATED; it does not claim the full-scale requirement.
#
# The trial's own history is the reason to trust it: it found three real concurrency
# defects (concurrent first open, migration bookkeeping, backup under a lock) and a
# flaky classification that counted a correct refusal as a failure.
set -eu

[ -f Cargo.toml ] || { echo "stress-lane: product not bootstrapped" >&2; exit 2; }

echo "stress-lane: running the concurrent writers/readers/backup trial"
cargo test -p linchpin-desktop --test stress_concurrency --locked -- --nocapture
echo "stress-lane: ok (see .agent/evidence/stress/report.json; ABBREVIATED workload)"

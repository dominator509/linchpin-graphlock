#!/usr/bin/env bash
# Soak / endurance lane (E2E-018, DOD-038's endurance term).
#
# WHY THIS EXISTS. The soak workload lives in a cargo test target, so the case
# could only be run as part of the whole unit suite -- and the DOD-041 applicability
# probe for E2E-018 recorded "no soak/endurance run exists", which stopped being true
# when the harness was written. This lane is a real, independently runnable entry
# point: an auditor can reproduce the endurance number without running 216 tests,
# and the applicability matrix can name a command instead of an absence.
#
# SCALE, stated rather than implied. The clause's full-scale soak value IS specified
# by the pack: .agent/verification/E2E_SUITE_LIBRARY.md (source of E2E-018,
# E2E-SoakResourceLeakTesting.md) requires 24/48/72+ hours of sustained nominal load
# with continuous telemetry on dedicated persistent infrastructure, and forbids
# time-bounded runners. This lane does NOT satisfy that; whatever it runs is an
# ABBREVIATED trial labeled separately by the harness, and DOD-038 remains
# DEFERRED_LONG_RUNNING until the specified scale is completed on such a host.
#
# Duration: LINCHPIN_SOAK_SECONDS (default 600 here, 30 in the harness itself) and
# LINCHPIN_SOAK_HEARTBEAT (default 30). Runs below 300s are labeled SMOKE_SUBSET by
# the harness and write no evidence, by design.
set -eu

[ -f Cargo.toml ] || { echo "soak-lane: product not bootstrapped" >&2; exit 2; }

SECONDS_TO_RUN="${LINCHPIN_SOAK_SECONDS:-600}"
export LINCHPIN_SOAK_SECONDS="$SECONDS_TO_RUN"

echo "soak-lane: running ${SECONDS_TO_RUN}s abbreviated endurance trial"
cargo test -p linchpin-desktop --test soak_abbreviated --locked -- --nocapture
echo "soak-lane: ok (see .agent/evidence/soak/report.json; ABBREVIATED, not the clause's 24/48/72+ hour scale)"

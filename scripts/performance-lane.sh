#!/usr/bin/env bash
# Performance workload lane (E2E-008, DOD-022).
#
# WHY THIS EXISTS. The encoded workload and its thresholds live in a cargo test
# target, so the DOD-041 applicability probe for E2E-008 recorded "no performance
# workload orchestration exists" after the workload had been built. This lane gives
# the case a real, independently runnable entry point.
#
# WHAT IT ENFORCES, read from the harness rather than restated: 200 conception events
# written one at a time through the production command into a fresh file-backed
# vault, every event read back through a SECOND connection, with p50/p95/max write
# latency bounds, total write and read bounds and a vault-size bound that FAIL the
# run, plus the assertion that the read observed every write (so a fast but lossy
# path cannot pass by dropping work). The environment is recorded in the report.
#
# LIMITS, stated rather than implied: the profile is debug and the workload is
# single-machine, single-process and concurrency-free, so the numbers are regression
# bounds on this host, not published service levels; no cost metric is encoded
# because the product has no metered resource.
set -eu

[ -f Cargo.toml ] || { echo "performance-lane: product not bootstrapped" >&2; exit 2; }

echo "performance-lane: running the 200-event workload with enforced thresholds"
cargo test -p linchpin-desktop --test performance_gate --locked -- --nocapture
echo "performance-lane: ok (see .agent/evidence/performance/report.json)"

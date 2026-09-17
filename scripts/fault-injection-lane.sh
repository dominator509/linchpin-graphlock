#!/usr/bin/env bash
# Fault-injection lane (GEN-115, DOD-036's fault-injection term).
#
# WHY THIS EXISTS. Fault injection was recorded in the DOD-041 applicability table as
# absent ("no fault injection harness exists") long after the recovery drill began
# injecting real faults at the filesystem: total loss of the vault file and its WAL
# siblings, and in-place corruption of a live vault. The exact-artifact lane adds a
# hard process kill. Naming a real command in the matrix requires a command that runs
# the case on its own; this is it.
#
# WHAT IT INJECTS, read from the harness rather than restated here: two fault classes
# are exercised across repeated cycles, recovery runs through the PRODUCTION command,
# and the recovered state is reconciled by content digest through an independent
# connection with the objectives (RPO/RTO/MTTR) measured against stated thresholds.
set -eu

[ -f Cargo.toml ] || { echo "fault-injection-lane: product not bootstrapped" >&2; exit 2; }

echo "fault-injection-lane: running the fault-injected recovery drill"
cargo test -p linchpin-desktop --test recovery_drill --locked -- --nocapture
echo "fault-injection-lane: ok (see .agent/evidence/recovery-drill/report.json)"

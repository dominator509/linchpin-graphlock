# EP-010 / M1 — Run V-000..V-021 without product-code changes

Status: **DONE — every stage has an executed accounting record, and the one
product-code change made during this node is recorded as REMEDIATOR work with its
rerun obligation instead of being hidden.**

## What "run the stages" means here, and how it was accounted

The pack defines 22 stage plans under `.agent/verification/stage-plans/`, each with
the same exit criterion:

> "every owned ID has one truthful status/evidence record; FAIL stays FAIL; blocked
> dependencies are scoped; write stage accounting and next action."

Stage ownership is not invented: the 484 registry rows carry a `default_stage`
column, so every stage's owned IDs are the pack's own grouping. `scripts/stage-accounting.py`
maps each group to its stage plan **by name** (the registry uses the pack's 01..20
numbering, which is offset from the plan numbering — the mapping is asserted in both
directions, and an unmapped group or a mapping to a nonexistent plan is a hard error),
then computes each stage verdict from the per-ID accounting. A stage can never be
greener than its worst owned row.

Result: `.agent/verification/reports/STAGE_ACCOUNTING.md` and
`.agent/verification/state/STAGE_ACCOUNTING.json`.

| Verdict | Stages |
| --- | --- |
| PASS (14) | V-000, V-001, V-002, V-003, V-006, V-007, V-008, V-009, V-010, V-012, V-014, V-016, V-018, V-019 |
| EXTERNAL_REQUIRED (1) | V-015 (manual AT / usability validation owed to human participants) |
| DEFERRED_LONG_RUNNING (1) | V-017 (the pack's 24/48/72+ hour soak scale needs a dedicated host) |
| BLOCKED or PARTIAL (6) | V-004 (GEN-082), V-005 (GEN-081), V-011 (E2E-013), V-013 (GEN-044, GEN-051, GEN-071), V-020 (E2E-011), V-021 (BC-111 plus six external IDs) |

`owned_assignments` is 520 rather than 484 because the pack declares the group
`15-17-PERFORMANCE-STRESS-RECOVERY` as spanning three stages (V-016, V-017, V-018);
those IDs are owned in each of the three, which is recorded rather than forced into
one bucket.

## The harness cursor was lying, and now is not

`harness-next.sh` read `active_stage` from `.agent/verification/state/RUN_STATE.json`,
which still contained the blueprint's seed value:

```json
{"status": "NOT_STARTED", "active_stage": null, "candidate_epoch": 0}
```

so the cursor printed `V-000` forever while every stage had a record. `RUN_STATE.json`
is now written by the stage accounting with the real cursor (status, open stages,
completed stages, candidate commit and epoch), and `harness-next.sh` prints
`RUN_STATE_UNKNOWN:<status>` rather than naming a stage it cannot justify.

## Product-code change during this node: recorded, not hidden

M1's title forbids product-code changes while the stages run (AUDITOR mode). The
stage record above was derived from evidence produced in AUDITOR mode, but the M3
drift accounting then **found a real architecture violation** (`crates/application`
depending on concrete adapters, plus two dead dependency edges) and repairing it
required production-code change. That is REMEDIATOR work:

- the repair is in `crates/application/Cargo.toml` and `crates/application/src/lib.rs`
  (`OperationsSoakTest::with_probe` — the OS working-set read is now an injected port);
- DOD-040 obliges the affected descendants to be re-executed against the new
  candidate, which the settle path does (`scripts/change-invalidation.py` →
  `scripts/rerun-invalidated.py`) with a new epoch recorded in `EPOCH.json`;
- the stage accounting and every currency lane are re-derived after that rerun, so
  the records in this milestone describe the post-repair candidate.

Read this as: the stages were run without product-code change; the drift repair is a
separate, recorded remediation whose reruns this milestone's accounting reflects.

## What is not claimed

- No stage is reported PASS on the strength of another stage's evidence.
- A stage owning a FAIL/PARTIAL/BLOCKED row is reported BLOCKED/PARTIAL, never PASS,
  even when the rest of its IDs are green (V-013 owns 298 IDs and is BLOCKED on three).

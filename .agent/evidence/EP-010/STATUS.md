# EP-010 — Production Readiness and Ship

Status: **COMPLETE — every milestone has executed, machine-derived evidence. Node
closure is not a release verdict: the machine ship gate returns
`CONDITIONAL_EXTERNAL_GATES`.**

Requirements: REQ-SHIP-001. Dependencies: EP-009 (DONE_VERIFIED).

## Milestone state

| Milestone | State | Evidence |
| --- | --- | --- |
| M1 — run V-000..V-021 without product-code changes | **DONE** | `M1/STATUS.md`; `scripts/stage-accounting.py` → `.agent/verification/reports/STAGE_ACCOUNTING.md` — 22 stages, PASS 14, EXTERNAL_REQUIRED 1, DEFERRED_LONG_RUNNING 1, BLOCKED/PARTIAL 6, each naming its owned IDs. `RUN_STATE.json` (the harness cursor `harness-next.sh` reads) now describes this run instead of the blueprint's `NOT_STARTED` seed |
| M2 — account all 484 registry IDs and 42 DoD clauses | **DONE** | `M2/STATUS.md`; 484 accounted (484 unique, 0 missing/duplicated), 42 clauses each with one disposition, both tallies carried together in `RELEASE_GATE.json` |
| M3 — architecture/claim/spec-to-code drift reconciliation | **DONE** | `M3/STATUS.md`; `scripts/architecture-drift.py` implements the nine-question review with three mechanical checks and six bound items; it found real drift (application depending on concrete adapters, two dead dependency edges), the drift was repaired, and the self-test proves the gate still catches planted violations |
| M4 — human UAT/accessibility/external gates and residual risks | **DONE** | `M4/STATUS.md`; `RESIDUAL_RISK_AND_EXTERNAL_GATES.md`, `FINAL_PRODUCTION_READINESS_REPORT.md` and `NEXT_ACTION.md` are generated from the executed state (the pack's templates had become false statements: an empty readiness report, a `NOT_STARTED` status and a stale "sole blocker") |
| M5 — emit the verdict and manual release instructions | **DONE** | `M5/STATUS.md` and `M5/MANUAL_RELEASE.md`; verdict produced only by `scripts/ship-gate.py`, currently `CONDITIONAL_EXTERNAL_GATES` with no blocking clause; deployment is manual by policy |

## Clause dispositions this node answers

| Clause | Disposition | Basis |
| --- | --- | --- |
| DOD-028 final completion report | PASS | `.agent/evidence/FINAL_COMPLETION_REPORT.md`; the report distinguishes verified, partial, unverified, blocked, external, accepted-risk and remaining-limitation items |
| DOD-030 484-ID accounting | PASS | 484 rows, 484 unique IDs, one status each, machine-validated and currency-checked |
| DOD-031 dependency-scoped blocking | PASS | `DEPENDENCY_BLOCKER_GRAPH.json`; blockers affect only proven dependents, and independent lanes continued (visible in the per-stage record) |
| DOD-032 status taxonomy | PASS | every non-PASS row uses the exact taxonomy term; `build-accounting.py` rejects anything outside the set |
| DOD-040 invalidation and rerun | PASS | the epoch digest covers 153 inputs; the settle path reruns affected stages; the cross-version matrix re-provisions when its staged artifacts no longer match the source |
| DOD-041 domain-pack applicability | PASS | 484 applicability decisions with evidence attached; conditional packs activated or skipped from repository evidence |
| DOD-042 machine-validated verdict | PASS | `RELEASE_GATE.json` = `CONDITIONAL_EXTERNAL_GATES`, produced by the gate and checked by two lanes |
| DOD-001 requirement traceability | **EXTERNAL_REQUIRED** | 57 of 59 requirements bound; REQ-REL-001 needs the clean room, REQ-REL-004 needs human sign-off |
| DOD-034 virgin clean room | **EXTERNAL_REQUIRED** | no zero-state host; ADR-004 scopes support to Windows 10+ |
| DOD-038 full-scale soak | **DEFERRED_LONG_RUNNING** | 24/48/72+ h on dedicated infrastructure; the 3600 s trial is labelled `ABBREVIATED` |
| DOD-039 human sign-off | **EXTERNAL_REQUIRED** | ADR-005: no agent may simulate UAT, AT validation or legal review |

## Closure checks

- **Anti-gaming**: `python3 scripts/anti-gaming-scan.py .` and
  `.agent/evidence/ANTI_GAMING_FINDINGS.md`; every match classified, no unresolved hit.
- **Architecture drift**: `scripts/architecture-drift.py` PASS, self-test PASS.
- **DoD accounting**: `scripts/build-dod-status.py --check` (42 rows, currency enforced).
- **Ledger**: the node's `DONE_VERIFIED` event is appended to the hash-chained
  `.agent/state/LEDGER.jsonl` and validated by `scripts/validate-hash-ledger.py`.

## What "COMPLETE" does and does not mean here

It means every milestone of this node has evidence produced by executing something,
the accounting is exhaustive and machine-checked, and the node's own rules are
satisfied. It does **not** mean the product is released: eight stages still carry open
owned IDs (six blocked/partial, one external, one deferred), four clauses are not
PASS, and the verdict is capped at `CONDITIONAL_EXTERNAL_GATES`. Every one of those is
enumerated, with its reason and its unblock condition, in
`.agent/verification/reports/RESIDUAL_RISK_AND_EXTERNAL_GATES.md` — generated from the
state files, so it cannot claim less or more than the evidence supports.

# Final Submission Report

Derived by `scripts/residual-risk-report.py`. This file previously held an early-run narrative
("the graph output is definitively NEXT EP-003", "EP-002 fully implemented with 100% test
coverage") that had become false as the run advanced. It is now derived from the executed state,
so it cannot describe a superseded graph position again.

## Graph position

All eleven nodes EP-000..EP-010 have reached `DONE_VERIFIED` in the hash-chained ledger
(`.agent/state/LEDGER.jsonl`); `sh scripts/graph-next.sh` reports `ALL_DONE`. Node closure means
milestones executed and DoD dispositions recorded — it is **not** a release GO.

## Verified

- 42 DoD clauses: {"DEFERRED_LONG_RUNNING": 1, "EXTERNAL_REQUIRED": 3, "PASS": 38}
- 484 registry IDs: {"DEFERRED_LONG_RUNNING": 1, "EXTERNAL_REQUIRED": 7, "NOT_APPLICABLE": 410, "NOT_RUN_BLOCKED_MATERIAL": 4, "PARTIAL": 4, "PASS": 58}
- Candidate `18203a8`, epoch `9b4b5bb04ea28788…` over 156 inputs
- Artifact: `target\release\bundle\msi\LINCHPIN_0.1.0_x64_en-US.msi` `501ac72e018fa382…` with the installed executable bound to the package payload
- Ship gate: **CONDITIONAL_EXTERNAL_GATES**, blocking clauses: none

## Not verified, not claimed

- Zero-state clean-room install, human UAT/AT validation, 24/48/72+ hour soak, code signing and
  Windows 11 support. Each is an external gate, an accepted limitation or a deferred scale, and each
  is enumerated with its evidence in `RESIDUAL_RISK_AND_EXTERNAL_GATES.md`.

## Deployment

Not deployed. Auto-deploy is `no`; the manual path is
`.agent/evidence/EP-010/M5/MANUAL_RELEASE.md`. No push and no PR were performed by the run.

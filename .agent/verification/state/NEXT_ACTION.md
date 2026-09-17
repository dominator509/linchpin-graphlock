# Next Action

Derived by `scripts/residual-risk-report.py` from the executed state. Historical running notes are
preserved in `NEXT_ACTION_HISTORY.md`; this file always describes the current state.

Ship-gate verdict: **CONDITIONAL_EXTERNAL_GATES** with no blocking clause.

## Clauses waiting on an external participant or host

- **DOD-001** (EXTERNAL_REQUIRED): RECLASSIFIED FROM PARTIAL TO EXTERNAL_REQUIRED, and the clause is NOT satisfied: 57 of 59 requirements carry executed PASS evidence and the remaining two cannot be bound from inside this repository. See `.agent/evidence/DOD-001-unbound-requirements.md`.
- **DOD-034** (EXTERNAL_REQUIRED): A virgin clean room is still required and still absent; this clause cannot be satisfied on a developer host, because the host carries the full Rust/Node/WebView2 toolchain and hidden prerequisites cannot be excluded. See `.agent/evidence/EP-009/M3/STATUS.md`.
- **DOD-039** (EXTERNAL_REQUIRED): CONFIRMED EXTERNAL_REQUIRED by explicit human decision (ADR-005, .agent/evidence/ADR-005-external-signoff-gates.md), not merely left unrun. See `.agent/evidence/ADR-005-external-signoff-gates.md`.

## Clauses deferred by scale

- **DOD-038** (DEFERRED_LONG_RUNNING): CORRECTED MEASUREMENT, and the correction moves the STATUS rather than the evidence. See `apps/desktop/src-tauri/tests/soak_abbreviated.rs`.

## The next lawful manual action

Run the documented install on a clean Windows 10+ machine and record it against REQ-REL-001: that is
the only remaining action that closes more than one gate (DOD-034 / E2E-011 / REQ-REL-001) and it
cannot be performed by tooling on this host. Everything else is either an external human sign-off
(ADR-005) or a dedicated-host endurance run (DOD-038).

# Residual Risk and External Gates

Derived by `scripts/residual-risk-report.py` from the executed state. Do not hand-edit;
`python3 scripts/residual-risk-report.py --check` fails when this text no longer matches
`DOD_STATUS.jsonl`, `COMPLETE_TEST_ACCOUNTING.csv`, `STAGE_ACCOUNTING.json`, the threat model,
the advisory register and `RELEASE_GATE.json`.

- Ship-gate verdict: **CONDITIONAL_EXTERNAL_GATES**
- Blocking clauses: none
- DoD clauses: {"DEFERRED_LONG_RUNNING": 1, "EXTERNAL_REQUIRED": 3, "PASS": 38}
- Registry IDs: {"DEFERRED_LONG_RUNNING": 1, "EXTERNAL_REQUIRED": 7, "NOT_APPLICABLE": 410, "NOT_RUN_BLOCKED_MATERIAL": 4, "PARTIAL": 4, "PASS": 58}
- Candidate: `3608d71`, epoch `a639086301ff52fd…` over 156 inputs
- Artifacts: executable `289bd76cbe2c1a6b…`, MSI `acab80f496749a4d…`

## Clause-level gates that are not PASS

| Clause | Status | Why it is open | Evidence |
| --- | --- | --- | --- |
| DOD-001 | EXTERNAL_REQUIRED | RECLASSIFIED FROM PARTIAL TO EXTERNAL_REQUIRED, and the clause is NOT satisfied: 57 of 59 requirements carry executed PASS evidence and the remaining two cannot be bound from inside this repository. | `.agent/evidence/DOD-001-unbound-requirements.md` |
| DOD-034 | EXTERNAL_REQUIRED | A virgin clean room is still required and still absent; this clause cannot be satisfied on a developer host, because the host carries the full Rust/Node/WebView2 toolchain and hidden prerequisites cannot be excluded. | `.agent/evidence/EP-009/M3/STATUS.md` |
| DOD-038 | DEFERRED_LONG_RUNNING | CORRECTED MEASUREMENT, and the correction moves the STATUS rather than the evidence. | `apps/desktop/src-tauri/tests/soak_abbreviated.rs` |
| DOD-039 | EXTERNAL_REQUIRED | CONFIRMED EXTERNAL_REQUIRED by explicit human decision (ADR-005, .agent/evidence/ADR-005-external-signoff-gates.md), not merely left unrun. | `.agent/evidence/ADR-005-external-signoff-gates.md` |

## Registry rows that are not PASS and not NOT_APPLICABLE

| ID | Status | Stage(s) | Why it is open |
| --- | --- | --- | --- |
| E2E-007 | EXTERNAL_REQUIRED | V-015 | NOT PROVEN: No manual assistive-technology validation and no usability session with real users has been performed; |
| E2E-020 | EXTERNAL_REQUIRED | V-021 | NOT PROVEN: No user acceptance test has been run by any real user and no acceptance record exists to sign. |
| GEN-018 | EXTERNAL_REQUIRED | V-021 | NOT PROVEN: No penetration test has been performed at any depth by any party; |
| GEN-020 | EXTERNAL_REQUIRED | V-021 | NOT PROVEN: No red-team exercise has been run, and attacker creativity is not modelled anywhere in this harness. |
| GEN-021 | EXTERNAL_REQUIRED | V-021 | NOT PROVEN: No purple-team exercise has been run and no detection-engineering feedback loop exists. |
| GEN-103 | EXTERNAL_REQUIRED | V-021 | NOT PROVEN: No Common Criteria evaluation, protection profile or accreditation has been obtained or started. |
| SUP-015 | EXTERNAL_REQUIRED | V-021 | EXTERNAL_REQUIRED, and the evidence cited here is the ACCEPTED owner decision that keeps it so (ADR-005), not a test result. |
| E2E-018 | DEFERRED_LONG_RUNNING | V-017 | NOT PROVEN: Long-duration stability at the specified 24/48/72+ hour scale: no flatline claim is made, no P99-creep-per-day slope is measured, and the abbreviated trials are labeled separately rather than extrapolated. |
| E2E-011 | PARTIAL | V-020 | NOT PASS: not a virgin clean room; |
| E2E-013 | PARTIAL | V-011 | NOT PROVEN: NOT PASS: simultaneous mixed-fleet skew -- two different versions running at once against shared state -- is NOT APPLICABLE to this product and is recorded as such in the evidence, because it is a single-user desktop application with device-local s |
| GEN-044 | PARTIAL | V-013 | NOT PROVEN: NOT PASS: the CWE mapping is prose in the findings file, not a machine-readable weakness column per finding, and no dedicated weakness-enumeration scanner runs. |
| GEN-081 | PARTIAL | V-005 | NOT PROVEN: NOT PASS: no hosted CI configuration exists in this repository (measured: no GitHub Actions, GitLab, Jenkins, CircleCI or Woodpecker config), so nothing prevents a merge that bypasses these gates on a machine that does not run them, and there is no |
| BC-111 | NOT_RUN_BLOCKED_MATERIAL | V-021 | NOT PROVEN: No HSM/TPM test was executed, because no such device is used by the product and none can be truthfully emulated from this host. |
| GEN-051 | NOT_RUN_BLOCKED_MATERIAL | V-013 | NOT PROVEN: No authorization test was executed, because executing one would require inventing an identity model the product does not have. |
| GEN-071 | NOT_RUN_BLOCKED_MATERIAL | V-013 | NOT PROVEN: No sandbox escape or confinement test was executed, because there is no confinement to escape. |
| GEN-082 | NOT_RUN_BLOCKED_MATERIAL | V-004 | NOT PROVEN: No pre-commit hook was tested, because none is installed. |

## Accepted risks (threat model risk register)

| ID | Risk | Status | Decision | Owner |
| --- | --- | --- | --- | --- |
| R-1 | unsigned release (no code-signing certificate) | ACCEPTED | ADR-003 | product owner |
| R-2 | no OS-encrypted container and an in-process keyring | ACCEPTED | scope map UO-01 limitation | product owner |
| R-3 | RPO after the last backup is unbounded (no off-device replication) | ACCEPTED | REQ-REL-005 evidence | product owner |
| R-4 | clean-room install, human UAT and manual AT validation are outstanding | EXTERNAL | ADR-004, ADR-005 | external participants |
| R-5 | full-scale soak (24/48/72+ hours) uncompleted | DEFERRED | DOD-038 DEFERRED_LONG_RUNNING | needs a dedicated host |
| R-6 | no credentialed provider lane is wired, so this product never transmits a credential and a remote provider's auth behaviour is unexercised by it | OPEN | DOD-014 is satisfied for every failure a provider can return to this architecture (401/403/connection-refused/5xx executed against a real loopback socket, mutation-proven by MUT-OPS-014-a); the LANE itself remains a capability gap recorded as UO-09 PARTIAL in the scope map, and wiring one would oblige DOD-014 to be re-run against that lane's real auth failures | product owner |

## Accepted advisories (supply chain)

| Advisory | Package | Decision | Reachability |
| --- | --- | --- | --- |
| — | — | no open advisory | — |

## Scope exclusions that are decisions, not omissions

- **Unsigned release** — accepted, owner decision `.agent/evidence/ADR-003-unsigned-release.md`.
- **Windows 10 or higher** — clean-room scope fixed by `.agent/evidence/ADR-004-clean-room-scope.md`; Windows 11 is not claimed.
- **Human UAT, manual assistive-technology validation, legal review** — remain `EXTERNAL_REQUIRED` by `.agent/evidence/ADR-005-external-signoff-gates.md`; no agent may simulate them.
- **Mixed-fleet / multi-machine version skew** — NOT APPLICABLE: single-user desktop product, device-local storage, no fleet and no such support claimed (recorded in the cross-version matrix report).
- **Credentialed provider lanes** — not wired, and the product reports them as not configured; no credential is transmitted by this build.

## Next lawful manual action

`DOD-001` is `EXTERNAL_REQUIRED`: RECLASSIFIED FROM PARTIAL TO EXTERNAL_REQUIRED, and the clause is NOT satisfied: 57 of 59 requirements carry executed PASS evidence and the remaining two cannot be bound from inside this repository. See `.agent/evidence/DOD-001-unbound-requirements.md`.

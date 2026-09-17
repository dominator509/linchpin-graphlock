# EP-010 / M5 — Emit GO / CONDITIONAL_EXTERNAL_GATES / NO_GO and manual release instructions

Status: **DONE — the verdict is machine-produced, is one of the four lawful values,
and the manual release path is documented and identity-bound.**

## The verdict is produced, not written

`scripts/ship-gate.py` derives the verdict from `DOD_STATUS.jsonl`, the 484-ID
accounting and the artifact digests, and writes
`.agent/verification/reports/RELEASE_GATE.json`. It is checked by
`harness-validate.sh` (`--check`) and by `verify.sh`, so a hand-edited verdict fails
the harness.

Measured at the current candidate:

| Field | Value |
| --- | --- |
| Verdict | **CONDITIONAL_EXTERNAL_GATES** |
| Blocking clauses | none |
| DoD tally | PASS 38, EXTERNAL_REQUIRED 3, DEFERRED_LONG_RUNNING 1 |
| Registry tally | PASS 58, EXTERNAL_REQUIRED 7, PARTIAL 4, NOT_RUN_BLOCKED_MATERIAL 4, DEFERRED_LONG_RUNNING 1, NOT_APPLICABLE 410 |
| Artifact under test | MSI + installed executable bound to the package payload |

`GO` is unreachable by design while DOD-034 (zero-state clean room), DOD-039 (human
UAT/AT sign-off) and DOD-038 (24/48/72+ hour soak) are open, and DOD-001 cannot bind
REQ-REL-001 without the clean room. `NO_GO` is not correct either: nothing is failing.
`CONDITIONAL_EXTERNAL_GATES` is the honest terminal state, and ADR-005 already
recorded that a GO verdict "must be CONDITIONAL_EXTERNAL_GATES at best".

Note carefully what the earlier verdict history shows: before round 61 the same gate
returned a verdict **with** blocking clauses because DOD-014 was PARTIAL. That was
closed by executing the provider auth-failure path, not by reclassifying it, and the
blocking list is now empty. This milestone did not touch that.

## Manual release

`.agent/evidence/EP-010/M5/MANUAL_RELEASE.md`, generated from `RUN_MANIFEST.json`, so
the digests it names cannot drift from the artifact the gate measured. Auto-deploy
authorization is `no` (AGENTS.md section 5e): the artifact is ship-ready and the
deployment is manual. The procedure references only published operator documents
(`DEPLOYMENT.md`, `RELEASE.md`, `ROLLBACK.md`), which `scripts/doc-exec.py` executes
as a gate.

## Deployment state

**Not deployed.** No push, no PR, no tag, no publication was performed by this run.
The next lawful manual action is recorded in
`.agent/verification/state/NEXT_ACTION.md`: run the documented install on a clean
Windows 10+ machine and record it against REQ-REL-001.

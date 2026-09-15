# Human-readable Ledger
| timestamp_utc | node | event | candidate_epoch | evidence |
| --- | --- | --- | --- | --- |
| generation | FORGE | PACK_GENERATED | 0 | blueprint only; no product node is complete |
| 2026-09-08 | EP-000 | CLOSED_BLOCKED | 0 | exact toolchain probes and product-bootstrap evidence unavailable; see .agent/evidence/EP-000/STATUS.md |
| 2026-09-09 | EP-000 | DONE_VERIFIED | EP-000-remediation-001 | .agent/evidence/EP-000/STATUS.md |
| 2026-09-09 | EP-001 | DONE_VERIFIED | EP-001-remediation-001 | .agent/evidence/EP-001/STATUS.md |
| 2026-09-09 | EP-002 | DONE_VERIFIED | EP-002-remediation-001 | .agent/evidence/EP-002/STATUS.md |
| 2026-09-09 | EP-003 | DONE_VERIFIED | EP-003-remediation-001 | .agent/evidence/EP-003/STATUS.md |
| 2026-09-09 | EP-004 | DONE_VERIFIED | EP-004-remediation-001 | .agent/evidence/EP-004/STATUS.md |
| 2026-09-09 | EP-005 | DONE_VERIFIED | EP-005-remediation-001 | .agent/evidence/EP-005/STATUS.md |
| 2026-09-09 | EP-006 | DONE_VERIFIED | EP-006-remediation-001 | .agent/evidence/EP-006/STATUS.md |
| 2026-09-09 | EP-007 | DONE_VERIFIED | EP-007-remediation-001 | .agent/evidence/EP-007/STATUS.md |
| 2026-09-09 | EP-008 | DONE_VERIFIED | EP-008-remediation-001 | .agent/evidence/EP-008/STATUS.md |
| 2026-09-09 | EP-009 | SUPERSEDED | EP-009-attempt-001 | recorded reason 'missing windows environment' was FALSE -- disproven by EP-009's own M1/M2/M3 evidence (host Windows 10.0.19045, real msiexec install/uninstall logs). Superseded by EP-009-remediation-001; status corrected to SUPERSEDED, not DONE_VERIFIED, because M4 (update/rollback) is genuinely still open. Original row preserved in .agent/evidence/EP-009/REMEDIATION.md |
| 2026-09-14 | EP-009 | REMEDIATION_REOPENED | EP-009-remediation-001 | environment blocker disproven by measurement (host Windows 10.0.19045; M1 artifacts and M3 install/launch/uninstall re-verified at current candidate); see .agent/evidence/EP-009/REMEDIATION.md |

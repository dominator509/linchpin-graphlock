# EP-009 — Deployment and Release

Status: **COMPLETE — every milestone has executed evidence. Node closure does not
mean GO: the machine ship gate returns `CONDITIONAL_EXTERNAL_GATES`, and the
external gates are named below.**

Requirements: REQ-REL-001, REQ-REL-005. Dependencies: EP-008 (DONE_VERIFIED).

## Milestone state

| Milestone | State | Evidence |
| --- | --- | --- |
| M1 — frozen Windows installer/app artifacts | **DONE** | `.agent/evidence/EP-009/M1/STATUS.md`; `sh scripts/build.sh` produces MSI + NSIS from one candidate; identity pinned in `RUN_MANIFEST.json` — exe `00286e82…` (12,500,992 B), MSI `a29cdb38…` (5,709,824 B); rebuilds byte-identical (SUP-004) |
| M2 — SBOM / notices / signing / provenance | **DONE** | `.agent/evidence/EP-009/M2/STATUS.md`; CycloneDX 1.5 SBOM (720 components, 720 purls) with a currency lane, generated notices, licence gate enforced (MPL-2.0 by ADR-002), provenance manifest, unsigned release by ADR-003 |
| M3 — clean Win10 install / smoke / E2E | **DONE on this host** | `.agent/evidence/EP-009/M3/STATUS.md`; `sh scripts/smoke-test.sh` exit 0 (silent install → ARP read-back → launch with real window title → clean uninstall); exact-artifact CDP E2E with 15 assertions (`.agent/evidence/artifact-e2e/STATUS.md`); zero-state virgin clean room = DOD-034 `EXTERNAL_REQUIRED` |
| M4 — update / rollback / uninstall vault preservation | **DONE** | `.agent/evidence/EP-009/M4/STATUS.md`; lane 3 `scripts/vault-preservation-e2e.sh` and lane 4 cross-version matrix — 8/8 steps PASS against realistic persistent state at source `62da949ba`, mutation-proven, source-bound and currency-enforced |
| M5 — freeze release candidate digest | **DONE** | `RUN_MANIFEST.json` + `.agent/verification/reports/RELEASE_GATE.json` + `.agent/evidence/sbom/THIRD_PARTY_NOTICES.md`; handing to an auditor is the human gate DOD-039 |

## Clause dispositions this node answers

| Clause | Disposition | Basis |
| --- | --- | --- |
| DOD-003 distribution artifacts | PASS | both formats built, parsed through the Windows Installer COM API as an independent reader |
| DOD-004 exact-artifact smoke/E2E | PASS | `.agent/evidence/artifact-e2e/STATUS.md`; E2E drives the packaged executable, not a dev server |
| DOD-029 candidate/artifact pinning | PASS | `RUN_MANIFEST.json`, refreshed by the settle path, verified by a currency lane |
| DOD-035 upgrade/downgrade/rollback | PASS | cross-version matrix (see M4) |
| DOD-036 backup/restore/recovery objectives | PASS | `.agent/evidence/REQ-REL-005-recovery.md`, `.agent/evidence/recovery-drill/report.json` |
| DOD-034 virgin clean room | **EXTERNAL_REQUIRED** | no zero-state host exists (PF-016); scope Windows 10+ by ADR-004, Windows 11 not claimed (see `M3/STATUS.md`) |
| DOD-039 human UAT / AT / legal sign-off | **EXTERNAL_REQUIRED** | ADR-005; cannot be produced by tooling |
| DOD-038 full-scale soak | **DEFERRED_LONG_RUNNING** | the pack specifies 24/48/72+ h on dedicated infrastructure; abbreviated trial labelled separately |

## What is NOT claimed

1. **Not a GO.** The verdict is produced only by `scripts/ship-gate.py`; at this
   candidate it is `CONDITIONAL_EXTERNAL_GATES` with no blocking clause.
2. **Not a clean-room install.** This host carries the full toolchain; the
   zero-state proof is the external gate above.
3. **Not signed.** ADR-003.
4. **Not deployed.** Auto-deploy authorization is `no` (AGENTS.md §5e), so
   deployment is manual: the artifact is ship-ready and the manual release
   instructions live in `.agent/evidence/EP-010/M5/MANUAL_RELEASE.md`.
5. **REQ-REL-001 remains unbound** in the requirement traceability until the
   clean-room golden path is executed; the reason is recorded in
   `.agent/evidence/DOD-001-unbound-requirements.md` instead of being papered over.

## History preserved

The previous content of this file recorded the node as
`REMEDIATION_REOPENED`, and before that it carried a single false line
(`NOT_RUNNABLE_ENV(...)`) inherited from the pack's authoring sandbox. Both are
preserved in `REMEDIATION.md` in this directory and in the hash-chained ledger,
not deleted: the correction is part of the record.

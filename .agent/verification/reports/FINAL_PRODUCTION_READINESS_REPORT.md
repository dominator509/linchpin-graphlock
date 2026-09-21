# Final Production Readiness Report

Derived by `scripts/residual-risk-report.py` from the executed V-000..V-021 accounting
and the machine ship gate. Do not hand-edit.

- Verdict: **CONDITIONAL_EXTERNAL_GATES** (from `scripts/ship-gate.py`, no blocking clause: none)
- Candidate: `3608d71`, epoch `a639086301ff52fd…` over 156 tracked inputs
- Artifact under test: `target\release\linchpin-desktop.exe` `289bd76cbe2c1a6b…`; installer `target\release\bundle\msi\LINCHPIN_0.1.0_x64_en-US.msi` `acab80f496749a4d…`
- DoD clauses: {"DEFERRED_LONG_RUNNING": 1, "EXTERNAL_REQUIRED": 3, "PASS": 38}
- Registry IDs: {"DEFERRED_LONG_RUNNING": 1, "EXTERNAL_REQUIRED": 7, "NOT_APPLICABLE": 410, "NOT_RUN_BLOCKED_MATERIAL": 4, "PARTIAL": 4, "PASS": 58}

## What is verified

- Every DoD clause the release depends on carries an executed disposition; the clauses that are
  not PASS are `EXTERNAL_REQUIRED` or `DEFERRED_LONG_RUNNING` and are enumerated in
  `RESIDUAL_RISK_AND_EXTERNAL_GATES.md` with their evidence.
- All 484 registry IDs carry one applicability decision and one accounted status; the per-stage
  view is `STAGE_ACCOUNTING.md`.
- The exact artifact was built, installed, launched, driven over CDP, upgraded, downgraded,
  uninstalled and rolled back on the real host, with the user's persistent state reconciled.
- Supply chain, secret scanning, SBOM currency, licence policy, advisory assessment, code metrics,
  threat model, attack trees, weak-crypto scan, coverage and mutation sensitivity all run as lanes.

## What is not verified, and is not claimed

- Zero-state (virgin) clean-room install: `EXTERNAL_REQUIRED`, no such host exists here.
- Human UAT and manual assistive-technology validation: `EXTERNAL_REQUIRED` (ADR-005).
- Full-scale soak at the pack's 24/48/72+ hour scale: `DEFERRED_LONG_RUNNING`; a 3600 s
  abbreviated trial is recorded and labelled separately.
- Code signing: accepted limitation (ADR-003).
- Windows 11: outside the declared clean-room scope (ADR-004).

## Deployment state

Auto-deploy authorization is `no`, so the artifact is ship-ready but **not deployed**; the manual
release path is `.agent/evidence/EP-010/M5/MANUAL_RELEASE.md`.

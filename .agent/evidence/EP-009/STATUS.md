# EP-009 — Deployment and Release

Status: **IN PROGRESS — re-opened for remediation (`EP-009-remediation-001`)**

The previous content of this file was a single line:

```
NOT_RUNNABLE_ENV(Windows-target-only steps this Linux sandbox cannot run)
```

That line was **false for this environment** and is preserved here rather than
deleted so the correction is auditable. This host is Windows
(`Microsoft Windows NT 10.0.19045.0`, target `x86_64-pc-windows-msvc`), and every
Windows-target-only step EP-009 owns has since been executed on it — including
real `msiexec` install/uninstall cycles with logs already committed under
`.agent/evidence/EP-009/M2/`. The line described the Linux sandbox the blueprint
pack was authored in, and it was inherited into the ledger as an unverified
premise that kept `scripts/graph-next.sh` returning
`RUN_BLOCKED dependencies prevent: EP-010` on every run. Full analysis:
`REMEDIATION.md` in this directory.

## Milestone state

| Milestone | State | Evidence |
| --- | --- | --- |
| M1 — frozen Windows installer/app artifacts | **DONE** | `M1/STATUS.md`; `sh scripts/build.sh` exit 0 produces MSI (5,558,272 B) and NSIS (3,121,331 B) from one candidate |
| M2 — SBOM / notices / signing / provenance | **PARTIAL** | SBOM merged across ecosystems, CycloneDX 1.5, 720 components, `generate-sbom.py --check` green. MPL-2.0 resolved by ADR-002. Signing/provenance not applicable by decision (ADR-003) |
| M3 — clean Win10 install / smoke / E2E | **PARTIAL** | `M3/STATUS.md`; `sh scripts/smoke-test.sh` exit 0 — silent install, ARP read-back, launch with window title and `responding=True`, clean uninstall; exact-artifact CDP E2E passes 15 assertions. Zero-state ("virgin") clean room still EXTERNAL_REQUIRED (DOD-034) |
| M4 — update / rollback / uninstall-vault-preservation | **PARTIAL** | `M4/STATUS.md`; `scripts/vault-preservation-e2e.sh` executed as lane 3 of `scripts/test-e2e.sh`. Data preservation mutation-proven. Cross-version rollback not testable: only one version exists |
| M5 — freeze release candidate digest | **DONE** | Artifact identity pinned in `.agent/verification/reports/RELEASE_GATE.json` and repeated in `THIRD_PARTY_NOTICES.md` |

## External prerequisites still open

- **PF-016** — zero-state/virgin target for DOD-034. Scope narrowed to Windows 10
  or higher by ADR-004; Windows 11 is not claimed. No virgin host exists, so
  DOD-034 remains `EXTERNAL_REQUIRED`.
- **PF-017 / PF-018** — human sign-off gates, confirmed `EXTERNAL_REQUIRED` by
  ADR-005.

## Not closed

M4's cross-version paths (version-skew, downgrade to an older schema, mixed fleet)
cannot be executed while only one version exists, so EP-009 is not eligible for
`DONE_VERIFIED` and is recorded as `REMEDIATION_REOPENED` in the ledger rather
than being marked complete.

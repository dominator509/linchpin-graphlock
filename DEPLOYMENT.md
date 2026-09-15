# Deployment
V1 deploy means produce an unsigned Windows 10 or higher x64 installer and release manifest; auto-deploy is not authorized. Release pipeline: clean checkout -> frozen install -> tests -> SBOM/license/security -> build -> digest -> clean Windows 10+ target install -> smoke/E2E -> update/rollback -> uninstall/preserve-vault -> final accounting. Publishing a release is MANUAL after GO. Never test an updater against a user's production vault without a verified backup/rollback path.

## Signing step: NOT PERFORMED (ADR-003)

The pipeline above has no `sign` step. The release is **unsigned** — the PF-015
signing identity is unavailable and the owner accepted an unsigned release as a
documented limitation (`.agent/evidence/ADR-003-unsigned-release.md`). The step
between `build` and `digest` is therefore a digest-only identity step:

```text
sh scripts/artifact-identity.sh target/release/bundle/msi/LINCHPIN_0.1.0_x64_en-US.msi
```

Expect SmartScreen and possible AV warnings on install; that is a property of an
unsigned build, not of a compromised artifact. Verify by SHA-256 against
`RELEASE.md` and `.agent/verification/reports/RELEASE_GATE.json`.

## Platform scope: Windows 10 or higher (ADR-004)

Clean-target verification covers Windows 10 or higher. Windows 11 is not
verified and must not be claimed. A zero-state ("virgin") clean-room install
(DOD-034) remains **EXTERNAL_REQUIRED**: this project has no zero-state host, and
the scope decision does not substitute for one.

## External gates (ADR-005)

Human UAT / manual assistive-technology validation (PF-017) and independent
patent-workflow review (PF-018) stay `EXTERNAL_REQUIRED`. While they are open the
strongest lawful verdict is `CONDITIONAL_EXTERNAL_GATES`, not `GO`.

# Release
A release candidate has immutable commit SHA + installer/app digests. GO requires zero blocked core nodes, all applicable DoD clauses, complete 484-ID accounting, product UO live-fire proofs, anti-gaming PASS, architecture-drift reconciliation, clean Win10 artifact tests, security/license/SBOM proof, manual accessibility/UAT where required, artifact integrity verification and rollback proof. Auto-deploy is no; publication remains manual.

## Signed status: UNSIGNED (accepted limitation — ADR-003)

The shipped artifact is **not code-signed**. The signing identity required by
PF-015 is unavailable, and the project owner accepted an unsigned release as a
documented limitation (`.agent/evidence/ADR-003-unsigned-release.md`). This
section is the disclosure that decision requires.

Consequences a consumer or operator must expect:

- **Windows SmartScreen will warn** on the MSI/NSIS installer ("Windows protected
  your PC"), and antivirus heuristics may flag a freshly built, low-reputation
  binary. This is an expected condition of an unsigned build, **not** evidence of
  tampering or infection.
- **No build-provenance attestation is produced.** None is claimed.

Verify the artifact by digest instead of by signature:

```text
sh scripts/artifact-identity.sh target/release/bundle/msi/LINCHPIN_0.1.0_x64_en-US.msi
```

compare the SHA-256 against `.agent/verification/reports/RELEASE_GATE.json`
(`artifact.msi_sha256`) and against `.agent/evidence/sbom/THIRD_PARTY_NOTICES.md`
under "Artifact identity". A digest proves *integrity* against a value obtained
out-of-band; it does **not** prove *authorship*. That residual risk is accepted
and recorded in ADR-003.

If a signing identity is later acquired, ADR-003 is superseded (not edited) and
this section is replaced by the signature-verification procedure.

## Supported platform: Windows 10 or higher (ADR-004)

Clean-room and compatibility evidence covers **Windows 10 or higher**. Windows 11
is **not** verified — no Windows 11 target was available — so no artifact,
README or release note may describe the product as Windows 11 compatible until
real Windows 11 evidence exists (`.agent/evidence/ADR-004-clean-room-scope.md`).

## External gates: never an unqualified GO (ADR-005)

Human UAT / manual assistive-technology validation (PF-017) and independent
patent-workflow review (PF-018) remain `EXTERNAL_REQUIRED`. They cannot be
satisfied by an agent or by automated tooling, so while they are open the best
lawful verdict is `CONDITIONAL_EXTERNAL_GATES`, never an unqualified `GO`
(`.agent/evidence/ADR-005-external-signoff-gates.md`).

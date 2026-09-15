# ADR-003: Unsigned release accepted as a documented limitation

Status: **ACCEPTED** (human decision, 2026-09-14)
Candidate: `75fc35c` / `x86_64-pc-windows-msvc`
Related: PF-015, DOD-003, EP-009 M2/M4

## Context

DOD-003 requires the production distribution artifact "in every supported format"
with "signatures/attestations when applicable". PF-015 lists a Windows signing
identity as `REQUIRED_BEFORE_DEPLOY` and its stated failure effect is "blocks GA
release".

Probed directly on this host rather than assumed:

```text
Get-ChildItem Cert:\CurrentUser\My   -CodeSigningCert   -> (none)
Get-ChildItem Cert:\LocalMachine\My  -CodeSigningCert   -> (none)
```

There is no signing identity, and obtaining one (OV/EV certificate, or a cloud
signing service) is a purchase and identity-verification process that no agent
can complete. The choice was therefore between leaving DOD-003 blocked
indefinitely and disclosing an unsigned release.

## Decision

The project owner **accepts an unsigned release as a documented limitation**.
Signing is not waived as a gate that was passed; it is recorded as
not-applicable *by explicit decision*, with the consequences disclosed.

## What this does and does not change

- **Does:** DOD-003's "signatures/attestations when applicable" clause is
  satisfied by a recorded decision that they are not applicable, instead of by
  omission. DOD-003 is no longer blocked on PF-015.
- **Does not:** produce a signature or a build-provenance attestation. No
  provenance attestation exists and none is claimed. The artifact-identity
  evidence (`artifact/identity` SHA-256 pins, `scripts/artifact-identity.sh`) is
  what binds a tested artifact to a shipped one; that is checksum-based, not
  cryptographic authorship.
- **Consequence for users:** an unsigned MSI/NSIS installer will trigger Windows
  SmartScreen and may trigger AV heuristics. This is an **expected condition of
  an unsigned build**, and must be stated in the operator-facing documentation so
  it is not misdiagnosed as tampering or infection.

## Disclosure requirement (accepted with the decision)

`SECURITY.md` and the operator documentation must state, wherever the artifact is
described as installable:

1. the release is unsigned;
2. SmartScreen/AV warnings are expected and how to verify the artifact by
   SHA-256 instead;
3. the trust model that replaces signature verification: the published digest
   plus `scripts/artifact-identity.sh`.

## Residual risk (accepted, with owner)

Unsigned distribution means a consumer cannot verify *authorship* from the
artifact alone, only *integrity* against a digest obtained out-of-band. If the
distribution channel is ever compromised, the digest alone does not protect
against a substituted artifact served with a substituted digest. This risk is
accepted for the current scope and would be closed by acquiring a signing
identity (PF-015) — at which point this ADR is superseded rather than edited.

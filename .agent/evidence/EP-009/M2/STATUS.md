# EP-009 / M2 — SBOM, notices, signing and provenance manifest

Status: **DONE for every part this repository can execute; the signing part is a
recorded owner decision (ADR-003), not an omission.**

This file did not exist before: M2's state was only described in the node's own
STATUS.md, which meant the milestone had no evidence document of its own. The
artifacts below are the executed ones.

## 1. SBOM (executed, currency-enforced)

| Field | Value |
| --- | --- |
| Artifact | `.agent/evidence/sbom/linchpin.cdx.json` |
| Format | CycloneDX 1.5 (`bomFormat: CycloneDX`, `specVersion: 1.5`) |
| Components | 720, every one carrying a `purl` (measured: 720 of 720) |
| Generator | `scripts/generate-sbom.py`, derived from the committed `Cargo.lock` and `pnpm-lock.yaml` |
| Currency lane | `python3 scripts/generate-sbom.py --check`, run by `harness-validate.sh` and `verify.sh`; it fails when the SBOM no longer matches the locks |

A measured defect in an earlier revision is preserved here rather than hidden: an
initial generation produced 720 components of which **0** carried a `purl`. That
was found by reading the file back, not by trusting the generator's exit code, and
is recorded in `.agent/evidence/applicable-cases/GEN-072.md`.

## 2. Third-party notices (executed)

`.agent/evidence/sbom/THIRD_PARTY_NOTICES.md` is generated from the same locked
graphs. Licence enforcement is separate and stricter: `cargo deny check licenses`
runs in the supply-chain lane, and the one copyleft dependency that the
permissive-first policy in AGENTS.md §10 flagged — MPL-2.0 — was left **red**
until the owner reviewed it and recorded the decision in
`.agent/evidence/ADR-002-mpl-dependencies.md`. The gate was not weakened to make
it green.

## 3. Signing — decided, not silently skipped

There is no Windows code-signing identity in this environment (PF-015). Rather
than dropping the requirement or claiming a signature, the owner decision in
`.agent/evidence/ADR-003-unsigned-release.md` accepts an unsigned release as a
**documented limitation** with its consequences stated. The ship gate keeps the
residual visible: an unsigned artifact is one of the reasons the final verdict is
`CONDITIONAL_EXTERNAL_GATES` rather than `GO`.

## 4. Provenance manifest (executed)

| Requirement | Evidence |
| --- | --- |
| Exact candidate, base revision, epoch | `.agent/verification/state/RUN_MANIFEST.json` (candidate commit, base revision `3ee9c3f4…`, epoch digest over 153 inputs) |
| Build inputs pinned | committed `Cargo.lock` (`621c52cd…`) and `pnpm-lock.yaml` (`45b0fad1…`); every build and test lane passes `--locked` |
| Artifact identity | `target/release/linchpin-desktop.exe` `00286e82…` (12,500,992 B); MSI `a29cdb38…` (5,709,824 B) |
| Reproducibility | `.cargo/config.toml` sets `-C link-arg=/Brepro`; two forced rebuilds produce a **byte-identical** release executable. `.agent/evidence/SUP-004-reproducible-build-verification.md` |
| Container difference measured, not assumed | the NSIS setups differ in their **compressed stream** (3,233,958 B vs 3,232,330 B) while the payload they install is **byte-identical** (8 files extracted from each). `.agent/evidence/sup004/nsis-divergence.json` |

## What this milestone does not claim

- No signature, no attestation, no timestamp authority: ADR-003.
- No external SBOM consumer has ingested the file; its validity is proven by
  schema-shaped generation from the locks plus the currency lane, not by a
  third-party service.

## Milestone verdict

**DONE.** Every element M2 names is either executed with currency-enforced
evidence (SBOM, notices, provenance) or resolved by a recorded owner decision
with its limitation stated (signing).

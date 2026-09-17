# Next Action

## The one blocking clause

DOD-014 (`PARTIAL`) is the sole blocker in the ship gate. Its credential half has no
subject to test: this architecture wires no credentialed provider (openai/anthropic/xai
report not-wired and `configured=false` in the packaged build) and the pack puts
credential ownership in the first-party CLI tools (SPEC-005 / REQ-LLM-004), with terms
review preceding any integration. The dependency half is executed and mutation-proven.
Closing it needs an owner decision to build a credentialed lane under change control, or
an explicit, boundary-stated reclassification of the credential half. No agent may do
either silently.

DOD-038 (`DEFERRED_LONG_RUNNING`) needs a dedicated host for the pack's 24/48/72+ hour
soak; DOD-001/034/039 need the clean room and named human validators. The next lawful
MANUAL action is unchanged: run the documented install on a clean machine and record it
(REQ-REL-001).

## SUP-004, fully characterised, blocked on packaging decisions

All three artifacts are now measured: the release executable is BYTE-IDENTICAL across
rebuilds (`/Brepro`); the MSI carries an identical payload but WiX regenerates
`ProductCode` per build; the NSIS setup carries an identical payload (8 files, every digest
equal) inside a non-reproducible COMPRESSED STREAM. Two unblock paths, both packaging
decisions rather than build fixes:

1. Pin `ProductCode` per version through a custom WiX template or fragment (Tauri's schema
   exposes `upgradeCode` only) — carries an upgrade-semantics review, because a pinned
   ProductCode is what makes patching possible and must change between versions.
2. Make NSIS compression deterministic, or stop shipping NSIS and ship the MSI alone (the
   format the installer lanes and the clean-room procedure already use).

## Applicability rows still open

- **GEN-013 (security code metrics)** and **GEN-014 (complexity)** are adjacent but
  distinct: coverage, mutation sensitivity and reachability classification are measured and
  enforced; no complexity or security-density metric exists. A real measurement needs a tool
  or a hand-rolled AST pass, so measure the cost before claiming either.
- **GEN-105/107/108/109** remain the formal-methods cluster and stay NOT_APPLICABLE unless a
  real specification language or model checker appears; GEN-108's reason points at
  GEN-093/GEN-106 so the boundary is explicit.


## Applicability rows worth closing next, with real harnesses

Round 54 closed GEN-091 by authoring a threat model and BINDING it
(`check-threat-model.py`); round 55 did the same for GEN-092 (attack trees bound to the
model and to the code) and claimed GEN-093 from executed fail-closed negatives. Both new
validators run in verify.sh, and both refused to pass on first use — the pattern works.

Next candidates, each needing its own verification before any status moves:

- **GEN-013 (security code metrics)** and **GEN-014 (complexity)** are adjacent but
  distinct: coverage, mutation sensitivity and reachability classification are measured
  and enforced; no complexity or security-density metric exists. Do not merge them. A real
  `cycomatic`-style measurement would need a tool or a hand-rolled AST pass, so measure the
  cost before claiming either.
- **GEN-105/107/108/109** remain the formal-methods cluster and stay NOT_APPLICABLE
  unless a real specification language or model checker appears; GEN-108's reason now
  points at GEN-093/GEN-106 so the boundary is explicit.
- **NSIS byte isolation (SUP-004)**: measure which bytes differ between two NSIS builds
  with `/Brepro` in place; the MSI residual is already characterised as WiX's per-build
  `ProductCode`, and the remediation needs a custom WiX template plus an upgrade-semantics
  review.



Standing instruction: name the harness, add a per-ID case result whose citations
`scripts/applicable-case-evidence.py --check` re-reads from the tree, then rebuild
accounting. `scripts/audit-applicability-absences.py` remains a reading list, never a
decision tool (80 of 89 claims produce a keyword signal).


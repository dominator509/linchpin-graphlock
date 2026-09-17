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

## SUP-004, now blocked only on installer-package identity

The release executable is byte-identical across rebuilds (`/Brepro`, round 53). What is
left: the MSI's `ProductCode` regenerates per build (UpgradeCode/ProductVersion stable)
and the NSIS container's differing bytes are NOT yet isolated. Next steps, in order:
isolate the NSIS difference byte-for-byte (measurement only); then pin `ProductCode` per
version through a custom WiX template, which Tauri's schema does not expose and which
carries an upgrade-semantics review.

## Applicability rows worth closing next, with real harnesses

Round 54 closed GEN-091 by authoring a threat model and BINDING it
(`check-threat-model.py`); round 55 did the same for GEN-092 (attack trees bound to the
model and to the code) and claimed GEN-093 from executed fail-closed negatives. Both new
validators run in verify.sh, and both refused to pass on first use — the pattern works.

Next candidates, each needing its own verification before any status moves:

- **GEN-106 (property-based security testing)** claims none exists while
  `test_sanitize_path_property_over_generated_corpus` generates a corpus and the fuzz
  campaign mutates one. Verify the claim against those two before moving it; GEN-107/108
  sit in the formal-methods cluster and stay NOT_APPLICABLE unless a real property harness
  appears.
- **GEN-013 (security code metrics)** and **GEN-014 (complexity)** are adjacent but
  distinct: coverage, mutation sensitivity and reachability classification are measured
  and enforced; no complexity or security-density metric exists. Do not merge them.
- **GEN-043 (vulnerability assessment record)** — advisories are scanned and disclosed
  under GEN-042; no severity/exploitability assessment exists.
- **GEN-058 (weak-primitive detection)** — the implementation is tested under GEN-057;
  nothing scans for weak primitives, and the dependency ban list names none.

Standing instruction: name the harness, add a per-ID case result whose citations
`scripts/applicable-case-evidence.py --check` re-reads from the tree, then rebuild
accounting. `scripts/audit-applicability-absences.py` remains a reading list, never a
decision tool (80 of 89 claims produce a keyword signal).


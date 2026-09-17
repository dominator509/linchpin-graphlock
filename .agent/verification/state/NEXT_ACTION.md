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

Round 54 closed GEN-091 by authoring a threat model and BINDING it (`check-threat-model.py`
fails on drift, and runs in verify.sh). The same shape applies to its neighbours:

- **GEN-093 / GEN-094 (abuse and misuse case suites)** are recorded NOT_APPLICABLE, but
  the product already asserts misuse scenarios fail closed: a non-loopback provider
  endpoint refused with POLICY and no attempt, an export of restricted content refused by
  the firewall, an ungranted MCP capability denied, malformed filing input refused, a
  zip-slip path rejected, an empty prompt rejected before any I/O. Reclassifying ONE of
  them (with a per-ID case result citing those tests) and cross-referencing the other is
  the honest move; claiming both from one evidence set would double-count.
- **GEN-092 (attack trees)** could then be authored FROM the threat model, with a
  validator binding each leaf to a modelled threat -- the pattern that made GEN-091 a
  case rather than a document.
- **GEN-106 (property-based security testing)** claims none exists while
  `test_sanitize_path_property_over_generated_corpus` generates a corpus and the fuzz
  campaign mutates one; verify before moving it. GEN-107/108 sit in the formal-methods
  cluster and stay NOT_APPLICABLE unless a real property harness appears.

Standing instruction: name the harness, add a per-ID case result whose citations
`scripts/applicable-case-evidence.py --check` re-reads from the tree, then rebuild
accounting. `scripts/audit-applicability-absences.py` remains a reading list, never a
decision tool (80 of 89 claims produce a keyword signal).

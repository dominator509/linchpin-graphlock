# Next Action

## The one blocking clause

DOD-014 (`PARTIAL`) is the sole blocker in the ship gate. Its credential half has no
subject to test: this architecture wires no credentialed provider (openai/anthropic/xai
report not-wired and `configured=false` in the packaged build), and the pack's own
governance puts credential ownership in the first-party CLI tools (SPEC-005 /
REQ-LLM-004) with terms review preceding any integration. The dependency half is
executed and mutation-proven (unreachable endpoint, non-loopback refusal, bounded retry
exhaustion). Closing the clause requires either an owner decision to build a
credentialed lane under change control, or an explicit, boundary-stated
reclassification of the credential half. No agent may do either silently.

DOD-038 (`DEFERRED_LONG_RUNNING`) needs a dedicated host for the pack's 24/48/72+ hour
soak; DOD-001/034/039 need the clean room and named human validators. The next lawful
MANUAL action is unchanged: run the documented install on a clean machine and record it
(REQ-REL-001).

## Standing work on the applicability matrix

Rounds 50-52 corrected 13 rows whose absence claims or decisions were wrong, and added
two guards: duplicate probe keys are rejected by `scripts/validate-generated-pack.py`,
and a case result for an ID decided `NOT_APPLICABLE` is rejected by
`scripts/build-accounting.py`. Both failure modes were silent before -- the second one
was introduced by a round-51 edit and caught only because the tally was re-checked.

`scripts/audit-applicability-absences.py` is a READING LIST, not a decision tool: it
reports a keyword signal for 80 of 89 remaining absence claims, the same
false-positive rate this repository already documented for regex probes. It is
deliberately not a gate lane.

## SUP-004 is now blocked only on installer-package identity

Round 53 made the release **executable** byte-identical across rebuilds (`/Brepro`) and
isolated what is left: the MSI's `ProductCode` is regenerated per build while
`UpgradeCode`/`ProductVersion` stay stable, and the NSIS container's differing bytes are
**not yet isolated**. Two bounded next steps, in order:

1. Isolate the NSIS difference byte-for-byte (the MSI is already characterised through
   the Windows Installer COM API) — measurement only, no product change.
2. Pin `ProductCode` per version through a custom WiX template or fragment. Tauri's
   config schema exposes `upgradeCode` only, so this means shipping a template, and it
   carries an upgrade-semantics review: a pinned ProductCode is what makes patching
   possible and must change between versions.

## Worth building next, with a real harness behind it

**GEN-091 (threat modelling)** is recorded NOT_APPLICABLE with "no threat model artefact
exists", which is true today and is closable by work rather than by an outside party: a
threat-model document covering the declared assets (UO-01..12), the five truth
boundaries (TB-1..TB-5), the disclosure firewall, the loopback-only transport, the vault
container and the MCP capability boundary — plus a validator that fails when the model
drifts from the code (a boundary added in `crates/domain/src/scope.rs` but not modelled,
or a mitigation cited without its test). That turns a skip into an executed case with a
discriminating check, in the style that has worked all run.

What remains is bounded, individual review of rows whose claim might have aged, in the
style that worked: name the harness that exists, add a per-ID case result whose
citations `scripts/applicable-case-evidence.py --check` re-reads from the tree, then
rebuild accounting. Rows not yet examined in that style include GEN-013/014 (security
metrics, complexity measurement), GEN-043 (vulnerability assessment record), GEN-058
(weak-primitive detection), GEN-091 (threat-model artefact), GEN-106/107/108
(property-based and formal verification), and SUP-004 (the reproducible-build residual,
where the honest status stays PARTIAL until the linker metadata is eliminated).

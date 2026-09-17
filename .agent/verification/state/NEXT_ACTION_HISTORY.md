# Next Action — historical running notes

Preserved from the hand-written `NEXT_ACTION.md` when it became a derived document.

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

## SUP-004 is closed on the gate's own criteria

Round 60 re-read the gate and measured, and both halves of my earlier remediation plan were
wrong: SUP-004 does NOT require byte-identical containers (its method covers the
non-feasible case by normalising documented fields and comparing semantic contents), and
pinning `ProductCode` would not have produced identical MSIs because the summary stream's
**PackageCode** and timestamps are build-specific by format design. The clause now passes
with: byte-identical executable, semantically identical payloads in both packages,
installed-binary binding, and the environment/locale/timezone/network record its method
requires. Cross-machine reproducibility stays where it belongs — DOD-034's
EXTERNAL_REQUIRED virgin-clean-room gate (PF-016).


## Applicability rows still open

- **GEN-105/107/108/109** remain the formal-methods cluster (model-based tests, a formal
  verification harness, formal security-property verification, a theorem prover) and stay
  NOT_APPLICABLE unless a real specification language or model checker appears. GEN-108's
  reason points at GEN-093/GEN-106 so the boundary is explicit.
- **GEN-092's neighbours** (GEN-094 misuse, GEN-093 now closed) and the remaining ~400
  NOT_APPLICABLE rows have been audited by keyword only, which is a reading list rather than
  evidence; the rows that matter are closed one at a time with a harness and verified
  citations, which is the only method that has worked.

## The metrics ratchet, as a standing obligation

`scripts/code-metrics.py` enforces production complexity <= 40, production panicking calls
<= 15, zero production todos and one allowlisted `unsafe` block. The panic bound is a
RATCHET: it may be lowered, and raising it needs a recorded decision in the source comment.
Measuring it again after any production change is the point, and `--check` fails when the
numbers move without the report being regenerated.



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


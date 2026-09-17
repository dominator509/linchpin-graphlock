# Next Action

## Open measurement audit (round 50 finding, not yet closed)

The General-pack probe table (`GEN_PROBES` in `scripts/build-applicability.py`) still
carries 122 absence claims. Five E2E rows of the same kind were corrected in round 50
because the harness they denied had since been built; the General table has NOT been
audited the same way. Candidates that look stale on first inspection, each of which
needs its own verification before any status moves:

- `GEN-027` "no mutation-based fuzzing engine is configured" — the mutation-fuzz
  campaign exists (`apps/desktop/src-tauri/tests/fuzz_campaign.rs`, mutation-proven by
  MUT-FUZZ-001). Whether a hand-written seeded mutator counts as an "engine" decides
  this row; the decision must be recorded rather than assumed.
- `GEN-115` "no fault injection harness exists" — the recovery drill injects total
  loss and in-place corruption at the filesystem and the exact-artifact lane kills the
  process (DOD-015/DOD-036).
- Borderline, each needing evidence before any change: `GEN-036` (client-side security
  suite for the WebView — Playwright + artifact lanes exist), `GEN-057`/`GEN-058`
  (crypto usage / weak crypto tests — `evidence::threat_control_tests` exists),
  `GEN-065` (configuration hardening baseline — release config parsing tests exist),
  `GEN-111` (incident response rehearsal — crash-reporter incident tests and the
  recovery drill exist), `GEN-119` (anomaly detection — diagnostics derive alerts).

Rule for closing them: the same one applied in round 50 — name the real command in the
probe, add a per-ID case result whose citations are re-read from the tree by
`scripts/applicable-case-evidence.py --check`, and rebuild accounting. No row may move
on a blanket reclassification.

## Immediate next action

DOD-014 remains the only blocking clause (`PARTIAL`): the credential half of
fail-closed behaviour has no lane to test, because this architecture wires no
credentialed provider and the pack's governance (terms review) precedes any provider
integration. Resolving it requires either an owner decision to build such a lane under
change control, or an explicit reclassification of that half with its boundary stated.
DOD-038 is `DEFERRED_LONG_RUNNING` and needs a dedicated host for the pack's
24/48/72+ hour soak scale; nothing in this repository can shorten that.

The next lawful MANUAL action is unchanged: run the documented install on a clean
machine and record it (REQ-REL-001, DOD-034).

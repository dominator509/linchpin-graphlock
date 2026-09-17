# DOD-001 — Unbound Requirements: Measured Implementation State

Generated during the DOD-001 audit. Candidate `b511ae9` (base `962e365`).
**Refreshed at candidate `fbc9a49`** — the counts below moved and are re-measured,
not carried forward (see "Progress" at the end).

DOD-001 RULE: "Every promised behavior has a stable requirement ID and at least
one acceptance test before implementation is declared complete."

`scripts/bind-requirements.py` reports 35 of 59 requirements bound to executed
acceptance tests at the time of the original audit. The remaining **24** were
`NO_BOUND_TEST`. That single status conflates two very different situations, and
the distinction decides what work is actually required:

* **ABSENT behavior** — no acceptance test *can* pass, because the promised
  behavior does not exist. The remedy is implementation, not a test.
* **PRESENT behavior** — the behavior exists but nothing asserts it. The remedy
  is a test.

This file records the state of each unbound requirement **as directly
verified**, and marks the rest `UNASSESSED` rather than guessing.

## Method note — and a caution about automated probes

A first attempt measured these requirements with regex probes over the source
tree. It produced **false results in both directions** and was discarded:

| Claim from probes | Direct verification | Why the probe lied |
| --- | --- | --- |
| REQ-UI-001 "nav Docket / nav Commercialize" present | absent | matched the words in other files, not the UI's navigation |
| REQ-UI-004 "destructive confirm" present | absent | matched `confirmation_number`, a USPTO receipt field |
| REQ-PAT-003 claim classes present | absent | `Select-String` is case-insensitive by default and matched the English word "observation" in a comment |

Conclusion drawn and applied here: for a claim about the existence of a
capability, **read the definition site**. A lexical probe over a concatenated
tree is not evidence of implementation. Only rows verified by reading code are
listed as findings below.

## Verified findings

| Requirement | State | Evidence (directly inspected) |
| --- | --- | --- |
| REQ-UI-001 | **ABSENT** | Promises 13 primary-navigation surfaces. `apps/desktop/src/App.tsx` renders 3 sections: `Conception Lab` (line 377), `Patent Architect` (line 466), `Filing` (line 497). No occurrence of Dashboard, Opportunity Radar, Research War Room, Docket, Prosecution, Commercialize, Evidence Vault, Integrations, Incidents or Settings anywhere in the file. |
| REQ-UI-002 | **PARTIAL** | Promises 4 persistent badges (confidentiality, filing/priority, evidence coverage, provider egress). The file contains exactly one: `<p>Local-First Confidentiality Boundary Active.</p>` (line 360). No badge for filing/priority, evidence coverage or egress state. |
| REQ-UI-003 | **PARTIAL** | The prohibition half holds and is covered by an executed E2E test (5 prohibited phrases asserted absent). The preferred vocabulary half is 1 of 4: `draft` appears 15 times; `screen`, `evidence` and `uncertainty` appear 0 times in `App.tsx`. |
| REQ-UI-004 | **ABSENT** | No consequence-specific confirmation or audit for destructive/high-impact actions exists in the UI. The only `confirm` substring is `confirmation_number`, a filing-receipt field. |
| REQ-DOM-005 | **ABSENT** | Promises the claim graph "enforces dependency/category/limitation identity". `crates/domain/src/lib.rs` `ClaimGraph::map_threat(reference_id, limitation_id, description)` (line 263) pushes the threat with **no validation** that either `EntityId` refers to an existing limitation or reference, and returns `ThreatMap`, not `Result`. The struct exists; the enforcement does not. |
| REQ-DOM-006 | **ABSENT** | No support-matrix type or function exists. |
| REQ-DOM-009 | **ABSENT** | No docket ruleset source/version type or function exists. |
| REQ-DOM-010 | **FIXED and TESTED** | `RepairCapsule` and `RedactionPolicy` exist in `crates/crash_reporter/src/lib.rs`. Audit found the pass did **not** fully redact and the guard claimed a check it never performed — both fixed this round; see `AG-014` below. Three `/// covers: REQ-DOM-010` tests now pass. |
| REQ-PAT-003 | **ABSENT** | Promises each research claim carries exactly one of 7 classes (`OBSERVATION`, `HYPOTHESIS`, `INFERENCE`, `LEGAL_RULE_SUMMARY`, `MARKET_SIGNAL`, `PATENT_THREAT`, `COMMERCIAL_TARGET_ASSERTION`). There is no `ClaimClass` enum and no occurrence of any class name as an identifier anywhere in the Rust or TypeScript sources. |

## Correction to the DOD-001 disposition's "UI/A11Y" grouping

The previous DOD-001 note listed the unbound set as "UI/A11Y, licensing,
release/provenance, some domain rules". For the UI subset that framing was too
generous: REQ-UI-001 and REQ-UI-004 are not untested features, they are
**unimplemented** features. Recorded here so the gap is not read as
test-writing work.

## UNASSESSED — explicitly not claimed either way

These were unmeasured at the time of the original audit. They are **not**
reported as absent, and no probe result for them is trusted:

REQ-COM-002, REQ-COM-003, REQ-DATA-003, REQ-FOUND-001, REQ-FOUND-002,
REQ-LIC-001, REQ-LIC-002, REQ-LIC-003, REQ-OPS-002, REQ-PLAT-002, REQ-REL-001,
REQ-REL-002, REQ-REL-004, REQ-REL-005, REQ-SCOPE-001.

All of those except REQ-COM-002, REQ-REL-001, REQ-REL-004 and REQ-SCOPE-001 have
since been assessed, implemented where they were absent, and bound to executed
acceptance tests (see "Progress").

## Binder defect found and fixed

`scripts/bind-requirements.py` scanned only `*.rs` sources and collected test
IDs only from `cargo test --list`. The 14 executed Playwright acceptance tests
in `apps/desktop/e2e/` were therefore **invisible**, so requirements they cover
could never bind no matter how the spec was written. The binder now also
discovers `*.spec.ts` under `apps/desktop/e2e` and attributes a requirement to a
Playwright test only when an explicit `covers:` marker appears **inside that
test's own body** — a file-level docblock mention is deliberately not counted,
because it asserts suite-wide scope and would bind requirements to tests that do
not exercise them. E2E test IDs count as collected only when Playwright's JSON
report exists for the candidate, so presence in a spec file is never mistaken
for execution.

MUTATION-PROVEN: injecting `// covers: REQ-UI-003` into the language test's body
raises bound requirements from 35 to 36 (`NO_BOUND_TEST` 24 → 23); restoring the
file returns both counts to their prior values.

This fix does **not** by itself change any status, and that is the honest
outcome: the two UI requirements nearest to binding are only partially
implemented (table above), so binding them would have **overstated** DOD-001
compliance. The fix removes a real blind spot without inflating the result.

## Consequence for DOD-001

**Reclassified from `PARTIAL` to `EXTERNAL_REQUIRED`, and the clause is NOT
satisfied.** Measured at candidate `c317684` by `python3 scripts/bind-requirements.py`:
222 collected tests, 59 requirements, **57 bound to an executed PASS**, 2
`NO_BOUND_TEST`.

The taxonomy distinction decides the status, and it is not a wording preference:

* `PARTIAL` means work remains **that this run could do**;
* `EXTERNAL_REQUIRED` means the remaining condition needs a **real outside
  participant**.

Both residues are the second kind, and each is the subject of a clause this run
already records as `EXTERNAL_REQUIRED`:

| Requirement | Residual | Same subject as |
| --- | --- | --- |
| REQ-REL-001 | signed virgin clean-room install and boot of the exact final artifact | DOD-034 (`EXTERNAL_REQUIRED`, ADR-004) |
| REQ-REL-004 | manual AT validation, human UAT, long-running fuzz/soak | DOD-039 (`EXTERNAL_REQUIRED`, ADR-005) |

Two properties were checked before making the change, because a reclassification
that quietly removes a release blocker would be laundering:

1. **The clause's own OR ELSE still applies.** DOD-001 says "the item is
   INCOMPLETE and NODE_DONE/GO is prohibited until the mapping exists and
   passes". The mapping does not exist for 2 of 59 items, so GO remains
   prohibited; `EXTERNAL_REQUIRED` records where the condition lives, not that it
   is waived.
2. **The verdict did not move.** Before: `NO_GO` with one blocker,
   `DOD-001=PARTIAL`. After: `NO_GO` with three blockers
   (`DOD-014=PARTIAL`, `DOD-035=FAIL`, `DOD-038=PARTIAL`) and three external
   conditions (`DOD-001`, `DOD-034`, `DOD-039`). The blocker list grew because
   `scripts/ship-gate.py` was, in the same round, changed to account for **every**
   release-scoped clause in `DOD_REGISTRY.csv` instead of a hand-written list of
   17 that had silently omitted DOD-014, DOD-034, DOD-035, DOD-038 and DOD-039.

Earlier in the run, 24 requirements were unbound and 7 were directly verified
absent or partial. Those required implementation, not reclassification, and were
closed by implementation (REQ-DOM-005 identity enforcement, REQ-DOM-010
redaction, REQ-DOM-006 support matrix, REQ-DOM-009 docket rulesets, REQ-PAT-003
claim classes, REQ-UI-001/004 navigation and consequence-specific confirmation,
REQ-UI-002/003 badges and vocabulary) plus tests. The two that remain are the
only ones whose closure needs someone this repository does not contain.

## Progress this round

**REQ-DOM-005 — implemented.** `ClaimGraph` promised to "enforce
dependency/category/limitation identity" and did not. `map_threat` and
`add_design_around` now validate identity and return
`Result<_, ClaimGraphError>`; a threat naming an unknown limitation or reference
is rejected instead of stored. Three tests, mutation-proven (removing the checks
fails two of them).

**REQ-DOM-010 — audited, two real defects fixed, three tests added.** Reported
in full as `AG-014` in `ANTI_GAMING_FINDINGS.md`. Summary: the redaction pass
missed credentials that were directly wrapped in punctuation (compact JSON,
parentheses, `key=…;`), and `is_safe_for_export()` asserted only two boolean
flags while its doc comment claimed it also verified that no registered secret
survived — a guard claiming a check it never ran. Both fixed and mutation-proven.

Bound requirements therefore move 35 → 37, collected tests 129 → 135, and
`NO_BOUND_TEST` 24 → 22.

## Progress — refreshed measurement at candidate `291a5cc`

Re-measured with `python3 scripts/bind-requirements.py`, not carried forward:

| Measure | Original audit | Now |
| --- | --- | --- |
| Collected tests | 129 | **222** |
| Requirements found | 59 | 59 |
| Requirements bound to an executed PASS | 35 | **57** |
| `NO_BOUND_TEST` | 24 | **2** |

The two that remain unbound are external, and neither can be closed from inside
this repository:

| Requirement | Why it is still unbound | Next action |
| --- | --- | --- |
| REQ-REL-001 | Demands a **signed virgin clean room** install and boot of the exact final artifact. No clean-room environment exists on this host, and no agent may sign for one. Recorded `EXTERNAL_REQUIRED` under ADR-004 (supported scope Windows 10 or higher). | External: a clean-room operator runs the documented install and records it. |
| REQ-REL-004 | Demands broad evidence that no automated run can produce: manual assistive-technology validation, human UAT, and long-running fuzz/soak. PF-017/PF-018 are `HUMAN_EXTERNAL` and unmet (ADR-005). | External: named human validators; the long-running half stays `DEFERRED_LONG_RUNNING`. |

Two requirements left this table this round and are now closed against executed
evidence:

* **REQ-SCOPE-001** — the UO-01..12 mapping and the five truth boundaries are
  declared in code, served over IPC, rendered in Settings, and asserted at the
  product boundary and against the packaged executable.
  `.agent/evidence/REQ-SCOPE-001-scope-and-truth-boundaries.md`
* **REQ-COM-002** — the chain-of-title timeline exists and reports gaps rather
  than bridging them; recordation is evidence and never validation; a lien is
  surfaced and blocks readiness. `.agent/evidence/REQ-COM-002-chain-of-title.md`

## The one change in this file that is a CORRECTION, not progress

The original audit recorded REQ-REL-005 as `UNASSESSED`. It is now implemented,
executed and bound: backup/restore at the storage, command, IPC and UI layers,
five fault-injected recovery cycles with measured RPO/RTO/MTTR, and three
machine-rerunnable mutation proofs. Two real defects were found while doing it,
one of which **destroyed data**: a mistyped backup path was accepted because
`Vault::digest_of` inherited SQLite's open-or-create semantics, so the restore
replaced the live vault with an empty database. Full record:
`.agent/evidence/REQ-REL-005-recovery.md`.

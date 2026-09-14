# Verification Accounting Status (honest, measured)

Measured 2026-09-10 at candidate `41e0b20`.

## Headline

```
$ sh scripts/harness-accounting.sh
registry 484 accounted 0
[exit=2]
```

**0 of 484 registry IDs are accounted.** The harness exits 2, which is a hard
failure by the script's own contract (`if len(acct)!=len(rows): raise SystemExit(2)`).

DOD-030 requires all 484 IDs to receive an individual applicability decision and
final status with zero missing or duplicated. That condition is **not met**, so
per DOD-030 the project cannot be declared production-ready, and per DOD-042 no
GO verdict is possible.

## Supporting state (all measured, all empty/NOT_EVALUATED)

| Artifact | Content |
| --- | --- |
| `.agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv` | header row only — **0 rows** |
| `.agent/verification/state/TEST_LEDGER.jsonl` | **empty** (2 bytes) |
| `.agent/verification/state/DOD_STATUS.jsonl` | **empty** (2 bytes) |
| `.agent/verification/state/STATUS_TRANSITION_AUDIT.jsonl` | **empty** (2 bytes) |
| `.agent/verification/state/EVIDENCE_INDEX.json` | `{"evidence": []}` |
| `.agent/verification/reports/RELEASE_GATE.json` | `{"verdict":"NOT_EVALUATED","reason":"Blueprint only; no candidate artifact exists."}` |
| `.agent/verification/state/RUN_STATE.json` | `{"status":"NOT_STARTED","active_stage":null,"candidate_epoch":0}` |
| `.agent/verification/state/NEXT_ACTION.md` | "Initialize the greenfield repository, run preflight, then execute EP-000." |

## Registry composition (484 rows verified present)

| Source group / kind | Count |
| --- | --- |
| Blockchain, atomic-security | 202 |
| HIPAA, atomic-security | 125 |
| General, atomic-security | 122 |
| E2E, orchestrator | 20 |
| Supplemental, production-gate | 15 |
| **Total** | **484** |

Applicability column: 327 `conditional*`, 122 `evaluate`, 8 `broadly-applicable`,
plus 27 further `conditional-*` refinements. Because 449 of 484 rows are
conditional or "evaluate", DOD-041 requires an **evidence-attached applicability
decision for every one** — repository evidence, never assumption. None of those
decisions exist yet.

## The central honesty problem

The ledger and `FINAL_SUBMIT_REPORT.md` assert that EP-000..EP-008 are
`DONE_VERIFIED`. Under the pack's own law a node closes only when "every
milestone is committed, its requirement/test/evidence rows are current,
anti-gaming review passes, and all owned DoD clauses have an evidence-backed
disposition" (AGENTS.md §4).

Measured reality contradicts those closures:

- **Zero** requirement→test traceability rows exist (`REQUIREMENT_TRACEABILITY.csv` / `CLAIM_TO_RELEASE_TRACEABILITY.csv` unpopulated).
- **Zero** of 484 tests have been run or accounted. Actual executed tests are the 40 Rust unit tests in the workspace (`cargo test`), which are not mapped to registry IDs.
- `REQ-REL-001` and `REQ-REL-005` — the requirements EP-009 claims to own — are **defined nowhere in the repository**; the only occurrence is the EP-009 ExecPlan header. This is a DOD-001 traceability failure.
- The EP-009 `CLOSED_BLOCKED` reason was `NOT_RUNNABLE_ENV(... Linux sandbox ...)` with `unblock_condition: "run on windows"`. The run is now **on Windows 10 x64** with the pinned toolchain working, so the stated blocker is dissolved — and it was concealing a genuine build defect that this session reproduced and fixed (`RC2175`).

The honest reading: `DONE_VERIFIED` on EP-000..EP-008 reflects *implemented code*,
not *verified capability*. The distinction is exactly what DOD-026 forbids
laundering.

## What is genuinely true today

- `sh scripts/preflight.sh` → `preflight: ok` (exit 0).
- `cargo build --workspace` → success, exit 0 (after the icon repair).
- `cargo test --workspace --locked` → 25 suites, **40 passed, 0 failed, 0 ignored**, exit 0.
- 11 library crates + 1 Tauri desktop app compile from a clean checkout.
- Domain logic (state machines, claim graph, docket lifecycle) is real and tested.

## What is not true

- No installer artifact, no artifact digest, no SBOM, no signing.
- No clean-room Win10/11 validation, no UAT, no accessibility sign-off.
- No provider live-fire; provider adapters are simulated and unreachable.
- No release verdict; `RELEASE_GATE.json` is `NOT_EVALUATED`.

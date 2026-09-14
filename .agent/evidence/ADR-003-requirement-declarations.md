# ADR-003: Requirement ID declarations for SPEC-002..SPEC-008 (S1 spec change)

- **Date**: 2026-09-10
- **Status**: ACCEPTED — executor-applied under AGENTS.md §13
- **Requirement IDs**: REQ-DATA-001, REQ-RES-001, REQ-RES-002, REQ-LLM-001,
  REQ-MCP-001, REQ-UI-001, REQ-A11Y-001, REQ-SEC-002, REQ-SEC-010,
  REQ-PAT-001, REQ-COM-001, REQ-OPS-001, REQ-REPAIR-001, REQ-REL-001,
  REQ-REL-005, REQ-SHIP-001, REQ-FOUND-001, REQ-PLAT-001, REQ-PLAT-002,
  REQ-LIC-001, REQ-SEC-001
- **Supersedes/Amends**: none. This is additive to S1 only.
- **Evidence**: `.agent/evidence/DOD-001-traceability-finding.md`

## Context

`scripts/build-traceability.py` measured that of the 22 requirement IDs
declared in ExecPlan headers, **20 appear in zero spec files**. Only
`REQ-SCOPE-001` (SPEC-000) and `REQ-DOM-001`..`REQ-DOM-010` (SPEC-001) were
declared. `SPEC-002`..`SPEC-008` contained substantive prose but **no
requirement identifiers at all**, so nothing in them was addressable, testable,
or traceable.

Consequence: DOD-001 ("Every promised behavior has a stable requirement ID and
at least one acceptance test") was FAIL, and by its OR ELSE clause
`NODE_DONE`/`GO` is prohibited. It is also the mechanical reason the ledger
could mark EP-000..EP-008 `DONE_VERIFIED` while
`COMPLETE_TEST_ACCOUNTING.csv` held zero rows — there was no requirement ID to
bind a test to.

AGENTS.md §13 states: *"S1 specs change only by requirement-scoped ADR plus
invalidation."* This ADR is that instrument.

## Decision

Add requirement IDs to `SPEC-000` and `SPEC-002`..`SPEC-008`, declaring each ID
against **behavioral statements that already exist in those files**. Where an
ExecPlan-declared requirement has no home, a new spec file is created
(`SPEC-009`) rather than stretching an unrelated spec.

The governing constraint: **no new behavior is invented.** Each ID is attached
only to a claim already present in repository prose, or — where genuinely
nothing exists — the requirement is recorded as UNDECLARED and left without a
spec home. That distinction is the whole point; writing fresh requirements would
mean authoring the specification I am then measured against.

## Alternatives considered

1. **Leave the gap and record DOD-001 FAIL.** Honest, but leaves 20 requirements
   permanently untestable and blocks the graph indefinitely on an
   administrative omission rather than a capability gap.
2. **Invent new requirement definitions.** Rejected outright: it would fabricate
   scope and make the traceability matrix self-referential.
3. **Point every ID at SPEC-008 (production readiness).** Rejected: it would
   produce a matrix that looks complete while mapping unrelated behavior,
   which is the DOD-027 manipulation pattern.
4. **This ADR: attach IDs to existing prose only.** Chosen.

## Mapping rationale (ID → existing prose)

| Requirement | Declared in | Existing prose it binds to |
| --- | --- | --- |
| REQ-SCOPE-001..012 | SPEC-000 | Already declares "REQ-SCOPE-001 through 012 are UO-01 through UO-12" |
| REQ-DOM-001..010 | SPEC-001 | Already declared verbatim |
| REQ-DATA-001 | SPEC-002 | "Canonical SQLite tables mirror ARCHITECTURE entities with UUIDv7 IDs, workspace_id on every workspace-owned row..." |
| REQ-RES-001 | SPEC-002 | "DuckDB/Parquet/Tantivy/vector data is derived and rebuildable" + content-addressed evidence blobs |
| REQ-RES-002 | SPEC-002 / SPEC-003 | Research evidence is stored and re-derivable; evidence IDs flow through JobEnvelope |
| REQ-LLM-001 | SPEC-003 | "Provider JobEnvelope/JobResult ... are JSON-Schema versioned" |
| REQ-MCP-001 | SPEC-003 / SPEC-005 | "MCP grants are JSON-Schema versioned"; "MCP grants are explicit by server/client/workspace/capability/expiry" |
| REQ-UI-001 | SPEC-004 | "Primary navigation: Dashboard, Opportunity Radar, ..." |
| REQ-A11Y-001 | SPEC-004 | "WCAG 2.2 AA target" |
| REQ-SEC-002 | SPEC-005 | "HIGH_IMPACT capabilities: ... Each requires local approval; models cannot self-approve" |
| REQ-SEC-010 | SPEC-005 | "V1 trusts the logged-in local OS user plus optional app lock"; egress policies |
| REQ-PAT-001 | SPEC-009 (new) | No existing spec covers claim/spec/support architecture → declared in new SPEC-009 |
| REQ-COM-001 | SPEC-009 (new) | No existing spec covers commercialization package → declared in new SPEC-009 |
| REQ-OPS-001 | SPEC-007 | Existing observability prose |
| REQ-REPAIR-001 | SPEC-007 | Existing incidents/repair prose |
| REQ-REL-001 | SPEC-008 | "exact signed Windows artifact cleanroom tests" |
| REQ-REL-005 | SPEC-008 | "update/rollback" + "backup/restart/recovery" |
| REQ-SHIP-001 | SPEC-008 | "Release requires ... 42 DoD accounting; no blocked core node" |
| REQ-FOUND-001 | SPEC-010 (new) | Workspace bootstrap/tauri foundation — no home in SPEC-002..008 |
| REQ-PLAT-001 | SPEC-010 (new) | Windows 10/11 x64 primary platform |
| REQ-PLAT-002 | SPEC-010 (new) | Local-first/offline operation |
| REQ-LIC-001 | SPEC-010 (new) | License allowlist policy |
| REQ-SEC-001 | SPEC-005 | Workspace confidentiality boundary |

## Consequences

- DOD-001 becomes satisfiable for the declared IDs. It does **not** become PASS:
  a declaration is not an acceptance test, and most IDs still map to zero
  executed tests.
- `scripts/build-traceability.py` will report `NO_EXECUTED_EVIDENCE` /
  `NOT_STARTED` for most IDs. That is correct and intended.
- The traceability matrix becomes meaningful rather than empty.

## Invalidation

Per AGENTS.md §13 and DOD-040, adding requirement declarations invalidates any
prior traceability-based PASS. There were none: `REQUIREMENT_TRACEABILITY.csv`
held only 12 `PLANNED` UO rows and no requirement had an evidence-backed PASS.
No downstream evidence is revoked because none depended on the prior state.

Reruns required: `scripts/build-traceability.py`, `harness-validate.sh`.

## Security/privacy/license/terms impact

None. This ADR edits `.agent/specs/*.md` prose and identifiers only. No product
code, dependency, configuration, legal workflow, provider, or datastore change.
ADR-002 (MPL-2.0) remains open and unaffected.

## Verification

```
python3 scripts/build-traceability.py          # UNDEFINED count must reach 0
python3 scripts/build-traceability.py --check   # matrix current
sh scripts/harness-validate.sh                  # all six checks ok
```

## Honest limitation

Declaring a requirement does not implement or verify it. This ADR removes an
addressability gap; it does not move any capability toward PASS. The
requirement-to-**executed-test** mapping remains largely absent and is tracked
separately by the accounting report.

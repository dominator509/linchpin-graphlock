# DOD-001 Traceability Finding — 20 requirements have no spec definition

Measured 2026-09-10 at candidate `ddd1da6` by `scripts/build-traceability.py`.

## Finding

`AGENTS.md` §15 DOD-001 states: *"Every promised behavior has a stable
requirement ID and at least one acceptance test before implementation is
declared complete"*, with required evidence *"Requirement-to-test traceability
row plus executed acceptance evidence"*, or else *"The item is INCOMPLETE and
NODE_DONE/GO is prohibited until the mapping exists and passes."*

Measured state:

- All 10 ExecPlans (EP-000..EP-010) declare requirement IDs in their headers,
  22 distinct IDs in total.
- **Only 2 of those are declared in any spec.** `SPEC-000` declares
  `REQ-SCOPE-001`; `SPEC-001` declares `REQ-DOM-001` through `REQ-DOM-010`.
- **20 requirement IDs appear in zero spec files.** They are referenced by
  ExecPlans but defined nowhere.

### The 20 undefined requirements

| Owning node | Undefined requirement IDs |
| --- | --- |
| EP-000 | `REQ-PLAT-001`, `REQ-PLAT-002`, `REQ-LIC-001` |
| EP-001 | `REQ-FOUND-001`, `REQ-SEC-001` |
| EP-002 | *(REQ-DOM-001/010 — defined)* |
| EP-003 | `REQ-DATA-001`, `REQ-RES-001` |
| EP-004 | `REQ-LLM-001`, `REQ-MCP-001`, `REQ-RES-002` |
| EP-005 | `REQ-UI-001`, `REQ-A11Y-001` |
| EP-006 | `REQ-SEC-002`, `REQ-SEC-010` |
| EP-007 | `REQ-PAT-001`, `REQ-COM-001` |
| EP-008 | `REQ-OPS-001`, `REQ-REPAIR-001` |
| EP-009 | `REQ-REL-001`, `REQ-REL-005` |
| EP-010 | `REQ-SHIP-001` |

Verification command used:

```
$ grep -l "REQ-DATA-001\|REQ-UI-001\|REQ-SEC-001" .agent/specs/*.md
(no output)
```

Per-spec requirement coverage:

| Spec | Bytes | Requirement IDs declared |
| --- | --- | --- |
| SPEC-000-product-scope.md | 345 | `REQ-SCOPE-001` |
| SPEC-001-core-domain.md | 703 | `REQ-DOM-001`..`010` |
| SPEC-002-data-model.md | 501 | **none** |
| SPEC-003-api-contracts.md | 528 | **none** |
| SPEC-004-ui-ux-behavior.md | 567 | **none** |
| SPEC-005-auth-and-permissions.md | 474 | **none** |
| SPEC-006-error-handling.md | 449 | **none** |
| SPEC-007-observability.md | 428 | **none** |
| SPEC-008-production-readiness.md | 491 | **none** |

The specs contain real prose (e.g. SPEC-002 describes SQLite tables, UUIDv7,
content-addressed evidence) but carry **no requirement identifiers**, so nothing
in them is addressable, testable, or traceable.

## Why this matters

This is the mechanical reason the ledger could mark EP-000..EP-008
`DONE_VERIFIED` while `COMPLETE_TEST_ACCOUNTING.csv` held **zero** rows: there
was no requirement ID to map a test to. A node cannot have "requirement/test/
evidence rows current" (AGENTS.md §4) when its requirements have no definition
and no test binding. The closures were therefore not evidence-backed under the
pack's own law.

It also explains why `REQ-REL-001` and `REQ-REL-005` — the requirements EP-009
claims to own — cannot be satisfied: there is no statement of what they require,
so no acceptance test can be written against them.

## Disposition

`REQ-*` work is **INCOMPLETE** for 20 of 22 requirements, per DOD-001's OR ELSE
clause: "NODE_DONE/GO is prohibited until the mapping exists and passes."

This is recorded, not silently reconciled. Fixing it requires adding requirement
declarations to `SPEC-002`..`SPEC-008` and `SPEC-000`, which is an **S1 spec
change**: per AGENTS.md §13, S1 specs "change only by requirement-scoped ADR
plus invalidation", so it cannot be done as an incidental edit inside a node.

## Relationship to the DoD gate

`scripts/dod-gate.sh` reports "42 clauses present; execution status must be
proven by candidate evidence" — it validates the *registry*, not the mapping.
`DOD_STATUS.jsonl` is empty, so no DoD clause currently has an evidence-backed
disposition either. DOD-001 is therefore FAIL, and by its own OR ELSE clause GO
is prohibited until a requirement-to-test mapping exists and passes.

## Tooling added

`scripts/build-traceability.py` derives the chain
`requirement -> owning node -> spec -> implementation paths -> mapped registry
tests -> executed status` from repository evidence only. It emits
`NO_SPEC_DEFINITION` / `NO_IMPLEMENTATION_PATH` / `NO_MAPPED_TEST` /
`NOT_STARTED` / `PARTIAL` / `PASS` and prints the undefined list rather than
inventing mappings. `--check` verifies the matrix is current.

Output: `.agent/verification/REQUIREMENT_TRACEABILITY.csv` (22 rows).

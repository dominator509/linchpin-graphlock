# EP-007 - Patent Workflows, Commercialization and E2E

Requirements: REQ-PAT-001, REQ-COM-001. Dependencies: EP-005,EP-006.

## Scope
This node owns only the paths/components necessary for patent workflows, commercialization and e2e. It may not weaken prior specs/tests or introduce unapproved external services. All external names/versions are re-verified from repo/first-party evidence before transcription.

## Milestones

### M1 - Claim/spec/figure lint and deterministic DOCX/PDF package builder
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-007/M1/`. Do not use mocks as the final dependency proof.

### M2 - USPTO ruleset/form manifest + human Patent Center handoff/receipt import
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-007/M2/`. Do not use mocks as the final dependency proof.

### M3 - Office Action/rejection/cited-art response workspace
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-007/M3/`. Do not use mocks as the final dependency proof.

### M4 - Commercialization targets/scenario valuation/data room/non-confidential assets
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-007/M4/`. Do not use mocks as the final dependency proof.

### M5 - Run UO-01..12 live-fire, mutation proofs and domain regression
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-007/M5/`. Do not use mocks as the final dependency proof.

## Closure
Run owned tests plus affected regressions; update traceability and dependency proofs; run anti-gaming scan/review; run architecture drift gate; account applicable DoD clauses; append hash-ledger event. DONE_VERIFIED is forbidden while evidence is missing or a core dependency remains blocked.

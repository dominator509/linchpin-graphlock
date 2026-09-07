# EP-010 - Production Readiness and Ship

Requirements: REQ-SHIP-001. Dependencies: EP-009.

## Scope
This node owns only the paths/components necessary for production readiness and ship. It may not weaken prior specs/tests or introduce unapproved external services. All external names/versions are re-verified from repo/first-party evidence before transcription.

## Milestones

### M1 - Run V-000..V-021 without product-code changes
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-010/M1/`. Do not use mocks as the final dependency proof.

### M2 - Account all 484 registry IDs and 42 DoD clauses
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-010/M2/`. Do not use mocks as the final dependency proof.

### M3 - Architecture/claim/spec-to-code drift reconciliation
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-010/M3/`. Do not use mocks as the final dependency proof.

### M4 - Human UAT/accessibility/external gates and residual risks
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-010/M4/`. Do not use mocks as the final dependency proof.

### M5 - Emit GO/CONDITIONAL_EXTERNAL_GATES/NO_GO and manual-release instructions
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-010/M5/`. Do not use mocks as the final dependency proof.

## Closure
Run owned tests plus affected regressions; update traceability and dependency proofs; run anti-gaming scan/review; run architecture drift gate; account applicable DoD clauses; append hash-ledger event. DONE_VERIFIED is forbidden while evidence is missing or a core dependency remains blocked.

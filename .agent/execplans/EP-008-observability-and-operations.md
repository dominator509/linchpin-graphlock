# EP-008 - Observability and Repair

Requirements: REQ-OPS-001, REQ-REPAIR-001. Dependencies: EP-007.

## Scope
This node owns only the paths/components necessary for observability and repair. It may not weaken prior specs/tests or introduce unapproved external services. All external names/versions are re-verified from repo/first-party evidence before transcription.

## Milestones

### M1 - OpenTelemetry local pipeline and correlation
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-008/M1/`. Do not use mocks as the final dependency proof.

### M2 - Windows panic/minidump + Incident model
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-008/M2/`. Do not use mocks as the final dependency proof.

### M3 - Sanitized Repair Capsule + agent repair brief
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-008/M3/`. Do not use mocks as the final dependency proof.

### M4 - Approved Git issue/branch/PR integration real sandbox proof
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-008/M4/`. Do not use mocks as the final dependency proof.

### M5 - Backup/recovery/soak/reconciliation and resource-leak tests
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-008/M5/`. Do not use mocks as the final dependency proof.

## Closure
Run owned tests plus affected regressions; update traceability and dependency proofs; run anti-gaming scan/review; run architecture drift gate; account applicable DoD clauses; append hash-ledger event. DONE_VERIFIED is forbidden while evidence is missing or a core dependency remains blocked.

# EP-006 - Security and Permissions

Requirements: REQ-SEC-002, REQ-SEC-010. Dependencies: EP-004.

## Scope
This node owns only the paths/components necessary for security and permissions. It may not weaken prior specs/tests or introduce unapproved external services. All external names/versions are re-verified from repo/first-party evidence before transcription.

## Milestones

### M1 - Disclosure Firewall + public-export gate
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-006/M1/`. Do not use mocks as the final dependency proof.

### M2 - MCP/provider egress and HIGH_IMPACT approval policies
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-006/M2/`. Do not use mocks as the final dependency proof.

### M3 - Parser/archive/path/input hardening + fuzz targets
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-006/M3/`. Do not use mocks as the final dependency proof.

### M4 - Crash/log redaction and secret canary tests
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-006/M4/`. Do not use mocks as the final dependency proof.

### M5 - Dependency/SBOM/signature/update threat controls
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-006/M5/`. Do not use mocks as the final dependency proof.

## Closure
Run owned tests plus affected regressions; update traceability and dependency proofs; run anti-gaming scan/review; run architecture drift gate; account applicable DoD clauses; append hash-ledger event. DONE_VERIFIED is forbidden while evidence is missing or a core dependency remains blocked.

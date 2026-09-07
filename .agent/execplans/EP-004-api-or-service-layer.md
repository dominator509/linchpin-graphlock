# EP-004 - Provider, Research and MCP Service Layer

Requirements: REQ-LLM-001, REQ-MCP-001, REQ-RES-002. Dependencies: EP-003.

## Scope
This node owns only the paths/components necessary for provider, research and mcp service layer. It may not weaken prior specs/tests or introduce unapproved external services. All external names/versions are re-verified from repo/first-party evidence before transcription.

## Milestones

### M1 - Implement ProviderTransport contract + local model real proof
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-004/M1/`. Do not use mocks as the final dependency proof.

### M2 - Implement official OpenAI/xAI and approved/terms-gated adapters without credential harvesting
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-004/M2/`. Do not use mocks as the final dependency proof.

### M3 - Research planner, kill-search, citation/provenance fusion and checkpointing
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-004/M3/`. Do not use mocks as the final dependency proof.

### M4 - Design-Around Tournament orchestrator with independent runs
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-004/M4/`. Do not use mocks as the final dependency proof.

### M5 - MCP client/server + capability grants + prompt-injection boundary
Acceptance oracle: a requirement-scoped test/evidence target proves the real behavior and fails when the behavior is removed or policy is violated.
Order: add/confirm stable test ID -> demonstrate red when feasible -> implement/transcribe production change -> run narrow green -> run negative/mutation -> run milestone verification -> write evidence -> commit.
Evidence: `.agent/evidence/EP-004/M5/`. Do not use mocks as the final dependency proof.

## Closure
Run owned tests plus affected regressions; update traceability and dependency proofs; run anti-gaming scan/review; run architecture drift gate; account applicable DoD clauses; append hash-ledger event. DONE_VERIFIED is forbidden while evidence is missing or a core dependency remains blocked.

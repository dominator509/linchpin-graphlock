# SPEC-007 Observability

REQ-OPS-010 Structured event schema carries UTC time, level, correlation, workspace pseudonymous ID/hash, component, operation, duration, result and evidence IDs.

REQ-OPS-011 No document bodies, secrets or provider tokens appear in telemetry. Incident captures are local.

REQ-REPAIR-001 Crash capsule export applies deterministic redaction rules and logs redaction counts/hashes.

REQ-REPAIR-002 Release tests plant canary secret/invention strings and prove absence after export.

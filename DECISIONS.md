# Decisions / ADR Index

| ADR | Decision | Status | Reason |
| --- | --- | --- | --- |
| ADR-001 | Tauri v2 + React/TypeScript desktop | accepted | small native surface, Rust boundary, commercial-friendly license |
| ADR-002 | Rust canonical core; Python optional sidecar only | accepted | strong local reliability; ML ecosystem remains optional |
| ADR-003 | SQLCipher SQLite canonical + DuckDB/Parquet/Tantivy derived | accepted | local confidentiality + simple backup + rebuildable analytics/search |
| ADR-004 | first-party executable/protocol owns OAuth | accepted | minimize cost without credential scraping or undocumented consumer endpoints |
| ADR-005 | local model is mandatory release capability | accepted | privacy/offline/no recurring API dependence |
| ADR-006 | human Patent Center submission boundary | accepted | no assumed general filing API; signatures/legal/payment stay human-controlled |
| ADR-007 | five truth boundaries are domain invariants | accepted | prevents deceptive product/legal claims |
| ADR-008 | LINCHPIN vector + uncertainty, no patentability percentage | accepted | patentability/market/FTO cannot be collapsed honestly |
| ADR-009 | MCP client + server with high-impact capability gate | accepted | interoperable but fail-closed agent connectivity |
| ADR-010 | permissive-license-first; PQAI optional/benchmark-gated | accepted | commercial redistribution and domain ownership |
| ADR-011 | local-only telemetry default + Repair Capsule | accepted | useful repair evidence without invention exfiltration |
| ADR-012 | Gemini OAuth integration remains disabled until terms gate passes | accepted | current terms uncertainty must fail closed |

New decisions include date, requirement IDs, alternatives, evidence, consequences, invalidated nodes/tests and rollback. Use `.agent/templates/adr-template.md`.

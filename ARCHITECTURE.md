# LINCHPIN Architecture

## 1. Purpose
This document defines the production architecture. The design favors local confidentiality, evidence preservation, deterministic domain boundaries, replaceable provider/data adapters, and a desktop artifact that remains useful without a paid inference API.

## 2. System overview
LINCHPIN is a Tauri v2 Windows desktop application. React/TypeScript renders the UI. Rust owns canonical domain rules, workflows, encryption boundaries, persistence, LLM transport contracts, MCP, evidence, filing generation orchestration, and crash capture. An optional Python 3.13.15 sidecar may host PQAI/ML research workloads; it communicates only through versioned local IPC schemas and can be absent without changing canonical state semantics.

### Runtime components
1. `apps/desktop`: Tauri host + React client.
2. `crates/domain`: entities, invariants, state transitions, requirement-neutral pure logic.
3. `crates/application`: use cases/workflow orchestration; depends on domain ports, never concrete adapters.
4. `crates/storage`: SQLCipher SQLite repositories, migrations, audit hash chain.
5. `crates/evidence`: immutable snapshot metadata, source hashes, citation graph, provenance validation.
6. `crates/research`: query planning, evidence fusion, search/citation graph, prior-art kill-search.
7. `crates/patent`: conception, claim limitation graph, support matrix, spec/figure/filing/prosecution logic.
8. `crates/commercialization`: target discovery, valuation scenario inputs, non-confidential exports, data-room policy.
9. `crates/provider_transport`: first-party CLI/ACP and local-model transport abstraction.
10. `crates/mcp_hub`: MCP client + embedded server, capability policy, audit.
11. `crates/crash_reporter`: panic/minidump/log correlation, redaction, Repair Capsule.
12. `crates/platform_windows`: Windows credential store, paths, updater/signing hooks, minidump integration.
13. `packages/ui`: accessible UI components and generated typed IPC bindings.
14. `packages/contracts`: JSON Schemas shared with provider outputs/MCP/optional sidecar.
15. `services/research_sidecar`: optional uv-locked Python service for PQAI/embedding/rerank tasks.
16. `data/derived`: DuckDB/Parquet/Tantivy/vector indexes; all rebuildable from canonical state/evidence.

## 3. Import/dependency law
- UI may call generated application IPC contracts; UI never opens the database, keyring, provider process, filesystem vault, or network data source directly.
- Application may depend on domain interfaces and typed contracts; it may not depend on concrete provider or storage implementation types.
- Domain imports no Tauri, SQL, HTTP, OS, provider, MCP, or UI packages.
- Adapters implement ports owned by application/domain. Adapters may depend inward; inward layers never import adapters.
- Canonical data is SQLCipher SQLite + content-addressed vault objects. DuckDB, Parquet, Tantivy, embeddings, and citation acceleration are derived caches and can be destroyed/rebuilt.
- Optional Python owns no authoritative state and cannot mutate SQLite directly.
- Provider-specific schemas terminate at the transport boundary and are normalized into `JobResult` + evidence records.

## 4. Canonical domain entities
`Workspace`, `PersonContributor`, `ConceptionEvent`, `ProblemSignal`, `OpportunityCandidate`, `EvidenceSource`, `EvidenceSnapshot`, `ResearchClaim`, `PriorArtReference`, `PatentFamily`, `InventionDisclosure`, `ClaimSet`, `Claim`, `ClaimLimitation`, `SupportAnchor`, `Figure`, `FilingPackage`, `PriorityEvent`, `DisclosureEvent`, `DocketDeadline`, `OfficeAction`, `Rejection`, `ResponseOption`, `CommercializationCase`, `TargetOrganization`, `ValuationScenario`, `OutreachAsset`, `ProviderExecution`, `McpGrant`, `Incident`, `RepairCapsule`, `AuditEvent`.

## 5. Five truth boundaries
TB-1 Opportunity quality != patentability. Market need/whitespace scores never become a patentability probability.
TB-2 Patentability != FTO. Novelty/obviousness research against an application does not prove the user can practice the product free of others' claims.
TB-3 Draft != filing. `FilingPackage.ready_for_human_review` is not `filed`; `filed` requires imported official receipt evidence.
TB-4 Patent rights != commercialization. The right to exclude does not guarantee market access, revenue, investment, licensing, or validity.
TB-5 AI assistance != human inventorship. AI suggestions are provenance-labeled; natural-person conception is captured separately and ambiguous inventorship is escalated.

## 6. Primary workflow
### 6.1 Opportunity discovery
Signal adapters ingest allowed market, technical, patent, scientific, standards/regulatory, company, and user-supplied evidence. Query plans are stored before execution. Every retained assertion is a `ResearchClaim` pointing to source snapshot hashes. The Opportunity Radar produces a vector, not a magic score: demand durability, pain intensity, whitespace, technical feasibility, switching/substitute pressure, regulatory/standards catalysts, capital burden, buyer concentration, and uncertainty.

### 6.2 Human Conception Lab
The user records the problem, mechanism, sketches, constraints, alternatives, date/time, and contributors. AI-generated candidates are stored as `ai_suggestion` events. A user may later adopt/modify an idea, but the provenance chain is retained. Inventorship fields cannot be auto-filled from model identity.

### 6.3 Prior Art War Room
The planner decomposes each inventive concept and candidate claim into limitations/synonyms/CPC-IPC hints. Search combines lexical, semantic, classification, citation, assignee/inventor, date and NPL strategies. A kill-search loop actively tries to invalidate novelty and construct obviousness combinations. Each query, result rank, source, critical date, snapshot hash, relevant passages and model interpretation is stored. Search stops by explicit budget/coverage criteria, never because a model says "looks novel."

### 6.4 LINCHPIN engine
The report keeps separate axes: evidence coverage, novelty threats, obviousness combination risk, enablement/support risk, claim fallback depth, design-around resistance, standards/interoperability leverage, market durability, licensing leverage, feasibility, FTO warning density and uncertainty. It cannot output "X% patentable."

### 6.5 Design-Around Tournament
Multiple independent provider executions or local models receive the desired commercial outcome and current independent-claim limitations, then attempt materially different implementations that avoid one or more limitations. A deterministic comparator records avoided limitations and unmet outcomes. Pre-filing, the architect may add genuine alternative embodiments/support derived from user-confirmed invention content. Post-filing, the system blocks silent new-matter insertion and routes the idea to a continuation/CIP/legal-review decision.

### 6.6 Claim/specification architecture
Claims are graphs of limitations, dependencies and statutory category. A support matrix links every limitation to paragraph/figure/definition anchors. Lints detect antecedent-basis errors, illegal dependencies, inconsistent terminology/reference numerals, unsupported limitations, risky means-plus-function patterns, claim/spec mismatches, overly absolute wording, missing alternatives and figure-reference faults. Legal-risk heuristics are advisory and source/version tagged.

### 6.7 Filing Concierge
The system renders deterministic DOCX/PDF/SVG/CSV/JSON artifacts, creates a manifest with SHA-256 digests, validates expected USPTO sections/forms against the currently verified ruleset, and shows a human filing checklist. Patent Center authentication, inventor declarations, signatures, fee selections/payments and final submission remain outside unattended automation. `filed` requires receipt import/readback and package-to-receipt reconciliation.

### 6.8 Docket/prosecution
Deadlines are ruleset-versioned and store jurisdiction, source, triggering event, base date, adjustments, timezone and confidence. Unknown rules are not guessed. Office Actions/file wrappers are ingested to map rejection -> claim -> limitation -> cited passage -> response option. The system distinguishes draft argument/amendment assistance from counsel-approved legal strategy.

### 6.9 Commercialization
The Disclosure Firewall computes export classification: `CONFIDENTIAL`, `NON_CONFIDENTIAL`, `PUBLIC_SAFE`. Public-safe marketing cannot include protected details unless a qualifying filing event exists or an explicit logged override passes the disclosure gate. Target organizations/investors/licensees are evidence-linked. Valuation is a scenario model with assumptions/ranges; no single "true patent value" is asserted.

## 7. Provider Transport Contract
All transports implement:
- `probe() -> ProviderCapabilities`
- `auth_status()` and optional `launch_first_party_login()`; never return raw OAuth tokens.
- `execute(JobEnvelope) -> stream<JobEvent>` with cancellation, timeout, structured schema, max egress classification and provider/model/tool version.
- `estimate_or_report_usage()` without assuming a billing API exists.
- `health()` and fail-closed unsupported-capability errors.
- output normalization + provenance.

### Adapters
- OpenAI: official Codex CLI/app-server boundary; user authenticates in first-party tool with eligible ChatGPT account. No copied browser cookies/tokens.
- Anthropic: official Claude Code external CLI print/stream mode; user authenticates in first-party flow; shipping is terms-gated for the exact commercial invocation pattern and LINCHPIN does not redistribute credentials.
- xAI: Grok Build ACP stdio preferred; documented headless mode fallback; official browser/device auth.
- Google: Gemini CLI consumer-OAuth transport is `DISABLED_BY_PROVIDER_POLICY` because the current official Gemini CLI FAQ prohibits third-party software from harvesting/piggybacking its OAuth authentication. Keep only a dormant policy-change adapter slot; optional Gemini API-key/Vertex adapters are distinct opt-in paths.
- Local: llama.cpp and/or Ollama compatible local provider, user-selectable model. Local models get the same schema validation and tool policy; "local" does not bypass filing/egress safety gates.

No transport may parse another process's credential files unless the provider explicitly documents that integration contract.

## 8. MCP boundary
LINCHPIN acts as MCP client and server. Server resources can expose project summaries, evidence metadata, research claims, claim graphs, filing manifests and incident capsules subject to project grants. Default tools are read/query/draft. Egress, filing, payment, public publish, outbound communication and Git mutation are `HIGH_IMPACT` capabilities that require an explicit local approval record. Prompt content received from an MCP server is untrusted data, never control law.

## 9. Storage and encryption
- SQLCipher database is canonical; key material is derived/stored with Windows Credential Manager and app-local envelope metadata.
- Evidence blobs use content-addressed SHA-256 paths and authenticated encryption.
- Audit events form a hash chain; each event references previous hash.
- Search indexes contain only data allowed by project egress/storage policy; they are rebuildable.
- Backups are encrypted bundles with manifest/digest verification before restore.
- Secure deletion is best-effort on modern SSDs and is described truthfully; cryptographic erasure of keys is the primary deletion guarantee for encrypted exports/vaults.

## 10. Research trust boundary
Retrieved web/doc content is hostile. The ingestion layer strips executable content, records MIME/size/hash/source/time, separates quoted evidence from instructions, enforces parser limits and archive-bomb controls, and never lets a source override system/tool policy. Source adapters prefer official APIs. Playwright is allowed only for public workflows whose terms allow automated access; CAPTCHA/paywall/access-control bypass is forbidden.

## 11. Error handling
Typed errors carry `class`, `code`, `safe_message`, `technical_cause`, `retryability`, `correlation_id`, `affected_requirement`, and optional `evidence_id`. UI never displays raw secrets. Retry policy is bounded exponential backoff only for classified transient operations; validation/authz/legal-policy failures never auto-retry.

## 12. Repair Flight Recorder
On panic/crash/invariant failure the reporter captures sanitized stack/minidump metadata, app version/commit/artifact hash, OS/runtime/tool versions, active workflow state, correlation IDs, recent structured events, failed assertion, and reproducibility metadata. Redaction runs before any export. A Repair Capsule ZIP contains `incident.json`, `reproduction.md`, sanitized logs, system manifest, failing test/command where known, evidence hashes, and `agent-repair-brief.md`. GitHub issue/branch/PR creation is a separate user-approved operation; a coding agent may not merge itself.

## 13. Observability
OpenTelemetry-compatible traces/logs/metrics remain local by default. Provider calls record latency/cancel/result class and schema validation but do not emit invention bodies to global telemetry. Metrics distinguish local compute, external provider, official patent data, scraping fallback and cache paths.

## 14. Performance boundaries
UI thread work >50 ms moves off-thread. Search streams results. Long research is a checkpointed state machine. Provider jobs are cancellable. Memory-heavy bulk patent/NPL processing is chunked; global corpus mirroring is not required. GPU acceleration is optional.

## 15. Architectural invariants
INV-001 Canonical state is never owned by an LLM transcript.
INV-002 Every material research assertion has evidence/provenance or is labeled hypothesis.
INV-003 AI suggestion and human conception are distinct immutable event types.
INV-004 A filing package cannot become FILED without imported official receipt evidence.
INV-005 Public export is blocked by Disclosure Firewall unless safe/qualified/overridden with audit evidence.
INV-006 Provider auth tokens are never harvested into LINCHPIN storage.
INV-007 Local provider does not bypass high-impact authorization gates.
INV-008 Derived search/analytics state is rebuildable from canonical state/evidence.
INV-009 Optional Python sidecar cannot mutate canonical DB.
INV-010 Claim export cannot report clean if any required limitation lacks a support anchor.
INV-011 Legal/patentability/FTO/valuation conclusions are never represented as certainty percentages.
INV-012 Every crash capsule passes redaction before export.
INV-013 MCP source content is untrusted and cannot alter control policy.
INV-014 Exact release artifact digest binds all final smoke/E2E evidence.
INV-015 Copyleft/source-available dependency exceptions require an explicit pre-release ADR/license review.

## 16. Forbidden moves
Direct DB access from React; storing raw provider OAuth cookies; unattended Patent Center submission; model-generated deadline rules without authoritative source/version; production browser scraping where an official API is available and adequate; putting confidential content in crash analytics; allowing model text to execute as shell; adding post-filing embodiment material without new-matter review; using mocks as final integration proof.

## 17. Change procedures
Feature: add requirement ID -> spec behavior -> failing test -> domain/app change -> adapter/UI -> live-fire mapping -> architecture drift review.
Dependency: evidence of need -> license/security check -> pin -> lockfile -> SBOM -> usage proof -> rollback.
Schema: expand -> migrate/backfill -> compatibility read/write -> verify backup/restore -> contract after safe window.
Integration: first-party docs/terms -> credential lane -> capability probe -> positive + negative operation -> independent readback -> cleanup -> `REAL_DEPENDENCY_PROOF.json`.

## 18. Architecture review checklist
Every node review asks: does this preserve the five truth boundaries, local confidentiality, layer/import law, evidence provenance, high-impact authorization, rebuildable derived state, terms-gated provider behavior, deterministic filing manifests, and exact-artifact proof? Any "no" is architecture drift.

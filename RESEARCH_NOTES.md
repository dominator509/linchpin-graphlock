# LINCHPIN Deep Research Notes

Research cut: 2026-08-28. This file records architecture-driving findings used to fill the GraphLock inputs. It is not legal advice and source terms can change; PREFLIGHT must re-verify material provider/data-source terms before implementation or release.

## 1. Patent filing and legal workflow constraints

- USPTO Patent Center is the official electronic filing/management environment. Registered/identity-verified account controls and current Patent Center submission requirements make an unattended third-party "filing API" an unsafe architectural assumption. LINCHPIN therefore generates, validates, hashes, and packages application artifacts, then hands the user into a human review/sign/submit workflow.
  - https://www.uspto.gov/patents/apply/patent-center
- USPTO supports DOCX filing for utility nonprovisional specification/claims/abstract and Patent Center validates DOCX; non-DOCX submissions can incur a surcharge. The product therefore needs deterministic DOCX generation, preflight linting, a rendered comparison/preview, optional auxiliary PDF support, and receipt/hash reconciliation.
  - https://www.uspto.gov/patents/docx
- Current USPTO forms include the Patent Center auto-load provisional cover sheet SB/16 and many current filing/prosecution forms. Forms must be treated as versioned external artifacts, not hard-coded forever.
  - https://www.uspto.gov/patents/apply/forms
- USPTO Open Data Portal patent-file-wrapper APIs are retrieval/data services, not a general substitute for Patent Center filing. ODP access/key requirements and schemas must be capability-probed and cached.
  - https://data.uspto.gov/apis/getting-started
- A patent grants a right to exclude; it does not itself grant a positive right to practice an invention. Utility-patent term is generally measured to 20 years from the applicable filing date and maintenance fees are required. LINCHPIN must not describe patents as permanent monopolies.
  - https://www.uspto.gov/ip-policy/patent-policy/patents
  - https://www.uspto.gov/patents/maintain
- AI-assisted invention does not create a new inventorship standard; USPTO's revised November 2025 guidance says only natural persons can be inventors and conception remains central. LINCHPIN therefore needs a Human Conception Ledger, contributor provenance, and an inventorship-risk gate.
  - https://www.uspto.gov/subscription-center/2025/revised-inventorship-guidance-ai-assisted-inventions
- Early public disclosure can jeopardize rights, especially outside the United States. LINCHPIN therefore needs a Disclosure Firewall tied to filing/priority events before public marketing or investor-outreach exports.
  - https://www.uspto.gov/patents/basics/international-protection
- USPTO provides pro-se and inventor-assistance resources. The product should route high-stakes questions to registered counsel or official assistance resources instead of masquerading as counsel.
  - https://www.uspto.gov/patents/basics/using-legal-services/pro-se-assistance-program
  - https://www.uspto.gov/learning-and-resources/support-centers/inventors-assistance-center-iac

## 2. Patent/scientific data sources

- EPO Open Patent Services (OPS) is an official patent-data REST service using OAuth and quota controls. Build an adapter with local caching, rate limiting, backoff, and source snapshots.
  - https://www.epo.org/en/searching-for-patents/data/web-services/ops
- Crossref REST metadata is openly accessible with public/polite pools and rate/concurrency limits; useful for non-patent literature metadata.
  - https://www.crossref.org/documentation/retrieve-metadata/rest-api/access-and-authentication/
- SEC EDGAR exposes unauthenticated JSON APIs for company submissions/XBRL, useful as one public commercialization/market signal source.
  - https://www.sec.gov/search-filings/edgar-application-programming-interfaces
- PQAI is an MIT-licensed open-source prior-art search stack and is a candidate optional local semantic-prior-art subsystem, but only after transitive dependency/security/license audit and benchmark validation against LINCHPIN's evidence requirements.
  - https://github.com/pqaidevteam/pqai

## 3. Official LLM transporter findings

The transporter policy is "delegate authentication to the provider's official executable/protocol; do not scrape consumer sessions, extract tokens, or emulate undocumented endpoints." Each adapter is independently terms-gated and capability-probed at runtime.

- OpenAI Codex can be used with ChatGPT-plan authentication; Codex CLI supports non-interactive execution and structured output paths. Default integration is an external official Codex process or supported app-server boundary, not copied ChatGPT cookies.
  - https://help.openai.com/en/articles/11381614
  - https://developers.openai.com/codex/cli
- Anthropic Claude Code supports Claude account subscription authentication and non-interactive print/JSON modes. LINCHPIN calls the user's separately installed/authenticated first-party CLI as an external process. Commercial embedding/redistribution and any subscription-use restrictions remain a PREFLIGHT terms gate.
  - https://docs.anthropic.com/en/docs/claude-code/overview
  - https://docs.anthropic.com/en/docs/claude-code/iam
- xAI Grok Build provides browser/device authentication and explicitly documents headless execution and ACP stdio integration, making ACP the preferred Grok transporter when available.
  - https://docs.x.ai/docs/grok-build/overview
  - https://docs.x.ai/docs/grok-build/headless
  - https://docs.x.ai/docs/grok-build/acp
- Gemini CLI supports Google-account OAuth, but its current official FAQ explicitly says third-party software may not harvest or piggyback Gemini CLI OAuth to access Google's backend services and that doing so violates applicable terms/policies. Therefore Gemini consumer-OAuth transport is `DISABLED_BY_PROVIDER_POLICY` in LINCHPIN. The adapter slot may exist only for future policy change detection; today the supported Google route is a separately user-enabled Gemini API key or Vertex AI path.
  - https://github.com/google-gemini/gemini-cli/blob/main/docs/get-started/authentication.md
  - https://github.com/google-gemini/gemini-cli/blob/main/docs/faq.md
- Qwen Code's OAuth program was discontinued in 2026; it is not treated as a durable subscription transporter.
- Local/self-hosted model support is first-class through llama.cpp and/or Ollama-style local inference. Model selection is user-controlled. LINCHPIN does not claim a model is "uncensored"; it can support policy-neutral/local models with the same tool sandbox and evidence controls.

## 4. Competitive feature baseline

Feature research covered current patent drafting/prosecution products including Solve Intelligence, PatentBots, PatentPal, and Rowan/Clarivate. Baseline parity areas include prior-art search, drafting, claims/spec support, figures, proofreading, Office Action workflows, claim charts, FTO/infringement screens, portfolio analysis, terminology/reference checks, and Word/export workflows. LINCHPIN differentiates with: blue-ocean opportunity discovery before an invention exists; human-conception provenance; multi-model Design-Around Tournament; durable-demand/standards/chokepoint analysis; disclosure firewall; local-first evidence vault; official-subscription transporter hub; commercialization/licensing workspace; GraphLock-grade proof; and automatic crash-to-repair capsules.

References:
- https://www.solveintelligence.com/
- https://www.patentbots.com/
- https://patentpal.com/
- https://clarivate.com/intellectual-property/patent-intelligence/rowan-patents/

## 5. Open-source foundations

Recommended foundations/components are selected only when their licenses and dependencies survive the release license gate:
- Tauri -- MIT OR Apache-2.0 desktop shell: https://github.com/tauri-apps/tauri
- DuckDB -- MIT analytical engine: https://github.com/duckdb/duckdb
- Tantivy -- MIT full-text search: https://github.com/quickwit-oss/tantivy
- Playwright -- Apache-2.0 browser/test automation: https://github.com/microsoft/playwright
- PQAI -- MIT candidate prior-art subsystem: https://github.com/pqaidevteam/pqai

A generic AI-chat application is not selected as the product shell. The domain, evidence, legal-boundary, and claim-support model should be native LINCHPIN architecture rather than a skin around a chatbot.

## 6. MCP architecture

LINCHPIN is both an MCP client and an MCP server. The server exposes least-privilege project/research/evidence/drafting/incident resources and tools. Dangerous actions--external egress, public publication, filings, payments, outreach, Git writes--are not MCP-default capabilities and require explicit policy/HITL authorization. The implementation pins a current MCP SDK/spec version after PREFLIGHT rather than coding against an assumed "latest."

## 7. Commercialization, valuation, and ownership research

- USPTO Assignment Center is the current system for recording patent assignments and related ownership documents; public Assignment Search covers recorded patent assignment information and explicitly warns that recordation is ministerial rather than a USPTO determination of transaction validity. LINCHPIN therefore needs chain-of-title evidence, assignment-search imports, transaction-document checklists, and a human-controlled Assignment Center handoff rather than claiming that recordation validates ownership.
  - https://www.uspto.gov/patents/maintain/patents-assignments-change-search-ownership
  - https://assignmentcenter.uspto.gov/
- USPTO's Patent Assignment Dataset can support transaction/assignee research and includes assignments and other recorded documents affecting title. It is a research signal, not an automatic comparable-price database.
  - https://www.uspto.gov/ip-policy/economic-research/research-datasets/patent-assignment-dataset
- WIPO's current IP-valuation guidance treats valuation as context-dependent and identifies cost, market, and income approaches, with real-options and Monte Carlo methods useful for uncertainty-heavy technology transfer. LINCHPIN should therefore provide scenario ranges, explicit assumptions, sensitivity/Monte Carlo analysis, buyer-specific strategic fit, and evidence provenance -- never a fake single authoritative patent price.
  - https://www.wipo.int/en/web/business/ip-valuation
  - https://www.wipo.int/web-publications/intellectual-property-valuation-basics-for-technology-transfer-professionals/en/index.html
- Commercialization diligence must distinguish title/encumbrance evidence, legal status/remaining life, technical readiness, market adoption, buyer-specific synergies, development costs, substitutes/design-arounds, regulatory/standards constraints, and know-how/trade-secret complements. Outreach exports remain behind the Disclosure Firewall.

## 8. Toolchain research cut

Bootstrap versions verified from first-party release sources on 2026-08-28: Rust 1.98.0; Node.js 24.20.0 LTS; pnpm 11.21.0; React 19.2; Python 3.13.15; uv 0.12.0; Tauri core 2.11.5, Tauri CLI 2.11.4, @tauri-apps/api 2.11.1, @tauri-apps/cli 2.11.4. These are bootstrap pins, not permission to float: EP-000 rechecks current advisories/compatibility and records an ADR before any change.

## 9. Architecture decision summary

1. Local-first Windows desktop; future SaaS is optional.
2. Rust canonical domain core; optional Python research sidecar only behind typed contracts.
3. Encrypted local canonical store + rebuildable analytical/search indexes.
4. Evidence-first research: every material assertion points to snapshots/hashes/provenance.
5. Human conception and AI assistance are separate first-class records.
6. LINCHPIN score is a heuristic vector with explicit uncertainty; never a patentability probability.
7. Patent drafting uses a claim limitation graph and limitation-to-spec/figure support matrix.
8. Filing package generation is automated; legal filing/signature/payment remains human-controlled.
9. Disclosure Firewall prevents accidental pre-filing publicity.
10. Provider transporters delegate auth to first-party tools; no token harvesting.
11. Self-hosted local model is a first-class offline/privacy option.
12. Repair Flight Recorder generates sanitized, reproducible Repair Capsules for agentic repair and optional approved PR workflows.

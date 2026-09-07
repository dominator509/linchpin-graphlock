# Research Engine Specification

## Goal
Produce citation-complete, reproducible evidence sets for opportunity discovery and patent research. The engine never converts a relevance score into a legal conclusion.

## Source tiers
1. Primary patent sources: USPTO ODP / Patent Center public records where supported; EPO OPS; WIPO PATENTSCOPE links/exports where terms permit.
2. Scholarly/technical: Crossref plus source-native metadata, standards bodies and authoritative technical repositories.
3. Market evidence: SEC EDGAR and public company/industry sources with captured date, URL, terms and provenance.
4. Anecdotal/user-problem evidence: public forums and communities only through terms-compliant adapters; every excerpt stores source type and evidentiary weight.

## Search pipeline
- problem/need decomposition -> synonym/ontology expansion -> CPC/IPC candidate expansion -> lexical query set -> semantic query set -> backward/forward citation expansion -> assignee/inventor family expansion -> date/priority normalization -> dedupe/family clustering -> independent re-read.
- "Kill search" explicitly seeks references that invalidate novelty assumptions before drafting.
- Every claim-relevant source is immutable in the Evidence Vault with retrieved-at time, content hash, source identifier and parser version.
- Ranking is an investigation aid. It cannot label an invention "patentable," "clear," "safe," or "non-infringing."

## Reproducibility
A ResearchRun records normalized queries, source adapters and versions, time window, rate-limit behavior, raw-result hashes, reranker/model hash, exclusions, and final selected evidence. Re-running against frozen fixtures must reproduce deterministic transforms; live-source drift is reported rather than hidden.

## Adversarial quality
The Design-Around Tournament creates competing implementations constrained to avoid candidate independent-claim elements; the weakness report feeds claim/embodiment revision. Separate novelty, obviousness-risk, written-description/support, enablement, design-around, market-demand and commercialization evidence vectors are retained without collapsing them into a fake probability.

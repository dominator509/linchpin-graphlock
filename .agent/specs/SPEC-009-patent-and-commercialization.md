# SPEC-009 Patent and Commercialization Workflows

REQ-PAT-001 The invention disclosure state machine is defined in `PATENT_DOMAIN_MODEL.md`: `CAPTURE`, `CONCEPTION_CONFIRMED`, `RESEARCHING`, `ARCHITECTURE_READY`, `DRAFTING`, `FILING_PACKAGE_READY`, `FILED_RECEIPT_VERIFIED`, `PROSECUTION`, `ISSUED`, `ABANDONED`, `ARCHIVED`. Forbidden transitions: AI suggestion directly to `CONCEPTION_CONFIRMED`; package ready directly to `FILED_RECEIPT_VERIFIED`; public marketing bypassing the Disclosure Firewall.

REQ-PAT-002 The filing package state machine is `ASSEMBLING`, `LINT_FAILED`, `HUMAN_REVIEW_REQUIRED`, `READY_FOR_HUMAN_SUBMISSION`, `RECEIPT_PENDING_IMPORT`, `RECEIPT_RECONCILED`. Artifacts: specification DOCX, claims/abstract where applicable, figures, auxiliary PDF if selected, form-data worksheet, inventor/contributor worksheet, fee/entity checklist, disclosure/priority checklist, and a manifest SHA-256.

REQ-PAT-003 Every research claim carries exactly one class: `OBSERVATION`, `HYPOTHESIS`, `INFERENCE`, `LEGAL_RULE_SUMMARY`, `MARKET_SIGNAL`, `PATENT_THREAT`, `COMMERCIAL_TARGET_ASSERTION`. Only `OBSERVATION` may be emitted directly from a source record; all others store inference method and contrary evidence.

REQ-PAT-004 A docket deadline stores authoritative rule source/version, jurisdiction, trigger event, trigger evidence, base date, timezone, calculation rule ID, computed due date, extension semantics, status and review flag. A model may suggest a deadline but cannot make it authoritative without a ruleset/source mapping.

REQ-PAT-005 LINCHPIN never automates Patent Center signature, payment or submission. Filing handoff is a guided human action returning receipt evidence that is imported and reconciled.

REQ-COM-001 The commercialization package is disclosure-safe and evidence-backed, and implies no guaranteed valuation, buyer, investment outcome or transfer validity (`COMMERCIALIZATION_SPEC.md`). Every target reason cites source evidence and a date.

REQ-COM-002 Asset readiness builds a chain-of-title timeline from inventor/owner records, assignments, public Assignment Search evidence, liens/security-interest warnings when available, filing/grant/legal-status events, maintenance/deadline state, remaining-life assumptions, related families/continuations, know-how dependencies and unresolved ownership/inventorship questions. USPTO recordation is represented as recordation evidence, never as legal validation.

REQ-COM-003 Valuation outputs are ranges tied to explicit assumptions and labelled planning scenarios, not certified appraisals. Sensitivity tables show which assumptions dominate.

REQ-COM-004 The Disclosure Firewall blocks unpublished enabling detail in any go-to-market asset unless policy finds a qualifying filing event or an explicit human override is logged.

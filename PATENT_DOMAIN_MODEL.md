# Patent Domain Model and State Machines

## InventionDisclosure
States: `CAPTURE`, `CONCEPTION_CONFIRMED`, `RESEARCHING`, `ARCHITECTURE_READY`, `DRAFTING`, `FILING_PACKAGE_READY`, `FILED_RECEIPT_VERIFIED`, `PROSECUTION`, `ISSUED`, `ABANDONED`, `ARCHIVED`.

Forbidden transitions: AI suggestion directly to `CONCEPTION_CONFIRMED`; package ready directly to `FILED_RECEIPT_VERIFIED`; public marketing bypassing Disclosure Firewall.

## FilingPackage
States: `ASSEMBLING`, `LINT_FAILED`, `HUMAN_REVIEW_REQUIRED`, `READY_FOR_HUMAN_SUBMISSION`, `RECEIPT_PENDING_IMPORT`, `RECEIPT_RECONCILED`.
Artifacts: specification DOCX, claims/abstract where applicable, figures, auxiliary PDF if selected, form-data worksheet, inventor/contributor worksheet, fee/entity checklist, disclosure/priority checklist, manifest SHA-256.

## ResearchClaim
Every claim is one of `OBSERVATION`, `HYPOTHESIS`, `INFERENCE`, `LEGAL_RULE_SUMMARY`, `MARKET_SIGNAL`, `PATENT_THREAT`, `COMMERCIAL_TARGET_ASSERTION`. Only `OBSERVATION` may be emitted directly from a source record; all others store inference method and contrary evidence.

## DocketDeadline
Fields include authoritative rule source/version, jurisdiction, trigger event, trigger evidence, base date, timezone, calculation rule ID, computed due date, extension semantics, status and review flag. A model may suggest a deadline but cannot make it authoritative without a ruleset/source mapping.

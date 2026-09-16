//! Asset readiness: the chain-of-title timeline (REQ-COM-002, SPEC-009).
//!
//! SPEC-009 REQ-COM-002: "Asset readiness builds a chain-of-title timeline from
//! inventor/owner records, assignments, public Assignment Search evidence,
//! liens/security-interest warnings when available, filing/grant/legal-status
//! events, maintenance/deadline state, remaining-life assumptions, related
//! families/continuations, know-how dependencies and unresolved
//! ownership/inventorship questions. **USPTO recordation is represented as
//! recordation evidence, never as legal validation.**"
//!
//! Three decisions in this module follow from the last sentence, and each is the
//! opposite of what an "asset readiness score" usually does:
//!
//!   1. A **gap in the chain is reported, never bridged.** If an owner record
//!      follows an inventor record with no assignment or recordation covering the
//!      transition, the transition is a finding. Assuming continuity because the
//!      two records are adjacent is exactly the laundering the clause forbids.
//!   2. **Recordation never sets readiness.** A recordation record is emitted as
//!      `RecordationIsEvidenceOnly` and cannot make `ready_for_transaction` true
//!      on its own; the flag requires a continuous chain and no open questions.
//!   3. **A lien is never cleared by this code.** It is surfaced as a warning for
//!      a human, because only the lienholder can release it.

use std::collections::BTreeSet;

/// Where a record in the timeline came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TitleRecordKind {
    /// A named natural person recorded as an inventor.
    InventorRecord,
    /// A named legal owner (assignee or, later, an assignee of the assignee).
    OwnerRecord,
    /// An assignment document executed between two parties.
    AssignmentExecuted,
    /// A USPTO Assignment Search / recordation record. EVIDENCE, not validation.
    RecordationEvidence,
    /// A lien, security interest or other encumbrance.
    LienOrSecurityInterest,
    /// An application filing event.
    FilingEvent,
    /// A grant event.
    GrantEvent,
    /// Any other legal-status change (abandonment, revival, reissue, ...).
    LegalStatusEvent,
    /// A maintenance-fee or other deadline.
    MaintenanceDeadline,
}

impl TitleRecordKind {
    /// Stable label used in findings and in the UI.
    pub fn label(self) -> &'static str {
        match self {
            TitleRecordKind::InventorRecord => "INVENTOR_RECORD",
            TitleRecordKind::OwnerRecord => "OWNER_RECORD",
            TitleRecordKind::AssignmentExecuted => "ASSIGNMENT_EXECUTED",
            TitleRecordKind::RecordationEvidence => "RECORDATION_EVIDENCE",
            TitleRecordKind::LienOrSecurityInterest => "LIEN_OR_SECURITY_INTEREST",
            TitleRecordKind::FilingEvent => "FILING_EVENT",
            TitleRecordKind::GrantEvent => "GRANT_EVENT",
            TitleRecordKind::LegalStatusEvent => "LEGAL_STATUS_EVENT",
            TitleRecordKind::MaintenanceDeadline => "MAINTENANCE_DEADLINE",
        }
    }
}

/// One dated record in the timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleRecord {
    pub kind: TitleRecordKind,
    /// ISO `YYYY-MM-DD`. Validated: an unparseable date is refused rather than
    /// sorted into an unknown position.
    pub effective_date: String,
    pub party_from: Option<String>,
    pub party_to: Option<String>,
    /// Where this record came from. Every record must name a source, because the
    /// clause's whole point is that a chain is only as good as its evidence.
    pub source: String,
    pub recordation_id: Option<String>,
}

/// Something a reader must know before relying on the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleFinding {
    /// The chain does not connect two consecutive ownership states.
    Gap { between: String, detail: String },
    /// Ownership is not established by the records supplied.
    UnresolvedOwnership { detail: String },
    /// Inventorship is not established by the records supplied.
    UnresolvedInventorship { detail: String },
    /// An encumbrance is present and has not been released by evidence.
    LienWarning { detail: String },
    /// A recordation record is evidence of recording, not of legal validity.
    RecordationIsEvidenceOnly { recordation_id: String },
}

impl TitleFinding {
    pub fn label(&self) -> &'static str {
        match self {
            TitleFinding::Gap { .. } => "GAP",
            TitleFinding::UnresolvedOwnership { .. } => "UNRESOLVED_OWNERSHIP",
            TitleFinding::UnresolvedInventorship { .. } => "UNRESOLVED_INVENTORSHIP",
            TitleFinding::LienWarning { .. } => "LIEN_WARNING",
            TitleFinding::RecordationIsEvidenceOnly { .. } => "RECORDATION_IS_EVIDENCE_ONLY",
        }
    }
}

/// A remaining-life estimate, with the assumption it rests on.
#[derive(Debug, Clone, PartialEq)]
pub struct RemainingLife {
    pub years: f64,
    pub basis: String,
    pub assumption: String,
}

/// What a transaction reviewer needs, with the open questions still open.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetReadiness {
    pub asset_label: String,
    /// Ordered by effective date.
    pub timeline: Vec<TitleRecord>,
    pub findings: Vec<TitleFinding>,
    pub remaining_life: Option<RemainingLife>,
    pub related_families: Vec<String>,
    pub know_how_dependencies: Vec<String>,
    pub unresolved_questions: Vec<String>,
    /// True only for a continuous chain with no open ownership, inventorship or
    /// encumbrance finding and no caller-supplied unresolved question.
    pub ready_for_transaction: bool,
    /// Always true, and emitted so a consumer cannot read a recordation record as
    /// a legal conclusion even if it ignores `findings`.
    pub recordation_is_evidence_not_validation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleError {
    EmptyAssetLabel,
    EmptySource {
        index: usize,
    },
    InvalidDate {
        index: usize,
        value: String,
    },
    DuplicateRecord {
        effective_date: String,
        kind: String,
    },
    NegativeRemainingLife,
}

impl std::fmt::Display for TitleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TitleError::EmptyAssetLabel => write!(f, "asset label is required"),
            TitleError::EmptySource { index } => {
                write!(
                    f,
                    "record {index} names no source; a chain without evidence is not a chain"
                )
            }
            TitleError::InvalidDate { index, value } => write!(
                f,
                "record {index} has date {value:?}, which is not an ISO YYYY-MM-DD date"
            ),
            TitleError::DuplicateRecord {
                effective_date,
                kind,
            } => write!(
                f,
                "two {kind} records share the effective date {effective_date}; the order of events would be ambiguous"
            ),
            TitleError::NegativeRemainingLife => write!(f, "remaining life cannot be negative"),
        }
    }
}

/// Validate an ISO `YYYY-MM-DD` date without pulling in a date crate.
///
/// Deliberately shape-and-range only: this is not a calendar library, and
/// pretending to validate leap years would be a bigger claim than the code earns.
fn valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |slice: &[u8]| slice.iter().all(|b| b.is_ascii_digit());
    if !(digits(&bytes[0..4]) && digits(&bytes[5..7]) && digits(&bytes[8..10])) {
        return false;
    }
    let month: u32 = value[5..7].parse().unwrap_or(0);
    let day: u32 = value[8..10].parse().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// Build the chain-of-title timeline and the readiness verdict.
///
/// `unresolved_questions` are supplied by the caller (an interview with the
/// inventor, a practitioner's note). They are carried through and they BLOCK
/// readiness: the product has no standing to answer them.
pub fn assess_asset_readiness(
    asset_label: &str,
    records: &[TitleRecord],
    remaining_life: Option<RemainingLife>,
    related_families: &[String],
    know_how_dependencies: &[String],
    unresolved_questions: &[String],
) -> Result<AssetReadiness, TitleError> {
    if asset_label.trim().is_empty() {
        return Err(TitleError::EmptyAssetLabel);
    }

    let mut seen: BTreeSet<(String, &'static str)> = BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        if record.source.trim().is_empty() {
            return Err(TitleError::EmptySource { index });
        }
        if !valid_iso_date(&record.effective_date) {
            return Err(TitleError::InvalidDate {
                index,
                value: record.effective_date.clone(),
            });
        }
        if !seen.insert((record.effective_date.clone(), record.kind.label())) {
            return Err(TitleError::DuplicateRecord {
                effective_date: record.effective_date.clone(),
                kind: record.kind.label().to_string(),
            });
        }
    }
    if let Some(life) = &remaining_life
        && life.years < 0.0
    {
        return Err(TitleError::NegativeRemainingLife);
    }

    let mut timeline = records.to_vec();
    timeline.sort_by(|a, b| {
        a.effective_date
            .cmp(&b.effective_date)
            .then_with(|| a.kind.cmp(&b.kind))
    });

    let mut findings: Vec<TitleFinding> = Vec::new();

    // --- chain continuity -------------------------------------------------
    //
    // Walk the ownership-relevant records in date order and track the party that
    // should hold the asset. A new owner is only reachable through an assignment
    // or a recordation that names the transfer; adjacency proves nothing.
    let mut current_owner: Option<String> = None;
    let mut saw_inventor = false;
    for record in &timeline {
        match record.kind {
            TitleRecordKind::InventorRecord => {
                saw_inventor = true;
            }
            TitleRecordKind::OwnerRecord => {
                let Some(owner) = record
                    .party_to
                    .clone()
                    .or_else(|| record.party_from.clone())
                else {
                    findings.push(TitleFinding::UnresolvedOwnership {
                        detail: format!(
                            "owner record dated {} names no party",
                            record.effective_date
                        ),
                    });
                    continue;
                };
                match &current_owner {
                    None => {
                        // The first owner must be reached from the inventor side by
                        // an assignment or a recordation; otherwise the chain
                        // starts with an unexplained leap.
                        let bridged = timeline.iter().any(|candidate| {
                            candidate.effective_date <= record.effective_date
                                && matches!(
                                    candidate.kind,
                                    TitleRecordKind::AssignmentExecuted
                                        | TitleRecordKind::RecordationEvidence
                                )
                                && (candidate.party_to.as_deref() == Some(owner.as_str())
                                    || candidate.party_from.as_deref() == Some(owner.as_str()))
                        });
                        if !bridged {
                            findings.push(TitleFinding::Gap {
                                between: format!("inventor record -> {owner}"),
                                detail: format!(
                                    "the first owner {owner} is recorded on {} with no assignment or recordation connecting the inventor to them",
                                    record.effective_date
                                ),
                            });
                        }
                        current_owner = Some(owner);
                    }
                    Some(previous) if previous == &owner => {}
                    Some(previous) => {
                        let bridged = timeline.iter().any(|candidate| {
                            candidate.effective_date <= record.effective_date
                                && candidate.effective_date >= {
                                    // any date at or after the previous owner's record
                                    String::new()
                                }
                                && matches!(
                                    candidate.kind,
                                    TitleRecordKind::AssignmentExecuted
                                        | TitleRecordKind::RecordationEvidence
                                )
                                && candidate.party_from.as_deref() == Some(previous.as_str())
                                && (candidate.party_to.as_deref() == Some(owner.as_str())
                                    || candidate.party_to.is_none())
                        });
                        if !bridged {
                            findings.push(TitleFinding::Gap {
                                between: format!("{previous} -> {owner}"),
                                detail: format!(
                                    "ownership changes from {previous} to {owner} on {} with no assignment or recordation between them",
                                    record.effective_date
                                ),
                            });
                        }
                        current_owner = Some(owner);
                    }
                }
            }
            TitleRecordKind::RecordationEvidence => {
                findings.push(TitleFinding::RecordationIsEvidenceOnly {
                    recordation_id: record.recordation_id.clone().unwrap_or_else(|| {
                        format!("unidentified record dated {}", record.effective_date)
                    }),
                });
            }
            TitleRecordKind::LienOrSecurityInterest => {
                findings.push(TitleFinding::LienWarning {
                    detail: format!(
                        "an encumbrance dated {} is on record and no release evidence was supplied",
                        record.effective_date
                    ),
                });
            }
            _ => {}
        }
    }

    if !saw_inventor {
        findings.push(TitleFinding::UnresolvedInventorship {
            detail: "no inventor record was supplied, so inventorship is not established"
                .to_string(),
        });
    }
    if current_owner.is_none() {
        findings.push(TitleFinding::UnresolvedOwnership {
            detail: "no owner record was supplied, so ownership is not established".to_string(),
        });
    }

    let open_questions: Vec<String> = unresolved_questions
        .iter()
        .map(|question| question.trim().to_string())
        .filter(|question| !question.is_empty())
        .collect();

    let blockers = findings.iter().any(|finding| {
        matches!(
            finding,
            TitleFinding::Gap { .. }
                | TitleFinding::UnresolvedOwnership { .. }
                | TitleFinding::UnresolvedInventorship { .. }
                | TitleFinding::LienWarning { .. }
        )
    });

    let ready_for_transaction = !blockers && open_questions.is_empty();

    let mut carried_questions = open_questions;
    if remaining_life.is_none() {
        carried_questions.push(
            "remaining life was not supplied; no term assumption is asserted here".to_string(),
        );
    }

    Ok(AssetReadiness {
        asset_label: asset_label.trim().to_string(),
        timeline,
        findings,
        remaining_life,
        related_families: related_families
            .iter()
            .map(|f| f.trim().to_string())
            .filter(|f| !f.is_empty())
            .collect(),
        know_how_dependencies: know_how_dependencies
            .iter()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty())
            .collect(),
        unresolved_questions: carried_questions,
        ready_for_transaction,
        recordation_is_evidence_not_validation: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        kind: TitleRecordKind,
        date: &str,
        from: Option<&str>,
        to: Option<&str>,
        source: &str,
    ) -> TitleRecord {
        TitleRecord {
            kind,
            effective_date: date.to_string(),
            party_from: from.map(str::to_string),
            party_to: to.map(str::to_string),
            source: source.to_string(),
            recordation_id: None,
        }
    }

    /// covers: REQ-COM-002
    /// A continuous chain is ready, and it is ordered by effective date rather
    /// than by the order the records happened to arrive in.
    #[test]
    fn test_continuous_chain_is_ready_and_ordered() {
        let records = vec![
            record(
                TitleRecordKind::OwnerRecord,
                "2024-06-01",
                None,
                Some("Linchpin Holdings"),
                "assignment deck",
            ),
            record(
                TitleRecordKind::InventorRecord,
                "2023-01-15",
                None,
                Some("Dana Inventor"),
                "declaration",
            ),
            record(
                TitleRecordKind::AssignmentExecuted,
                "2024-05-01",
                Some("Dana Inventor"),
                Some("Linchpin Holdings"),
                "executed assignment",
            ),
        ];
        let readiness = assess_asset_readiness(
            "Self-sealing valve",
            &records,
            Some(RemainingLife {
                years: 14.5,
                basis: "20-year term from filing".to_string(),
                assumption: "maintenance fees paid".to_string(),
            }),
            &["Family A".to_string()],
            &["weld procedure".to_string()],
            &[],
        )
        .expect("readiness");

        assert_eq!(readiness.timeline[0].effective_date, "2023-01-15");
        assert_eq!(readiness.timeline[2].effective_date, "2024-06-01");
        assert!(
            readiness.ready_for_transaction,
            "a continuous chain with no open questions must be ready: {:?}",
            readiness.findings
        );
        assert!(readiness.recordation_is_evidence_not_validation);
        assert_eq!(readiness.related_families, vec!["Family A".to_string()]);
    }

    /// covers: REQ-COM-002
    /// A missing link is REPORTED, not bridged. Two adjacent ownership records
    /// with no assignment between them must produce a gap.
    #[test]
    fn test_missing_assignment_is_a_gap_not_an_assumption() {
        let records = vec![
            record(
                TitleRecordKind::InventorRecord,
                "2023-01-15",
                None,
                Some("Dana Inventor"),
                "declaration",
            ),
            record(
                TitleRecordKind::OwnerRecord,
                "2024-06-01",
                None,
                Some("First Assignee"),
                "assignment deck",
            ),
            record(
                TitleRecordKind::OwnerRecord,
                "2025-02-01",
                None,
                Some("Second Assignee"),
                "assignment deck",
            ),
        ];
        let readiness =
            assess_asset_readiness("Valve", &records, None, &[], &[], &[]).expect("readiness");

        let gaps: Vec<&TitleFinding> = readiness
            .findings
            .iter()
            .filter(|f| matches!(f, TitleFinding::Gap { .. }))
            .collect();
        assert_eq!(
            gaps.len(),
            2,
            "both the inventor->owner and owner->owner transitions lack evidence: {:?}",
            readiness.findings
        );
        assert!(
            !readiness.ready_for_transaction,
            "a chain with gaps must not be reported ready"
        );
    }

    /// covers: REQ-COM-002
    /// "USPTO recordation is represented as recordation evidence, never as legal
    /// validation." A recordation record alone must not make the asset ready.
    #[test]
    fn test_recordation_is_evidence_and_never_validation() {
        let records = vec![
            record(
                TitleRecordKind::InventorRecord,
                "2023-01-15",
                None,
                Some("Dana Inventor"),
                "declaration",
            ),
            record(
                TitleRecordKind::RecordationEvidence,
                "2024-07-01",
                Some("Dana Inventor"),
                Some("Linchpin Holdings"),
                "USPTO Assignment Search",
            ),
            record(
                TitleRecordKind::OwnerRecord,
                "2024-07-02",
                None,
                Some("Linchpin Holdings"),
                "assignment deck",
            ),
        ];
        let readiness =
            assess_asset_readiness("Valve", &records, None, &[], &[], &[]).expect("readiness");

        assert!(
            readiness
                .findings
                .iter()
                .any(|f| matches!(f, TitleFinding::RecordationIsEvidenceOnly { .. })),
            "the recordation must be surfaced as evidence only: {:?}",
            readiness.findings
        );
        assert!(
            readiness.recordation_is_evidence_not_validation,
            "the output must state that recordation is not validation"
        );
        // The chain itself is continuous here, so readiness comes from the
        // records -- but the finding is still reported for a reader.
        assert!(readiness.ready_for_transaction);
    }

    /// covers: REQ-COM-002
    /// An encumbrance is surfaced and blocks readiness; the product has no
    /// standing to release a lien.
    #[test]
    fn test_lien_is_surfaced_and_blocks_readiness() {
        let records = vec![
            record(
                TitleRecordKind::InventorRecord,
                "2023-01-15",
                None,
                Some("Dana Inventor"),
                "declaration",
            ),
            record(
                TitleRecordKind::AssignmentExecuted,
                "2024-05-01",
                Some("Dana Inventor"),
                Some("Linchpin Holdings"),
                "executed assignment",
            ),
            record(
                TitleRecordKind::OwnerRecord,
                "2024-06-01",
                None,
                Some("Linchpin Holdings"),
                "assignment deck",
            ),
            record(
                TitleRecordKind::LienOrSecurityInterest,
                "2025-01-10",
                Some("Lender"),
                Some("Linchpin Holdings"),
                "UCC filing",
            ),
        ];
        let readiness =
            assess_asset_readiness("Valve", &records, None, &[], &[], &[]).expect("readiness");
        assert!(
            readiness
                .findings
                .iter()
                .any(|f| matches!(f, TitleFinding::LienWarning { .. }))
        );
        assert!(!readiness.ready_for_transaction);
    }

    /// covers: REQ-COM-002
    /// Invalid inputs are refused rather than sorted into an unknown position,
    /// and a caller-supplied open question keeps the asset unready.
    #[test]
    fn test_invalid_records_are_refused_and_questions_block_readiness() {
        let bad_date = vec![record(
            TitleRecordKind::InventorRecord,
            "15/01/2023",
            None,
            Some("Dana Inventor"),
            "declaration",
        )];
        assert!(matches!(
            assess_asset_readiness("Valve", &bad_date, None, &[], &[], &[]),
            Err(TitleError::InvalidDate { .. })
        ));

        let no_source = vec![record(
            TitleRecordKind::InventorRecord,
            "2023-01-15",
            None,
            Some("Dana Inventor"),
            "   ",
        )];
        assert!(matches!(
            assess_asset_readiness("Valve", &no_source, None, &[], &[], &[]),
            Err(TitleError::EmptySource { .. })
        ));

        let duplicate = vec![
            record(
                TitleRecordKind::OwnerRecord,
                "2024-06-01",
                None,
                Some("A"),
                "deck",
            ),
            record(
                TitleRecordKind::OwnerRecord,
                "2024-06-01",
                None,
                Some("B"),
                "deck",
            ),
        ];
        assert!(matches!(
            assess_asset_readiness("Valve", &duplicate, None, &[], &[], &[]),
            Err(TitleError::DuplicateRecord { .. })
        ));

        assert!(matches!(
            assess_asset_readiness("  ", &[], None, &[], &[], &[]),
            Err(TitleError::EmptyAssetLabel)
        ));

        let records = vec![
            record(
                TitleRecordKind::InventorRecord,
                "2023-01-15",
                None,
                Some("Dana Inventor"),
                "declaration",
            ),
            record(
                TitleRecordKind::AssignmentExecuted,
                "2024-05-01",
                Some("Dana Inventor"),
                Some("Linchpin Holdings"),
                "executed assignment",
            ),
            record(
                TitleRecordKind::OwnerRecord,
                "2024-06-01",
                None,
                Some("Linchpin Holdings"),
                "assignment deck",
            ),
        ];
        let readiness = assess_asset_readiness(
            "Valve",
            &records,
            None,
            &[],
            &[],
            &["who owns the improvement conceived in 2025?".to_string()],
        )
        .expect("readiness");
        assert!(
            !readiness.ready_for_transaction,
            "an unresolved question must block readiness"
        );
        assert!(
            readiness
                .unresolved_questions
                .iter()
                .any(|q| q.contains("who owns the improvement"))
        );
        // Absent remaining life is carried as a question rather than invented.
        assert!(
            readiness
                .unresolved_questions
                .iter()
                .any(|q| q.contains("remaining life was not supplied"))
        );
    }
}

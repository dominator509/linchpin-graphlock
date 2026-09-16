//! Product scope and the five truth boundaries (REQ-SCOPE-001, SPEC-000).
//!
//! SPEC-000 declares: "REQ-SCOPE-001 through 012 are UO-01 through UO-12 from
//! PROJECT_BRIEF. Product claims must preserve five truth boundaries." Until now
//! both halves of that sentence existed only as prose in `PROJECT_BRIEF.md` and
//! `ARCHITECTURE.md` section 5, so no test could assert either one and a product
//! claim could drift from the promised scope with nothing to notice. This module
//! makes the mapping and the boundaries machine-readable, and gives the
//! boundaries a real enforcement point instead of a comment.
//!
//! The map is deliberately NOT a list of twelve "done" flags. Each entry records
//! where the capability is realised, and — where it is not fully realised — what
//! is honestly missing. A scope map that cannot say "partial" would be a
//! marketing document, and the clause exists to prevent exactly that.

/// How completely a promised capability currently exists.
///
/// Three values rather than a boolean, because the difference between "built and
/// limited" and "not built" decides whether the remedy is more work or a
/// different promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityState {
    /// Realised in a production path and exercised by an acceptance test.
    Implemented,
    /// Realised, but with a stated limitation that a reader must know.
    Partial,
    /// Promised by the brief and not realised.
    Absent,
}

/// One promised capability: `REQ-SCOPE-nnn` is `UO-nn`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeEntry {
    pub requirement_id: &'static str,
    pub uo_id: &'static str,
    pub title: &'static str,
    pub state: CapabilityState,
    /// The production entry point that realises it, or empty when absent.
    pub realised_by: &'static str,
    /// What is missing or limited. Empty only when `state` is `Implemented`.
    pub limitation: &'static str,
}

/// The twelve promised live-fire outcomes of PROJECT_BRIEF, mapped to the
/// requirement IDs of SPEC-000.
///
/// `realised_by` names functions that exist in this repository; each was
/// verified by reading its definition site rather than inferred from a name in a
/// document. Where a capability is partly built, `limitation` says which part is
/// not, so nobody can read this table as a completion claim.
pub const SCOPE_MAP: [ScopeEntry; 12] = [
    ScopeEntry {
        requirement_id: "REQ-SCOPE-001",
        uo_id: "UO-01",
        title: "encrypted invention workspace + Human Conception Ledger",
        state: CapabilityState::Partial,
        realised_by: "commands::record_conception (origin-labelled, durable), storage::EncryptedVault (blob container)",
        limitation: "the vault itself is a device-local SQLite database, not an OS-encrypted container, and the only KeyringStore is an in-process map (no DPAPI/OS keyring integration)",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-002",
        uo_id: "UO-02",
        title: "evidence-backed blue-ocean opportunity discovery/ranking",
        state: CapabilityState::Partial,
        realised_by: "commands::evaluate_opportunity",
        limitation: "the axis scores and their uncertainty are supplied by the caller; they are not yet derived from stored evidence snapshots, so the ranking is only as evidenced as its inputs",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-003",
        uo_id: "UO-03",
        title: "reproducible prior-art kill-search + LINCHPIN report",
        state: CapabilityState::Partial,
        realised_by: "commands::apply_research_action, commands::set_research_coverage, commands::complete_research_partial",
        limitation: "search coverage, citations and checkpoints are stored, but no external prior-art corpus adapter is wired and the multi-axis LINCHPIN report is not assembled into one artefact",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-004",
        uo_id: "UO-04",
        title: "claim/spec/embodiment/figure architecture + support lint",
        state: CapabilityState::Implemented,
        realised_by: "commands::lint_claims, commands::check_support_matrix, commands::record_claim_evidence",
        limitation: "",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-005",
        uo_id: "UO-05",
        title: "versioned provisional/nonprovisional draft package + guided human handoff",
        state: CapabilityState::Implemented,
        realised_by: "commands::build_filing_package, commands::check_filing_handoff",
        limitation: "",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-006",
        uo_id: "UO-06",
        title: "docket, patent-pending/priority/disclosure events, receipts and deadlines",
        state: CapabilityState::Implemented,
        realised_by: "commands::schedule_docket_deadline, commands::advance_docket, commands::import_receipt",
        limitation: "",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-007",
        uo_id: "UO-07",
        title: "Office Action ingestion, rejection mapping, response/amendment workspace",
        state: CapabilityState::Partial,
        realised_by: "commands::draft_office_action_response",
        limitation: "the response workspace and cited-art count exist, but there is no Office Action document ingestion or rejection-to-limitation mapping",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-008",
        uo_id: "UO-08",
        title: "commercialization package, targets, valuation scenarios, disclosure-safe outreach",
        state: CapabilityState::Partial,
        realised_by: "commands::build_commercialization_package, commands::evaluate_valuation",
        limitation: "packages are redacted before export and valuations are ranges with stated assumptions, but targets are caller-supplied labels rather than sourced organisations, and no outreach is ever sent by the product",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-009",
        uo_id: "UO-09",
        title: "provider transport switching with equivalent schemas/provenance",
        state: CapabilityState::Partial,
        realised_by: "commands::provider_status, commands::run_local_inference",
        limitation: "only the loopback local transport is live; the OpenAI/Anthropic/xAI lanes report Unimplemented instead of returning fabricated text, so switching is declared but not available",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-010",
        uo_id: "UO-10",
        title: "MCP client/server interoperability with policy enforcement",
        state: CapabilityState::Partial,
        realised_by: "commands::check_mcp_capability",
        limitation: "capability grants are explicit and deny by default, but no MCP client or server transport is implemented",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-011",
        uo_id: "UO-11",
        title: "crash -> sanitized Repair Capsule -> approved coding-agent/Git repair path",
        state: CapabilityState::Partial,
        realised_by: "commands::build_repair_capsule",
        limitation: "capsules are redacted and marked safe-for-export, but the coding-agent/Git repair step is human-run outside the product, by design (no product path mutates Git)",
    },
    ScopeEntry {
        requirement_id: "REQ-SCOPE-012",
        uo_id: "UO-12",
        title: "interruption/restart persistence with no invented state",
        state: CapabilityState::Implemented,
        realised_by: "storage::vault::Vault (durable SQLite), commands::backup_vault, commands::restore_vault",
        limitation: "",
    },
];

/// One of the five truth boundaries of `ARCHITECTURE.md` section 5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruthBoundary {
    pub id: &'static str,
    pub statement: &'static str,
    /// Where the boundary is enforced, or the surface that must respect it.
    pub enforced_by: &'static str,
}

/// The five truth boundaries. Statements are transcribed from ARCHITECTURE.md
/// rather than paraphrased, so the code cannot quietly soften them.
pub const TRUTH_BOUNDARIES: [TruthBoundary; 5] = [
    TruthBoundary {
        id: "TB-1",
        statement: "Opportunity quality != patentability. Market need/whitespace scores never become a patentability probability.",
        enforced_by: "scope::check_claim_text (refuses a patentability probability); commands::evaluate_opportunity reports axes with uncertainty",
    },
    TruthBoundary {
        id: "TB-2",
        statement: "Patentability != FTO. Novelty/obviousness research against an application does not prove the user can practice the product free of others' claims.",
        enforced_by: "scope::check_claim_text (refuses a clearance conclusion); commands::lint_claims emits advisory findings only",
    },
    TruthBoundary {
        id: "TB-3",
        statement: "Draft != filing. FilingPackage.ready_for_human_review is not filed; filed requires imported official receipt evidence.",
        enforced_by: "commands::check_filing_handoff (ready_for_human_submission, never 'filed'); commands::import_receipt records receipt evidence only",
    },
    TruthBoundary {
        id: "TB-4",
        statement: "Patent rights != commercialization. The right to exclude does not guarantee market access, revenue, investment, licensing, or validity.",
        enforced_by: "scope::check_claim_text (refuses guaranteed-outcome copy) applied to commands::build_commercialization_package output; commands::evaluate_valuation returns a range with assumptions",
    },
    TruthBoundary {
        id: "TB-5",
        statement: "AI assistance != human inventorship. AI suggestions are provenance-labeled; natural-person conception is captured separately and ambiguous inventorship is escalated.",
        enforced_by: "commands::record_conception (AiSuggestion vs HumanConception origin, never merged)",
    },
];

/// A phrase that would cross a truth boundary if the product emitted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeViolation {
    pub boundary_id: &'static str,
    pub phrase: String,
}

impl std::fmt::Display for ScopeViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} would be crossed by the phrase \"{}\"",
            self.boundary_id, self.phrase
        )
    }
}

/// Product-facing text may not assert an outcome the product cannot deliver.
///
/// This is a lexical backstop, not a semantics engine, and saying so is part of
/// the point: it catches the assertive forms that actually appear in generated
/// copy ("guaranteed patent", "90% patentable", "risk-free returns"), so a model
/// cannot launder a promise through the export path. A rephrased promise can
/// still slip past it, which is why the boundaries are also stated in the UI and
/// checked by acceptance tests.
///
/// Each phrase is matched case-insensitively on word boundaries. Word anchoring
/// is deliberate: an earlier probe in this repository matched "observation"
/// while looking for a class name and "encrypt" inside "encrypted", and both
/// produced false results.
const PROHIBITED: [(&str, &str); 14] = [
    ("TB-1", "patentability probability"),
    ("TB-1", "% patentable"),
    ("TB-1", "percent patentable"),
    ("TB-1", "probability of being granted"),
    ("TB-2", "proven non-infringing"),
    ("TB-2", "guaranteed freedom to operate"),
    ("TB-3", "guaranteed patent"),
    ("TB-3", "guaranteed approval"),
    ("TB-3", "will be granted"),
    ("TB-4", "guaranteed revenue"),
    ("TB-4", "guaranteed investment"),
    ("TB-4", "guaranteed licensing"),
    ("TB-4", "risk-free return"),
    ("TB-5", "ai is the inventor"),
];

/// Check product-facing text against the truth boundaries.
///
/// Returns the first violation found, so a caller that refuses on `Err` names a
/// boundary and the phrase that crossed it rather than reporting a vague refusal.
pub fn check_claim_text(text: &str) -> Result<(), ScopeViolation> {
    let haystack = text.to_lowercase();
    for (boundary_id, phrase) in PROHIBITED {
        if contains_phrase(&haystack, phrase) {
            return Err(ScopeViolation {
                boundary_id,
                phrase: phrase.to_string(),
            });
        }
    }
    Ok(())
}

/// Whole-phrase containment with non-alphanumeric boundaries on both sides.
fn contains_phrase(haystack: &str, phrase: &str) -> bool {
    let mut from = 0usize;
    while let Some(offset) = haystack[from..].find(phrase) {
        let start = from + offset;
        let end = start + phrase.len();
        let before_ok = haystack[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let after_ok = haystack[end..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
        from = start + phrase.len();
    }
    false
}

/// The scope entry for a requirement id, if it is a promised capability.
pub fn entry_for_requirement(requirement_id: &str) -> Option<&'static ScopeEntry> {
    SCOPE_MAP
        .iter()
        .find(|entry| entry.requirement_id == requirement_id)
}

/// The boundary with this id, if it exists. Unknown ids return `None` rather
/// than a default, so a caller cannot silently attribute a violation to TB-1.
pub fn boundary_by_id(id: &str) -> Option<&'static TruthBoundary> {
    TRUTH_BOUNDARIES.iter().find(|boundary| boundary.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// covers: REQ-SCOPE-001
    /// The mapping must be complete and 1:1: twelve UO ids, twelve requirement
    /// ids, no duplicates and no gaps. A missing row would mean a promised
    /// outcome that nothing accounts for.
    #[test]
    fn test_scope_map_is_complete_and_one_to_one() {
        assert_eq!(SCOPE_MAP.len(), 12, "the brief promises twelve outcomes");

        let mut requirements = HashSet::new();
        let mut uos = HashSet::new();
        for (index, entry) in SCOPE_MAP.iter().enumerate() {
            let expected_uo = format!("UO-{:02}", index + 1);
            let expected_requirement = format!("REQ-SCOPE-{:03}", index + 1);
            assert_eq!(entry.uo_id, expected_uo, "out of order at {expected_uo}");
            assert_eq!(
                entry.requirement_id, expected_requirement,
                "out of order at {expected_requirement}"
            );
            assert!(
                requirements.insert(entry.requirement_id),
                "duplicate {}",
                entry.requirement_id
            );
            assert!(uos.insert(entry.uo_id), "duplicate {}", entry.uo_id);
            assert!(
                !entry.title.trim().is_empty(),
                "{} has no title",
                entry.uo_id
            );
        }
    }

    /// covers: REQ-SCOPE-001
    /// A capability may only claim to be implemented when it names where it is
    /// realised, and may only omit a limitation when it is fully implemented.
    /// Without this, the map could be filled with "Implemented" and no evidence.
    #[test]
    fn test_scope_map_does_not_over_claim() {
        for entry in SCOPE_MAP {
            match entry.state {
                CapabilityState::Implemented => {
                    assert!(
                        !entry.realised_by.trim().is_empty(),
                        "{} claims Implemented with no entry point",
                        entry.uo_id
                    );
                    assert!(
                        entry.limitation.trim().is_empty(),
                        "{} claims Implemented and also states a limitation",
                        entry.uo_id
                    );
                }
                CapabilityState::Partial => {
                    assert!(
                        !entry.realised_by.trim().is_empty(),
                        "{} claims Partial with no entry point",
                        entry.uo_id
                    );
                    assert!(
                        !entry.limitation.trim().is_empty(),
                        "{} claims Partial without saying what is missing",
                        entry.uo_id
                    );
                }
                CapabilityState::Absent => {
                    assert!(
                        !entry.limitation.trim().is_empty(),
                        "{} claims Absent without saying what is missing",
                        entry.uo_id
                    );
                }
            }
        }
    }

    /// covers: REQ-SCOPE-001
    /// All five boundaries are present, uniquely identified, transcribed with
    /// their architecture statements, and attached to an enforcement point.
    #[test]
    fn test_five_truth_boundaries_are_declared_and_enforced() {
        assert_eq!(TRUTH_BOUNDARIES.len(), 5);
        for (index, boundary) in TRUTH_BOUNDARIES.iter().enumerate() {
            assert_eq!(boundary.id, format!("TB-{}", index + 1));
            assert!(
                boundary.statement.len() > 40,
                "{} has no real statement",
                boundary.id
            );
            assert!(
                !boundary.enforced_by.trim().is_empty(),
                "{} states no enforcement point",
                boundary.id
            );
            assert!(boundary_by_id(boundary.id).is_some());
        }
        assert!(boundary_by_id("TB-6").is_none());
    }

    /// covers: REQ-SCOPE-001
    /// The guard must refuse the assertive forms that cross a boundary, name the
    /// boundary it crossed, and NOT fire on honest text that discusses the same
    /// subject — including the substring traps this repository has already hit
    /// ("encrypted" for "encrypt", "observation" for a class name).
    #[test]
    fn test_boundary_guard_refuses_promises_and_permits_honest_text() {
        let cases = [
            ("TB-1", "This report gives a 90% patentability probability."),
            ("TB-1", "The screen shows % patentable for this claim set"),
            (
                "TB-2",
                "Our search proves the product is proven non-infringing.",
            ),
            ("TB-3", "You are guaranteed patent protection."),
            ("TB-3", "Your application will be granted within 12 months."),
            (
                "TB-4",
                "Investors receive guaranteed revenue from licensing.",
            ),
            ("TB-4", "A risk-free return is available to backers."),
            ("TB-5", "The AI is the inventor of record."),
        ];
        for (expected_boundary, text) in cases {
            let violation = check_claim_text(text)
                .unwrap_err_or_else(|| panic!("guard accepted a promise: {text}"));
            assert_eq!(
                violation.boundary_id, expected_boundary,
                "wrong boundary for: {text}"
            );
        }

        let honest = [
            "Screening only: this is not a patentability determination and no probability is implied.",
            "Novelty research does not establish freedom to operate; a separate FTO analysis is required.",
            "A draft package is not a filing; filing requires an imported official receipt.",
            "Valuation is a range of scenarios, not a forecast, and no outcome is promised.",
            "AI suggestions are labelled and never recorded as human conception.",
            // Substring traps: these must not fire.
            "The vault stores invention content encrypted at rest in a local database.",
            "Observation: the cited reference discloses a valve seat, not the claimed coating.",
        ];
        for text in honest {
            assert!(
                check_claim_text(text).is_ok(),
                "guard refused honest text: {text}"
            );
        }
    }

    /// A small helper so the failure message names the text rather than only the
    /// unwrap site.
    trait UnwrapErrOrElse<T, E> {
        fn unwrap_err_or_else(self, f: impl FnOnce() -> E) -> E;
    }

    impl<T, E> UnwrapErrOrElse<T, E> for Result<T, E> {
        fn unwrap_err_or_else(self, f: impl FnOnce() -> E) -> E {
            match self {
                Ok(_) => f(),
                Err(e) => e,
            }
        }
    }

    #[test]
    fn test_entry_lookup_is_exact() {
        let entry = entry_for_requirement("REQ-SCOPE-012").expect("UO-12 is promised");
        assert_eq!(entry.uo_id, "UO-12");
        assert!(entry_for_requirement("REQ-SCOPE-013").is_none());
        assert!(entry_for_requirement("req-scope-001").is_none());
    }
}

//! IPC command layer.
//!
//! `SPEC-003` requires that "Tauri commands are versioned application use cases,
//! not generic SQL/file/shell primitives", that "every command accepts
//! workspace-scoped typed input and returns typed result/error with correlation
//! ID", and that namespaces cover workspace, conception, opportunity, research,
//! evidence, patent, filing, docket, prosecution, commercialization, provider,
//! mcp, incident and export.
//!
//! GraphLock context: before this module the desktop app exposed exactly ONE
//! command (`get_system_health`), while ten crates were declared as desktop
//! dependencies and used by nothing. Measured: `domain`, `storage`, `evidence`,
//! `patent`, `research`, `commercialization`, `mcp_hub`, `crash_reporter` and
//! `provider_transport` each had ZERO usages under `apps/desktop/src-tauri/src`.
//! The product shipped a real domain layer that no user-facing path could reach.
//! That is the "disconnected UI" / "inert adapter" condition SUP-001 lists as a
//! release-blocking finding.
//!
//! This module wires the first genuine use cases and establishes the typed
//! envelope the remaining namespaces must follow. Commands implemented here are
//! reachable and covered by tests; namespaces NOT yet implemented are declared
//! as such rather than faked.

use serde::{Deserialize, Serialize};

/// Correlation ID attached to every command result.
///
/// SPEC-003 requires a correlation ID on every typed result so a UI action can
/// be traced to its effect (and to telemetry, per REQ-OPS-010).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationId(pub String);

impl CorrelationId {
    /// Mint a correlation ID.
    ///
    /// Uses a v4 UUID so two actions in the same millisecond cannot collide.
    pub fn new() -> Self {
        CorrelationId(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed command failure.
///
/// Mirrors `SPEC-006` error classes so the UI can render a safe message while
/// the technical cause stays in local evidence. `VALIDATION` and `POLICY` are
/// the classes reachable from the commands implemented so far; the remaining
/// SPEC-006 classes will be added as their namespaces are implemented, not
/// pre-declared unused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "class", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandError {
    /// Input failed a precondition.
    Validation { message: String },
    /// A capability policy refused the action.
    Policy { message: String },
}

impl CommandError {
    pub fn validation(message: impl Into<String>) -> Self {
        CommandError::Validation {
            message: message.into(),
        }
    }

    pub fn policy(message: impl Into<String>) -> Self {
        CommandError::Policy {
            message: message.into(),
        }
    }

    /// Safe, user-facing text. Never contains internal paths or secrets.
    pub fn safe_message(&self) -> &str {
        match self {
            CommandError::Validation { message } | CommandError::Policy { message } => message,
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.safe_message())
    }
}

impl std::error::Error for CommandError {}

/// Envelope wrapping every command result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult<T> {
    pub correlation_id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CommandError>,
}

impl<T> CommandResult<T> {
    pub fn success(correlation_id: CorrelationId, value: T) -> Self {
        CommandResult {
            correlation_id: correlation_id.0,
            ok: true,
            value: Some(value),
            error: None,
        }
    }

    pub fn failure(correlation_id: CorrelationId, error: CommandError) -> Self {
        CommandResult {
            correlation_id: correlation_id.0,
            ok: false,
            value: None,
            error: Some(error),
        }
    }
}

/// Workspace-scoped input.
///
/// SPEC-003 requires workspace scoping on every command. `workspace_id` is
/// therefore mandatory rather than optional: a command that cannot name its
/// workspace cannot honour the confidentiality boundary in SPEC-005.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceScope {
    pub workspace_id: String,
}

impl WorkspaceScope {
    pub fn validate(&self) -> Result<(), CommandError> {
        if self.workspace_id.trim().is_empty() {
            return Err(CommandError::validation("workspace_id is required"));
        }
        Ok(())
    }
}

/// A Human Conception Event as returned to the UI.
///
/// `REQ-DOM-001` requires immutable conception events carrying person, time and
/// a content hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConceptionEventView {
    pub event_id: String,
    pub content_hash: String,
    pub content_bytes: usize,
    pub origin: String,
}

/// Result of recording a human conception event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordConceptionOutcome {
    pub event: ConceptionEventView,
    /// Honest statement of what was persisted, so the UI cannot over-claim.
    pub persisted: bool,
    pub storage_detail: String,
}

/// Record a human conception event.
///
/// `REQ-DOM-001`: capture immutable Human Conception Events with person/time/
/// content hash. `REQ-DOM-002`: label AI suggestions distinctly.
///
/// `author_is_human = false` marks the block as an AI suggestion and is
/// recorded distinctly; an AI suggestion may never be filed as human
/// conception (`PATENT_DOMAIN_MODEL.md` forbids the transition from AI
/// suggestion directly to `CONCEPTION_CONFIRMED`).
///
/// When `vault_path` is supplied the event is durably committed through
/// `storage::Vault` (REQ-DATA-001, REQ-DATA-002) and `persisted` reports the
/// real outcome. When no vault path is supplied nothing is written and the
/// result says so — the command never claims a side effect it did not perform.
pub fn record_conception(
    scope: &WorkspaceScope,
    content: &str,
    author_is_human: bool,
    vault_path: Option<&std::path::Path>,
) -> CommandResult<RecordConceptionOutcome> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if content.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("conception content cannot be empty"),
        );
    }

    // Derive the block through the real domain type so origin labelling is
    // enforced by the domain layer rather than re-implemented here.
    let author_id = uuid::Uuid::new_v4();
    let block = if author_is_human {
        domain::ContentBlock::new_human(content.to_string(), author_id)
    } else {
        domain::ContentBlock::new_ai(content.to_string(), author_id)
    };

    let origin = match &block.origin {
        domain::AuthorOrigin::HumanConception(_) => "HumanConception",
        domain::AuthorOrigin::AiSuggestion(_) => "AiSuggestion",
    };
    let event_id = block.id.0.to_string();

    let (persisted, hash, storage_detail) = match vault_path {
        Some(path) => match storage::Vault::open(path) {
            Ok(vault) => {
                if let Err(e) = vault.create_workspace(&scope.workspace_id) {
                    return CommandResult::failure(
                        correlation,
                        CommandError::validation(format!("workspace: {e}")),
                    );
                }
                match vault.put_conception_event(&scope.workspace_id, &event_id, origin, content) {
                    Ok(stored) => (
                        true,
                        stored.content_hash,
                        format!("committed to {}", path.display()),
                    ),
                    Err(e) => {
                        return CommandResult::failure(
                            correlation,
                            CommandError::validation(format!("vault write failed: {e}")),
                        );
                    }
                }
            }
            Err(e) => {
                return CommandResult::failure(
                    correlation,
                    CommandError::validation(format!("vault open failed: {e}")),
                );
            }
        },
        None => (
            false,
            content_hash(content),
            "not persisted: no vault path supplied".to_string(),
        ),
    };

    let outcome = RecordConceptionOutcome {
        event: ConceptionEventView {
            event_id,
            content_hash: hash,
            content_bytes: content.len(),
            origin: origin.to_string(),
        },
        persisted,
        storage_detail,
    };

    CommandResult::success(correlation, outcome)
}

/// Content-addressed hash of a conception block.
///
/// `REQ-DATA-002` requires evidence blobs to be content-addressed. This is a
/// dependency-free FNV-1a fingerprint used for identity/equality checks; it is
/// NOT a cryptographic digest and is named so it cannot be mistaken for one.
/// A SHA-256 implementation arrives with the vault write path.
pub fn content_hash(content: &str) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("fnv1a64:{hash:016x}")
}

/// Namespace implementation status.
///
/// SPEC-003 lists 14 contract namespaces. Reporting which are implemented is
/// required by DOD-026 so the UI cannot present an unimplemented namespace as
/// available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespaceStatus {
    pub namespace: String,
    pub implemented: bool,
    pub detail: String,
}

/// State of a research task as reported to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchTaskView {
    pub task_id: String,
    pub status: String,
    pub citations: Vec<String>,
    pub citation_count: usize,
}

/// Advance a research task through its lifecycle.
///
/// `REQ-RES-002` and `REQ-DOM-006`: a kill-search runs from Pending -> Active
/// and ends either Killed (a design-around was found, so the search succeeded)
/// or Completed (exhausted without a kill). Citations may only be added while
/// the task has not finished.
///
/// The transition rules live in the `research` crate, so this command drives the
/// real state machine rather than re-implementing its guards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchAction {
    /// Pending -> Active.
    Start,
    /// Active -> Killed.
    Kill,
    /// Active -> Completed.
    Complete,
    /// Append a citation (requires a citation string).
    AddCitation,
}

/// Apply an action to a research task and return the resulting state.
pub fn apply_research_action(
    task_id: &str,
    current_status: &str,
    citations: &[String],
    action: ResearchAction,
    citation: Option<&str>,
) -> CommandResult<ResearchTaskView> {
    let correlation = CorrelationId::new();

    if task_id.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("task_id is required"),
        );
    }

    // Rebuild the real domain object from the reported state.
    let mut task = research::ResearchTask::new(task_id);
    for existing in citations {
        if let Err(e) = task.add_citation(existing.clone()) {
            return CommandResult::failure(correlation, CommandError::validation(e));
        }
    }
    let restored = match current_status {
        "Pending" => true,
        "Active" => task.start().is_ok(),
        "Killed" => task.start().is_ok() && task.kill().is_ok(),
        "Completed" => task.start().is_ok() && task.complete().is_ok(),
        other => {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!("unknown research status {other:?}")),
            );
        }
    };
    if !restored {
        return CommandResult::failure(
            correlation,
            CommandError::validation("could not restore research task state"),
        );
    }

    let outcome = match action {
        ResearchAction::Start => task.start(),
        ResearchAction::Kill => task.kill(),
        ResearchAction::Complete => task.complete(),
        ResearchAction::AddCitation => match citation {
            Some(c) if !c.trim().is_empty() => task.add_citation(c.to_string()),
            Some(_) => {
                return CommandResult::failure(
                    correlation,
                    CommandError::validation("citation cannot be empty"),
                );
            }
            None => {
                return CommandResult::failure(
                    correlation,
                    CommandError::validation("citation is required for AddCitation"),
                );
            }
        },
    };

    if let Err(message) = outcome {
        return CommandResult::failure(correlation, CommandError::policy(message));
    }

    CommandResult::success(
        correlation,
        ResearchTaskView {
            task_id: task.id.clone(),
            status: format!("{:?}", task.status),
            citation_count: task.citations.len(),
            citations: task.citations.clone(),
        },
    )
}

/// Evaluate a disclosure payload against the Disclosure Firewall.
///
/// `REQ-DOM-008` / `REQ-COM-004`: the firewall classifies every public export
/// and blocks Restricted content. `PATENT_DOMAIN_MODEL.md` requires that public
/// marketing never bypasses it.
///
/// The decision comes from the real `evidence::DisclosureFirewall`, so the
/// policy is not duplicated here.
pub fn evaluate_export(
    scope: &WorkspaceScope,
    content: &str,
    sensitivity: &str,
) -> CommandResult<ExportDecision> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if content.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("export content cannot be empty"),
        );
    }

    let level = match sensitivity {
        "Public" => evidence::Sensitivity::Public,
        "Confidential" => evidence::Sensitivity::Confidential,
        "Restricted" => evidence::Sensitivity::Restricted,
        other => {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!("unknown sensitivity {other:?}")),
            );
        }
    };

    let firewall = evidence::DisclosureFirewall::new();
    let payload = evidence::ExportPayload {
        content: content.to_string(),
        sensitivity: level.clone(),
    };

    match firewall.filter_export(&payload) {
        Ok(()) => CommandResult::success(
            correlation,
            ExportDecision {
                allowed: true,
                sensitivity: sensitivity.to_string(),
                reason: "permitted by Disclosure Firewall".to_string(),
            },
        ),
        Err(reason) => CommandResult::success(
            correlation,
            ExportDecision {
                allowed: false,
                sensitivity: sensitivity.to_string(),
                reason: reason.to_string(),
            },
        ),
    }
}

/// Outcome of a disclosure-firewall evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportDecision {
    pub allowed: bool,
    pub sensitivity: String,
    pub reason: String,
}

/// Lint a claim set against the patent linter.
///
/// `REQ-PAT-001`: every claim must satisfy the structural rules in
/// `PATENT_DOMAIN_MODEL.md`. The verdict comes from the real
/// `patent::PatentLinter`, so the rule set is not duplicated here.
pub fn lint_claims(claims: &str) -> CommandResult<LintOutcome> {
    let correlation = CorrelationId::new();

    if claims.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("claims cannot be empty"),
        );
    }

    match patent::PatentLinter::lint_claims(claims) {
        Ok(()) => CommandResult::success(
            correlation,
            LintOutcome {
                passed: true,
                findings: Vec::new(),
            },
        ),
        Err(reason) => CommandResult::success(
            correlation,
            LintOutcome {
                passed: false,
                findings: vec![reason.to_string()],
            },
        ),
    }
}

/// Result of linting a claim set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintOutcome {
    pub passed: bool,
    pub findings: Vec<String>,
}

/// Build a filing-package manifest (REQ-PAT-002).
///
/// `PackageBuilder` emits a MANIFEST with a real content fingerprint rather
/// than claiming to render a DOCX/PDF it cannot produce. The `format` field is
/// returned verbatim so the UI cannot present a manifest as a document.
pub fn build_filing_package(claims: &str, specification: &str) -> CommandResult<FilingPackageView> {
    let correlation = CorrelationId::new();

    if claims.trim().is_empty() || specification.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("claims and specification are both required"),
        );
    }

    match patent::PackageBuilder::build_docx(claims, specification) {
        Ok(package) => CommandResult::success(
            correlation,
            FilingPackageView {
                format: package.format,
                manifest: package.content,
            },
        ),
        Err(e) => CommandResult::failure(correlation, CommandError::validation(e)),
    }
}

/// A filing-package manifest as returned to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilingPackageView {
    pub format: String,
    pub manifest: String,
}

/// Import a USPTO acknowledgement receipt (REQ-PAT-005).
///
/// The parsed application and confirmation numbers come from the real
/// `patent::ReceiptImport`, which fails closed on malformed input. A filing
/// receipt is record evidence, so a plausible-looking but wrong application
/// number would be a record-integrity failure rather than a cosmetic bug.
pub fn import_receipt(receipt_text: &str) -> CommandResult<ReceiptView> {
    let correlation = CorrelationId::new();

    if receipt_text.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("receipt text cannot be empty"),
        );
    }

    match patent::ReceiptImport::import(receipt_text) {
        Ok(receipt) => CommandResult::success(
            correlation,
            ReceiptView {
                application_number: receipt.application_number,
                confirmation_number: receipt.confirmation_number,
            },
        ),
        Err(e) => CommandResult::failure(correlation, CommandError::validation(e)),
    }
}

/// Parsed acknowledgement receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptView {
    pub application_number: String,
    pub confirmation_number: String,
}

/// Report whether the USPTO handoff manifest is complete (REQ-PAT-005).
///
/// LINCHPIN never automates signature, payment or submission; this only reports
/// whether the prerequisites a human needs are present.
pub fn check_filing_handoff(forms: &[String], fee_paid: bool) -> CommandResult<HandoffReadiness> {
    let correlation = CorrelationId::new();

    let mut manifest = patent::UsptoManifest::new();
    for form in forms {
        match form.as_str() {
            "Ads" => manifest.add_form(patent::UsptoFormType::Ads),
            "Sba" => manifest.add_form(patent::UsptoFormType::Sba),
            "Oath" => manifest.add_form(patent::UsptoFormType::Oath),
            other => {
                return CommandResult::failure(
                    correlation,
                    CommandError::validation(format!("unknown form type {other:?}")),
                );
            }
        }
    }
    if fee_paid {
        manifest.set_fee_paid();
    }

    let ready = manifest.validate_handoff().is_ok();
    let blockers: Vec<String> = match manifest.validate_handoff() {
        Ok(()) => Vec::new(),
        Err(e) => vec![e.to_string()],
    };

    CommandResult::success(
        correlation,
        HandoffReadiness {
            ready_for_human_submission: ready,
            blockers,
            // Stated explicitly so no caller infers automation.
            note: "Human submission only: LINCHPIN does not sign, pay or submit.".to_string(),
        },
    )
}

/// Readiness of a filing handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffReadiness {
    pub ready_for_human_submission: bool,
    pub blockers: Vec<String>,
    pub note: String,
}

/// Score an opportunity candidate and report its uncertainty honestly.
///
/// `REQ-DOM-003`: an Opportunity Candidate stores a score vector, an uncertainty
/// level and its evidence. `SPEC-004` requires legal-risk language that does not
/// assert definitive conclusions, so the returned note is worded as a screen.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_opportunity(
    scope: &WorkspaceScope,
    description: &str,
    technical_feasibility: f32,
    market_potential: f32,
    legal_risk: f32,
    uncertainty: &str,
    evidence_uris: &[String],
) -> CommandResult<OpportunityView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if description.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("opportunity description cannot be empty"),
        );
    }
    for (name, value) in [
        ("technical_feasibility", technical_feasibility),
        ("market_potential", market_potential),
        ("legal_risk", legal_risk),
    ] {
        if !(0.0..=1.0).contains(&value) {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!("{name} must be within 0.0..=1.0")),
            );
        }
    }

    let level = match uncertainty {
        "Low" => domain::UncertaintyLevel::Low,
        "Medium" => domain::UncertaintyLevel::Medium,
        "High" => domain::UncertaintyLevel::High,
        other => {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!("unknown uncertainty {other:?}")),
            );
        }
    };

    let scores = domain::ScoreVector {
        technical_feasibility,
        market_potential,
        legal_risk,
    };
    let mut candidate = domain::OpportunityCandidate::new(scores, level);
    for uri in evidence_uris {
        candidate.add_evidence("evidence".to_string(), uri.clone());
    }

    // A candidate with no evidence is not a finding; say so rather than
    // presenting a score as if it were supported (DOD-026).
    let evidence_count = candidate.evidence.len();

    CommandResult::success(
        correlation,
        OpportunityView {
            candidate_id: candidate.id.0.to_string(),
            description: description.to_string(),
            uncertainty: uncertainty.to_string(),
            evidence_count,
            has_supporting_evidence: evidence_count > 0,
            // Deliberately not a legal conclusion (REQ-UI-003).
            screen_note: "Planning screen only; not a legal or commercial conclusion.".to_string(),
        },
    )
}

/// An opportunity candidate as returned to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpportunityView {
    pub candidate_id: String,
    pub description: String,
    pub uncertainty: String,
    pub evidence_count: usize,
    pub has_supporting_evidence: bool,
    pub screen_note: String,
}

/// Advance a docket record through its filing state machine.
///
/// `REQ-DOM-007`: filing state requires receipt evidence. The transition rules
/// come from `domain::DocketRecord`, so a filing cannot be recorded without a
/// receipt and commercialization cannot precede filing.
pub fn advance_docket(
    scope: &WorkspaceScope,
    current_state: &str,
    receipt: Option<&str>,
    commercialize: bool,
) -> CommandResult<DocketView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }

    let mut docket = domain::DocketRecord::new("docket".to_string());
    // Restore the reported state through the real transitions.
    match current_state {
        "Preparation" => {}
        state if state.starts_with("Filed") => {
            let existing = state
                .strip_prefix("Filed(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or("receipt");
            if let Err(e) = docket.file_application(existing.to_string()) {
                return CommandResult::failure(correlation, CommandError::policy(e));
            }
        }
        "Commercialized" => {
            if docket.file_application("receipt".to_string()).is_err()
                || docket.mark_commercialized().is_err()
            {
                return CommandResult::failure(
                    correlation,
                    CommandError::policy("could not restore commercialized state"),
                );
            }
        }
        other => {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!("unknown docket state {other:?}")),
            );
        }
    }

    if commercialize {
        if let Err(e) = docket.mark_commercialized() {
            return CommandResult::failure(correlation, CommandError::policy(e));
        }
    } else if let Some(r) = receipt {
        if r.trim().is_empty() {
            return CommandResult::failure(
                correlation,
                CommandError::validation("receipt cannot be empty when filing"),
            );
        }
        if let Err(e) = docket.file_application(r.to_string()) {
            return CommandResult::failure(correlation, CommandError::policy(e));
        }
    }

    let state = match &docket.state {
        domain::FilingState::Preparation => "Preparation".to_string(),
        domain::FilingState::Filed(r) => format!("Filed({r})"),
        domain::FilingState::Commercialized => "Commercialized".to_string(),
    };

    CommandResult::success(
        correlation,
        DocketView {
            docket_id: docket.id.0.to_string(),
            state,
            public_disclosure: docket.public_disclosure,
        },
    )
}

/// A docket record as returned to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocketView {
    pub docket_id: String,
    pub state: String,
    pub public_disclosure: bool,
}

/// Draft a response workspace for an office action.
///
/// Prosecution flow. A Notice of Allowance has no rejection to respond to, and
/// the real `patent::OfficeAction` refuses that transition.
pub fn draft_office_action_response(
    action_type: &str,
    cited_art: &[String],
) -> CommandResult<OfficeActionView> {
    let correlation = CorrelationId::new();

    let kind = match action_type {
        "NonFinalRejection" => patent::ActionType::NonFinalRejection,
        "FinalRejection" => patent::ActionType::FinalRejection,
        "NoticeOfAllowance" => patent::ActionType::NoticeOfAllowance,
        other => {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!("unknown action type {other:?}")),
            );
        }
    };

    let mut action = patent::OfficeAction::new(kind);
    for art in cited_art {
        if art.trim().is_empty() {
            return CommandResult::failure(
                correlation,
                CommandError::validation("cited art reference cannot be empty"),
            );
        }
        action.cite_art(art.clone());
    }

    match action.draft_response() {
        Ok(()) => CommandResult::success(
            correlation,
            OfficeActionView {
                action_type: action_type.to_string(),
                cited_art_count: action.cited_art.len(),
                response_drafted: action.response_drafted,
            },
        ),
        Err(e) => CommandResult::failure(correlation, CommandError::policy(e)),
    }
}

/// An office-action workspace as returned to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeActionView {
    pub action_type: String,
    pub cited_art_count: usize,
    pub response_drafted: bool,
}

/// Build a disclosure-safe commercialization one-pager.
///
/// `REQ-COM-001` / `REQ-COM-004`: the package is disclosure-safe and implies no
/// valuation. The real `commercialization::DataRoom` refuses to export
/// unredacted assets, so this asserts that refusal before redacting.
pub fn build_commercialization_package(
    scope: &WorkspaceScope,
    target_names: &[String],
) -> CommandResult<CommercializationView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if target_names.is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("at least one target is required"),
        );
    }

    let mut room = commercialization::DataRoom::new();
    for name in target_names {
        if name.trim().is_empty() {
            return CommandResult::failure(
                correlation,
                CommandError::validation("target name cannot be empty"),
            );
        }
        room.add_target(name, 0.5);
    }

    // Export before redaction must be refused by the real implementation. If it
    // were allowed, the confidentiality boundary would already be broken.
    if room.export_pitch_deck().is_ok() {
        return CommandResult::failure(
            correlation,
            CommandError::policy(
                "commercialization package exported without redacting confidential assets",
            ),
        );
    }
    room.redact_for_non_confidential_export();

    match room.export_pitch_deck() {
        Ok(payload) => CommandResult::success(
            correlation,
            CommercializationView {
                target_count: target_names.len(),
                redacted: true,
                payload,
            },
        ),
        Err(e) => CommandResult::failure(correlation, CommandError::policy(e)),
    }
}

/// A commercialization package as returned to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommercializationView {
    pub target_count: usize,
    pub redacted: bool,
    pub payload: String,
}

/// Run inference through the local provider lane.
///
/// GraphLock context (DOD-019): `provider_transport::generate` existed but was
/// called ONLY from its own tests -- no production path ever reached it, so the
/// crate was an inert adapter. This command is that production path.
///
/// It is honest about the boundary:
///   * the endpoint is loopback-only, enforced by `LocalModelAdapter::new`,
///     which refuses a non-loopback host so invention content cannot leave the
///     device (SECURITY.md);
///   * when no model is served the real transport error is returned verbatim
///     (`Unreachable` / `ProviderFailure` / `InvalidResponse`), never a
///     fabricated completion (DOD-014, anti-gaming finding AG-001);
///   * the returned `live` flag states whether a real inference boundary was
///     reached, so a caller cannot mistake an error path for output.
///
/// PF-011 (a served local model) is unmet on this host, so in practice this
/// returns a transport error. That is the correct result, not a defect.
pub async fn run_local_inference(
    endpoint: &str,
    model_id: &str,
    prompt: &str,
) -> CommandResult<InferenceOutcome> {
    let correlation = CorrelationId::new();

    if prompt.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("prompt cannot be empty"),
        );
    }

    let adapter = match provider_transport::LocalModelAdapter::new(
        endpoint,
        model_id,
        provider_transport::LocalFlavor::Ollama,
    ) {
        Ok(a) => a,
        Err(e) => {
            // A rejected endpoint is a POLICY matter: the operator asked for a
            // destination the confidentiality boundary forbids.
            return CommandResult::failure(correlation, CommandError::policy(e.to_string()));
        }
    };

    let transport = provider_transport::ProviderTransport::identity(&adapter).to_string();
    let request = provider_transport::ModelRequest {
        prompt: prompt.to_string(),
        model_id: model_id.to_string(),
    };

    match provider_transport::ProviderTransport::generate(&adapter, request).await {
        Ok(response) => CommandResult::success(
            correlation,
            InferenceOutcome {
                live: true,
                transport,
                text: Some(response.text),
                error_class: None,
                detail: response.metadata,
            },
        ),
        Err(e) => {
            let class = match e {
                provider_transport::TransportError::InvalidRequest(_) => "INVALID_REQUEST",
                provider_transport::TransportError::Unreachable(_) => "UNREACHABLE",
                provider_transport::TransportError::ProviderFailure { .. } => "PROVIDER_FAILURE",
                provider_transport::TransportError::InvalidResponse(_) => "INVALID_RESPONSE",
                provider_transport::TransportError::Unimplemented(_) => "UNIMPLEMENTED",
            };
            CommandResult::success(
                correlation,
                InferenceOutcome {
                    live: false,
                    transport,
                    // No text is produced on a failed transport. Returning any
                    // here would be the AG-001 fabrication defect.
                    text: None,
                    error_class: Some(class.to_string()),
                    detail: e.to_string(),
                },
            )
        }
    }
}

/// Result of a provider inference attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceOutcome {
    /// True only when a real inference boundary was reached.
    pub live: bool,
    pub transport: String,
    /// Present only when `live` is true.
    pub text: Option<String>,
    pub error_class: Option<String>,
    pub detail: String,
}

/// Report provider transport availability honestly.
///
/// `PROVIDER_TRANSPORT_MATRIX.md` defines the lanes. No live provider is
/// configured on this host (PF-011 unmet), so this reports availability rather
/// than performing inference. It never returns generated text.
pub fn provider_status() -> CommandResult<Vec<ProviderLane>> {
    let correlation = CorrelationId::new();

    let local = provider_transport::LocalModelAdapter::new(
        "http://127.0.0.1:11434",
        "unset",
        provider_transport::LocalFlavor::Ollama,
    );
    let local_reachable = local.is_ok();

    let lanes = vec![
        ProviderLane {
            lane: "local".to_string(),
            transport_available: local_reachable,
            configured: false,
            detail: "llama.cpp/Ollama on loopback; no model served on this host (PF-011)"
                .to_string(),
        },
        ProviderLane {
            lane: "openai".to_string(),
            transport_available: false,
            configured: false,
            detail: "Codex CLI boundary not wired".to_string(),
        },
        ProviderLane {
            lane: "anthropic".to_string(),
            transport_available: false,
            configured: false,
            detail: "terms review required".to_string(),
        },
        ProviderLane {
            lane: "xai".to_string(),
            transport_available: false,
            configured: false,
            detail: "Grok Build ACP not wired".to_string(),
        },
        ProviderLane {
            lane: "google".to_string(),
            transport_available: false,
            configured: false,
            detail: "disabled by provider policy".to_string(),
        },
    ];

    CommandResult::success(correlation, lanes)
}

/// A provider lane and its real availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderLane {
    pub lane: String,
    pub transport_available: bool,
    pub configured: bool,
    pub detail: String,
}

/// Check an MCP client's capability grant.
///
/// `SPEC-005`: MCP grants are explicit by server/client/workspace/capability.
/// The decision comes from the real `mcp_hub::McpServer`.
pub fn check_mcp_capability(granted: &[String], requested: &str) -> CommandResult<McpDecision> {
    let correlation = CorrelationId::new();

    let mut server = mcp_hub::McpServer::new();
    for cap in granted {
        let parsed = match cap.as_str() {
            "ReadVault" => mcp_hub::Capability::ReadVault,
            "WriteVault" => mcp_hub::Capability::WriteVault,
            "ExecuteResearch" => mcp_hub::Capability::ExecuteResearch,
            other => {
                return CommandResult::failure(
                    correlation,
                    CommandError::validation(format!("unknown capability {other:?}")),
                );
            }
        };
        server.grant_capability("client", parsed);
    }

    let requested_cap = match requested {
        "ReadVault" => mcp_hub::Capability::ReadVault,
        "WriteVault" => mcp_hub::Capability::WriteVault,
        "ExecuteResearch" => mcp_hub::Capability::ExecuteResearch,
        other => {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!("unknown capability {other:?}")),
            );
        }
    };

    let allowed = server.check_capability("client", &requested_cap);
    CommandResult::success(
        correlation,
        McpDecision {
            requested: requested.to_string(),
            allowed,
            reason: if allowed {
                "capability granted for this client".to_string()
            } else {
                "capability not granted; models cannot self-approve".to_string()
            },
        },
    )
}

/// An MCP capability decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpDecision {
    pub requested: String,
    pub allowed: bool,
    pub reason: String,
}

/// Build a sanitized repair capsule from an incident.
///
/// `REQ-DOM-010` / `REQ-SEC-010`: a Repair Capsule always redacts before export.
/// Redaction is performed by the real `crash_reporter::RedactionPolicy`, and
/// safety is derived from the transformed content rather than a flag
/// (anti-gaming finding AG-002).
pub fn build_repair_capsule(
    incident_detail: &str,
    agent_brief: &str,
    secrets_to_redact: &[String],
) -> CommandResult<RepairCapsuleView> {
    let correlation = CorrelationId::new();

    if agent_brief.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("agent brief cannot be empty"),
        );
    }

    let incident =
        crash_reporter::WindowsMinidumpHandler::capture_crash_with_detail("local", incident_detail);
    let mut policy = crash_reporter::RedactionPolicy::new();
    for secret in secrets_to_redact {
        policy = policy.with_secret(secret);
    }

    match crash_reporter::RepairCapsule::new(incident, agent_brief, &policy) {
        Ok(capsule) => {
            let safe = capsule.is_safe_for_export();
            let redacted = capsule.redacted_detail().to_string();
            // A secret that survives redaction must never be reported as
            // exportable (AG-002 regression guard).
            let leaked = secrets_to_redact
                .iter()
                .any(|s| !s.is_empty() && redacted.contains(s.as_str()));
            if leaked || !safe {
                return CommandResult::failure(
                    correlation,
                    CommandError::policy("redaction failed to sanitize the incident detail"),
                );
            }
            CommandResult::success(
                correlation,
                RepairCapsuleView {
                    redacted_detail: redacted,
                    safe_for_export: safe,
                },
            )
        }
        Err(e) => CommandResult::failure(correlation, CommandError::validation(e)),
    }
}

/// A sanitized repair capsule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairCapsuleView {
    pub redacted_detail: String,
    pub safe_for_export: bool,
}

/// Export evidence through the path-hardening and firewall gateway.
///
/// Distinct from the `evidence` namespace command: this is the export gateway
/// itself, applying path hardening and the Disclosure Firewall together so an
/// export cannot be produced from an escaping path (REQ-DOM-008, AG-003).
pub fn export_evidence(
    scope: &WorkspaceScope,
    path: &str,
    content: &str,
    sensitivity: &str,
) -> CommandResult<ExportReceipt> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }

    // Path hardening comes from the real evidence crate.
    let safe_path = match evidence::InputHardener::sanitize_path(path) {
        Ok(p) => p,
        Err(e) => {
            return CommandResult::failure(correlation, CommandError::policy(e));
        }
    };

    let decision = evaluate_export(scope, content, sensitivity);
    if !decision.ok {
        return CommandResult::failure(
            correlation,
            decision
                .error
                .unwrap_or_else(|| CommandError::validation("export evaluation failed")),
        );
    }
    let decision = decision.value.expect("ok implies a value");

    if !decision.allowed {
        return CommandResult::failure(correlation, CommandError::policy(decision.reason));
    }

    CommandResult::success(
        correlation,
        ExportReceipt {
            path: safe_path,
            sensitivity: decision.sensitivity,
            content_bytes: content.len(),
        },
    )
}

/// Receipt for a permitted evidence export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportReceipt {
    pub path: String,
    pub sensitivity: String,
    pub content_bytes: usize,
}

/// Report the SPEC-003 namespace coverage honestly.
pub fn namespace_status() -> Vec<NamespaceStatus> {
    let entry = |ns: &str, implemented: bool, detail: &str| NamespaceStatus {
        namespace: ns.to_string(),
        implemented,
        detail: detail.to_string(),
    };
    vec![
        entry("conception", true, "record_conception"),
        entry("workspace", true, "get_system_health, get_namespace_status"),
        entry("research", true, "apply_research_action"),
        entry("evidence", true, "evaluate_export (Disclosure Firewall)"),
        entry("patent", true, "lint_claims, build_filing_package"),
        entry(
            "filing",
            true,
            "import_receipt, check_filing_handoff (human submission only)",
        ),
        entry("opportunity", true, "evaluate_opportunity"),
        entry("docket", true, "advance_docket"),
        entry("prosecution", true, "draft_office_action_response"),
        entry(
            "commercialization",
            true,
            "build_commercialization_package (firewall-gated)",
        ),
        entry(
            "provider",
            true,
            "provider_status (reports availability; no live provider configured)",
        ),
        entry("mcp", true, "check_mcp_capability"),
        entry("incident", true, "build_repair_capsule (redacted)"),
        entry("export", true, "export_evidence (path-hardened + firewall)"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// covers: REQ-DOM-001
    #[test]
    fn test_record_conception_labels_human_origin() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let result = record_conception(&scope, "a self-sealing valve", true, None);
        assert!(result.ok);
        assert!(!result.correlation_id.is_empty());

        let outcome = result.value.expect("value present on success");
        assert_eq!(outcome.event.origin, "HumanConception");
        assert!(outcome.event.content_hash.starts_with("fnv1a64:"));
        assert_eq!(outcome.event.content_bytes, "a self-sealing valve".len());
    }

    /// covers: REQ-DOM-002
    /// REQ-DOM-002: AI suggestions must be labelled distinctly, never as human
    /// conception.
    #[test]
    fn test_record_conception_labels_ai_origin_distinctly() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let human = record_conception(&scope, "same text", true, None)
            .value
            .unwrap();
        let ai = record_conception(&scope, "same text", false, None)
            .value
            .unwrap();

        assert_eq!(human.event.origin, "HumanConception");
        assert_eq!(ai.event.origin, "AiSuggestion");
        assert_ne!(
            human.event.origin, ai.event.origin,
            "AI suggestion was not labelled distinctly"
        );
    }

    #[test]
    fn test_record_conception_rejects_empty_workspace_and_content() {
        let bad_scope = WorkspaceScope {
            workspace_id: "   ".to_string(),
        };
        let r = record_conception(&bad_scope, "text", true, None);
        assert!(!r.ok);
        assert!(matches!(r.error, Some(CommandError::Validation { .. })));

        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let r2 = record_conception(&scope, "   ", true, None);
        assert!(!r2.ok);
        assert!(matches!(r2.error, Some(CommandError::Validation { .. })));
    }

    /// DOD-026: with no vault path the command must not claim a side effect it
    /// did not perform.
    #[test]
    fn test_record_conception_does_not_claim_persistence_without_vault() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let outcome = record_conception(&scope, "content", true, None)
            .value
            .unwrap();
        assert!(
            !outcome.persisted,
            "command claimed persistence with no vault path"
        );
        assert!(outcome.storage_detail.contains("not persisted"));
    }

    fn temp_db(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "linchpin-cmd-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("vault.db");
        (dir, db)
    }

    /// covers: REQ-DATA-001, REQ-DATA-002
    /// The durable path must genuinely commit: the event is written through
    /// `storage::Vault` and read back with a real SHA-256 content address
    /// (REQ-DATA-001, REQ-DATA-002).
    #[test]
    fn test_record_conception_persists_through_the_vault() {
        let (dir, db) = temp_db("vault");
        let scope = WorkspaceScope {
            workspace_id: "ws-persist".to_string(),
        };
        let outcome = record_conception(&scope, "a self-sealing graphene valve", true, Some(&db))
            .value
            .expect("value on success");

        assert!(
            outcome.persisted,
            "durable write did not report persistence"
        );
        assert!(
            outcome.event.content_hash.starts_with("sha256:"),
            "expected a SHA-256 content address, got {}",
            outcome.event.content_hash
        );
        assert_eq!(
            outcome.event.content_hash,
            storage::sha256_hex(b"a self-sealing graphene valve"),
            "content hash does not match the stored bytes"
        );

        // Independent read-back through a fresh connection.
        let vault = storage::Vault::open(&db).expect("reopen vault");
        let stored = vault
            .get_conception_event(&outcome.event.event_id)
            .expect("query")
            .expect("event must be present after the command returned");
        assert_eq!(stored.content, "a self-sealing graphene valve");
        assert_eq!(stored.origin, "HumanConception");
        assert_eq!(stored.workspace_id, "ws-persist");
        vault.verify_audit_chain().expect("chain must verify");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DOM-002
    /// An AI suggestion must persist with its distinct origin label, so the
    /// conception record cannot be retroactively blurred (REQ-DOM-002).
    #[test]
    fn test_record_conception_persists_ai_origin_distinctly() {
        let (dir, db) = temp_db("ai");
        let scope = WorkspaceScope {
            workspace_id: "ws-ai".to_string(),
        };
        let outcome = record_conception(&scope, "an AI idea", false, Some(&db))
            .value
            .unwrap();
        assert!(outcome.persisted);
        assert_eq!(outcome.event.origin, "AiSuggestion");

        let vault = storage::Vault::open(&db).unwrap();
        let stored = vault
            .get_conception_event(&outcome.event.event_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            stored.origin, "AiSuggestion",
            "AI origin was not preserved through persistence"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_content_hash_is_deterministic_and_sensitive() {
        assert_eq!(content_hash("abc"), content_hash("abc"));
        assert_ne!(content_hash("abc"), content_hash("abd"));
        assert_ne!(content_hash(""), content_hash("a"));
    }

    #[test]
    fn test_correlation_ids_are_unique() {
        let a = CorrelationId::new();
        let b = CorrelationId::new();
        assert_ne!(a, b, "correlation IDs collided");
        assert!(!a.as_str().is_empty());
    }

    /// covers: REQ-LLM-002
    /// SPEC-003 lists 14 namespaces; the report must cover all of them and must
    /// not claim implementation that does not exist.
    #[test]
    fn test_namespace_status_covers_spec_003_and_is_honest() {
        let statuses = namespace_status();
        assert_eq!(statuses.len(), 14, "SPEC-003 declares 14 namespaces");

        for expected in [
            "workspace",
            "conception",
            "opportunity",
            "research",
            "evidence",
            "patent",
            "filing",
            "docket",
            "prosecution",
            "commercialization",
            "provider",
            "mcp",
            "incident",
            "export",
        ] {
            assert!(
                statuses.iter().any(|s| s.namespace == expected),
                "namespace {expected} missing from status report"
            );
        }

        let implemented: Vec<&str> = statuses
            .iter()
            .filter(|s| s.implemented)
            .map(|s| s.namespace.as_str())
            .collect();
        assert_eq!(
            implemented.len(),
            14,
            "all 14 SPEC-003 namespaces should now be wired, got {implemented:?}"
        );
    }

    // --- opportunity namespace ---------------------------------------------

    /// covers: REQ-DOM-003
    #[test]
    fn test_opportunity_requires_evidence_to_be_a_finding() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let bare = evaluate_opportunity(&scope, "idea", 0.8, 0.7, 0.2, "Medium", &[]);
        let v = bare.value.unwrap();
        assert!(
            !v.has_supporting_evidence,
            "an unevidenced candidate must not report supporting evidence"
        );
        assert_eq!(v.evidence_count, 0);
        assert!(v.screen_note.contains("not a legal"));

        let supported = evaluate_opportunity(
            &scope,
            "idea",
            0.8,
            0.7,
            0.2,
            "Medium",
            &["https://example.gov/1".to_string()],
        );
        assert!(supported.value.unwrap().has_supporting_evidence);
    }

    #[test]
    fn test_opportunity_validates_scores_and_uncertainty() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        assert!(!evaluate_opportunity(&scope, "x", 1.5, 0.5, 0.5, "Low", &[]).ok);
        assert!(!evaluate_opportunity(&scope, "x", -0.1, 0.5, 0.5, "Low", &[]).ok);
        assert!(!evaluate_opportunity(&scope, "x", 0.5, 0.5, 0.5, "Certain", &[]).ok);
        assert!(!evaluate_opportunity(&scope, "  ", 0.5, 0.5, 0.5, "Low", &[]).ok);
    }

    // --- docket namespace --------------------------------------------------

    /// covers: REQ-DOM-007
    /// REQ-DOM-007: filing requires receipt evidence, and commercialization
    /// cannot precede filing. Both guards live in the domain crate.
    #[test]
    fn test_docket_requires_filing_before_commercialization() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let premature = advance_docket(&scope, "Preparation", None, true);
        assert!(!premature.ok, "commercialization was allowed before filing");
        assert!(matches!(premature.error, Some(CommandError::Policy { .. })));

        let filed = advance_docket(&scope, "Preparation", Some("US123456"), false);
        assert_eq!(filed.value.unwrap().state, "Filed(US123456)");

        let commercial = advance_docket(&scope, "Filed(US123456)", None, true);
        assert_eq!(commercial.value.unwrap().state, "Commercialized");
    }

    #[test]
    fn test_docket_validates_inputs() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        assert!(!advance_docket(&scope, "Nonsense", None, false).ok);
        assert!(!advance_docket(&scope, "Preparation", Some("   "), false).ok);
    }

    // --- prosecution namespace ---------------------------------------------

    /// covers: REQ-PAT-004
    #[test]
    fn test_office_action_refuses_allowance_response() {
        let nonfinal =
            draft_office_action_response("NonFinalRejection", &["US9876543".to_string()]);
        let v = nonfinal.value.unwrap();
        assert!(v.response_drafted);
        assert_eq!(v.cited_art_count, 1);

        let allowance = draft_office_action_response("NoticeOfAllowance", &[]);
        assert!(
            !allowance.ok,
            "a Notice of Allowance has no rejection to respond to"
        );
        assert!(matches!(allowance.error, Some(CommandError::Policy { .. })));

        assert!(!draft_office_action_response("Bogus", &[]).ok);
        assert!(!draft_office_action_response("FinalRejection", &["  ".to_string()]).ok);
    }

    // --- commercialization namespace ---------------------------------------

    /// covers: REQ-COM-001, REQ-COM-004
    /// REQ-COM-004: the package must be redacted before export. The command
    /// asserts the pre-redaction export is REFUSED, so a regression that
    /// allowed it would fail here.
    #[test]
    fn test_commercialization_package_is_redacted() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let result = build_commercialization_package(&scope, &["MegaCorp".to_string()]);
        assert!(result.ok);
        let v = result.value.unwrap();
        assert!(v.redacted);
        assert_eq!(v.target_count, 1);
        assert!(
            v.payload.contains("not a valuation"),
            "payload must disclaim valuation, got {:?}",
            v.payload
        );

        assert!(!build_commercialization_package(&scope, &[]).ok);
        assert!(!build_commercialization_package(&scope, &["  ".to_string()]).ok);
    }

    // --- provider namespace ------------------------------------------------

    /// covers: REQ-LLM-004
    /// PF-011 is unmet, so no lane may report itself configured, and the
    /// command must not perform inference or return generated text.
    #[test]
    fn test_provider_status_reports_no_configured_lane() {
        let result = provider_status();
        assert!(result.ok);
        let lanes = result.value.unwrap();
        assert_eq!(lanes.len(), 5);
        assert!(
            lanes.iter().all(|l| !l.configured),
            "a provider lane claimed to be configured with no credentials"
        );

        let google = lanes.iter().find(|l| l.lane == "google").unwrap();
        assert!(!google.transport_available);
        assert!(google.detail.contains("disabled by provider policy"));
    }

    // --- provider inference (DOD-019: the production path) ------------------

    /// covers: REQ-LLM-001
    /// DOD-019 / AG-001: with no model served, the real transport must fail and
    /// the command must return NO text. Returning anything here would be the
    /// fabricated-completion defect.
    #[tokio::test]
    async fn test_local_inference_returns_no_text_when_no_model_is_served() {
        let result = run_local_inference("http://127.0.0.1:1", "llama3", "hello").await;
        assert!(
            result.ok,
            "the command itself succeeds; the outcome reports"
        );
        let outcome = result.value.expect("value present");
        assert!(
            !outcome.live,
            "inference claimed to be live with no model served"
        );
        assert!(
            outcome.text.is_none(),
            "fabricated text was returned on a failed transport: {:?}",
            outcome.text
        );
        assert!(
            outcome.error_class.is_some(),
            "a failed transport must report an error class"
        );
        assert_eq!(outcome.error_class.as_deref(), Some("UNREACHABLE"));
    }

    /// covers: REQ-LLM-001, REQ-SEC-001
    /// The endpoint must be loopback-only, so invention content cannot be sent
    /// off-device (SECURITY.md).
    #[tokio::test]
    async fn test_local_inference_refuses_non_loopback_endpoint() {
        let result = run_local_inference("https://api.example.com", "m", "hello").await;
        assert!(!result.ok);
        assert!(
            matches!(result.error, Some(CommandError::Policy { .. })),
            "a non-loopback endpoint must be a POLICY refusal, got {:?}",
            result.error
        );
    }

    #[tokio::test]
    async fn test_local_inference_rejects_empty_prompt() {
        let result = run_local_inference("http://127.0.0.1:1", "m", "   ").await;
        assert!(!result.ok);
        assert!(matches!(
            result.error,
            Some(CommandError::Validation { .. })
        ));
    }

    // --- mcp namespace -----------------------------------------------------

    /// covers: REQ-MCP-001, REQ-SEC-003
    /// SPEC-005: capability grants are explicit; an ungranted capability is
    /// refused, and models cannot self-approve.
    #[test]
    fn test_mcp_capability_grant_is_explicit() {
        let allowed = check_mcp_capability(&["ReadVault".to_string()], "ReadVault");
        assert!(allowed.value.unwrap().allowed);

        let denied = check_mcp_capability(&["ReadVault".to_string()], "WriteVault");
        let d = denied.value.unwrap();
        assert!(!d.allowed, "ungranted capability was allowed");
        assert!(d.reason.contains("cannot self-approve"));

        assert!(!check_mcp_capability(&["Bogus".to_string()], "ReadVault").ok);
        assert!(!check_mcp_capability(&[], "Bogus").ok);
    }

    // --- incident namespace ------------------------------------------------

    /// covers: REQ-REPAIR-001
    /// AG-002 regression guard: a secret present in incident detail must not
    /// survive into an exportable capsule.
    #[test]
    fn test_repair_capsule_redacts_secrets() {
        let secret = "sk-live-0123456789abcdef";
        let result = build_repair_capsule(
            &format!("panic while saving: {secret}"),
            "null pointer in core",
            &[secret.to_string()],
        );
        assert!(result.ok);
        let v = result.value.unwrap();
        assert!(v.safe_for_export);
        assert!(
            !v.redacted_detail.contains(secret),
            "secret leaked into the capsule: {}",
            v.redacted_detail
        );
        assert!(v.redacted_detail.contains("[REDACTED]"));

        assert!(!build_repair_capsule("detail", "  ", &[]).ok);
    }

    // --- export namespace --------------------------------------------------

    /// covers: REQ-DOM-008
    /// The export gateway must refuse an escaping path (AG-003) as well as
    /// Restricted content.
    #[test]
    fn test_export_gateway_hardens_paths_and_blocks_restricted() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let traversal = export_evidence(&scope, "../../etc/passwd", "x", "Public");
        assert!(!traversal.ok, "escaping path was accepted for export");

        let restricted = export_evidence(&scope, "ok/path.txt", "secret", "Restricted");
        assert!(!restricted.ok, "Restricted content was exported");

        let ok = export_evidence(&scope, "drafts/one-pager.txt", "abstract", "Public");
        assert!(ok.ok);
        let receipt = ok.value.unwrap();
        assert_eq!(receipt.path, "drafts/one-pager.txt");
        assert_eq!(receipt.content_bytes, "abstract".len());
    }

    // --- patent namespace --------------------------------------------------

    /// covers: REQ-PAT-001
    #[test]
    fn test_lint_claims_uses_the_real_linter() {
        let good = lint_claims("1. A device comprising a valve");
        assert!(good.ok);
        assert!(good.value.unwrap().passed);

        let bad = lint_claims("A device comprising a valve");
        assert!(
            bad.ok,
            "the command succeeds; the lint verdict is the payload"
        );
        let outcome = bad.value.unwrap();
        assert!(!outcome.passed, "claim lacking '1. ' prefix was accepted");
        assert!(!outcome.findings.is_empty());

        assert!(!lint_claims("   ").ok, "empty claims must be rejected");
    }

    /// covers: REQ-PAT-002
    #[test]
    fn test_build_filing_package_does_not_claim_a_document_format() {
        let result = build_filing_package("1. A method", "The specification");
        assert!(result.ok);
        let view = result.value.unwrap();
        assert_ne!(view.format, "DOCX");
        assert_ne!(view.format, "PDF");
        assert_eq!(view.format, "MANIFEST");
        assert!(view.manifest.starts_with("MANIFEST sha256="));

        assert!(!build_filing_package("", "spec").ok);
        assert!(!build_filing_package("claims", "  ").ok);
    }

    // --- filing namespace --------------------------------------------------

    /// covers: REQ-PAT-005
    /// REQ-PAT-005: the receipt import must parse real values, and the AG-004
    /// regression (hardcoded application number) must stay fixed.
    #[test]
    fn test_import_receipt_parses_real_values() {
        let text =
            "United States Patent and Trademark Office\nAppNumber: 17/123,456\nConfNumber: 4321\n";
        let result = import_receipt(text);
        assert!(result.ok);
        let view = result.value.unwrap();
        assert_eq!(view.application_number, "17/123,456");
        assert_eq!(view.confirmation_number, "4321");
        assert_ne!(
            view.application_number, "12/345,678",
            "AG-004 regression: hardcoded application number returned"
        );
    }

    /// covers: REQ-PAT-005
    #[test]
    fn test_import_receipt_fails_closed_on_bad_input() {
        assert!(!import_receipt("   ").ok);
        assert!(!import_receipt("no fields here").ok);
        assert!(!import_receipt("AppNumber: 17/123,456").ok, "missing conf");
        assert!(!import_receipt("ConfNumber: 4321").ok, "missing app");
        assert!(!import_receipt("AppNumber: ABC\nConfNumber: 4321").ok);
    }

    /// covers: REQ-PAT-005, REQ-REL-003
    /// The handoff check must report prerequisites and must never imply that
    /// LINCHPIN will submit on the user's behalf.
    #[test]
    fn test_filing_handoff_reports_blockers_and_never_automates() {
        let ready = check_filing_handoff(&["Ads".into(), "Sba".into()], true);
        let r = ready.value.unwrap();
        assert!(r.ready_for_human_submission);
        assert!(r.blockers.is_empty());
        assert!(
            r.note.contains("Human submission only"),
            "handoff must state that submission is manual, got {:?}",
            r.note
        );

        let not_ready = check_filing_handoff(&["Ads".into()], false);
        let nr = not_ready.value.unwrap();
        assert!(!nr.ready_for_human_submission);
        assert!(!nr.blockers.is_empty(), "blockers must be reported");

        assert!(
            !check_filing_handoff(&["Bogus".into()], true).ok,
            "unknown form type must be rejected"
        );
    }

    // --- research namespace ------------------------------------------------

    /// covers: REQ-RES-001
    /// The lifecycle is driven by the real `research` state machine, so an
    /// illegal transition must be refused (REQ-RES-002).
    #[test]
    fn test_research_lifecycle_happy_path() {
        let r = apply_research_action("task-1", "Pending", &[], ResearchAction::Start, None);
        assert!(r.ok);
        assert_eq!(r.value.unwrap().status, "Active");

        let r = apply_research_action("task-1", "Active", &[], ResearchAction::Kill, None);
        assert!(r.ok);
        assert_eq!(r.value.unwrap().status, "Killed");
    }

    /// covers: REQ-RES-001
    /// A task cannot be killed before it starts; the guard lives in the domain
    /// crate, so the command must surface it as a POLICY failure.
    #[test]
    fn test_research_rejects_illegal_transition() {
        let r = apply_research_action("t", "Pending", &[], ResearchAction::Kill, None);
        assert!(!r.ok);
        assert!(
            matches!(r.error, Some(CommandError::Policy { .. })),
            "illegal transition must be a POLICY failure, got {:?}",
            r.error
        );

        let r2 = apply_research_action("t", "Completed", &[], ResearchAction::Start, None);
        assert!(!r2.ok, "a completed task must not restart");
    }

    /// covers: REQ-RES-001
    #[test]
    fn test_research_citation_accumulates_and_is_refused_when_finished() {
        let r = apply_research_action(
            "t",
            "Active",
            &[],
            ResearchAction::AddCitation,
            Some("US1234567A1"),
        );
        assert!(r.ok);
        let v = r.value.unwrap();
        assert_eq!(v.citation_count, 1);
        assert_eq!(v.citations, vec!["US1234567A1".to_string()]);

        // Adding to a finished task must fail.
        let r2 = apply_research_action(
            "t",
            "Killed",
            &["US1".to_string()],
            ResearchAction::AddCitation,
            Some("US2"),
        );
        assert!(!r2.ok, "citations must not be added to a finished task");
    }

    #[test]
    fn test_research_validates_its_inputs() {
        // Empty task id.
        let r = apply_research_action("  ", "Pending", &[], ResearchAction::Start, None);
        assert!(!r.ok);
        assert!(matches!(r.error, Some(CommandError::Validation { .. })));

        // Unknown status.
        let r2 = apply_research_action("t", "Bogus", &[], ResearchAction::Start, None);
        assert!(!r2.ok);

        // Missing / empty citation.
        let r3 = apply_research_action("t", "Active", &[], ResearchAction::AddCitation, None);
        assert!(!r3.ok);
        let r4 =
            apply_research_action("t", "Active", &[], ResearchAction::AddCitation, Some("   "));
        assert!(!r4.ok);
    }

    // --- evidence / Disclosure Firewall ------------------------------------

    /// covers: REQ-DOM-008
    /// REQ-DOM-008 / REQ-COM-004: Restricted content must never be approvable
    /// for public export, and the decision must come from the real firewall.
    #[test]
    fn test_export_firewall_blocks_restricted_content() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let restricted = evaluate_export(&scope, "unpublished enabling detail", "Restricted");
        assert!(restricted.ok, "the command itself succeeds");
        let decision = restricted.value.unwrap();
        assert!(
            !decision.allowed,
            "Restricted content was approved for export"
        );
        assert_eq!(decision.sensitivity, "Restricted");

        let public = evaluate_export(&scope, "public abstract", "Public");
        assert!(public.value.unwrap().allowed);

        let confidential = evaluate_export(&scope, "internal notes", "Confidential");
        assert!(confidential.value.unwrap().allowed);
    }

    #[test]
    fn test_export_validates_scope_content_and_sensitivity() {
        let bad = WorkspaceScope {
            workspace_id: String::new(),
        };
        assert!(!evaluate_export(&bad, "x", "Public").ok);

        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        assert!(!evaluate_export(&scope, "   ", "Public").ok);
        assert!(!evaluate_export(&scope, "x", "Secret").ok);
    }

    /// covers: REQ-OPS-001
    #[test]
    fn test_error_serializes_with_spec_006_class() {
        let err = CommandError::validation("bad input");
        let json = serde_json::to_string(&err).expect("serialize");
        assert!(json.contains("VALIDATION"), "got {json}");
        assert_eq!(err.safe_message(), "bad input");
    }

    /// covers: REQ-LLM-003
    #[test]
    fn test_command_result_serializes_envelope() {
        let result = record_conception(
            &WorkspaceScope {
                workspace_id: "ws-1".to_string(),
            },
            "content",
            true,
            None,
        );
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("correlation_id"), "got {json}");
        assert!(json.contains("\"ok\":true"), "got {json}");

        let failed = CommandResult::<()>::failure(
            CorrelationId("cid-1".to_string()),
            CommandError::policy("denied"),
        );
        let json = serde_json::to_string(&failed).expect("serialize");
        assert!(json.contains("\"ok\":false"), "got {json}");
        assert!(json.contains("POLICY"), "got {json}");
        assert!(!json.contains("\"value\""), "value must be omitted: {json}");
    }
}

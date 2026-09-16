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
    /// Scopes the task must cover (REQ-OPS-002).
    pub requested_scopes: usize,
    /// Scopes actually covered.
    pub covered_scopes: usize,
    /// Requested minus covered, so the gap is visible rather than inferred.
    pub uncovered_scopes: usize,
    /// True only when every requested scope was covered.
    pub coverage_complete: bool,
    /// Present only for a partial completion, so the run can be resumed.
    pub checkpoint: Option<String>,
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
            requested_scopes: task.requested_scopes,
            covered_scopes: task.covered_scopes,
            uncovered_scopes: task.uncovered_scopes(),
            coverage_complete: task.coverage_complete(),
            checkpoint: task.checkpoint.clone(),
        },
    )
}

/// Declare a research task's scope and how much of it is covered
/// (REQ-OPS-002).
///
/// REQ-OPS-002 requires that partial research "marks incomplete coverage", so
/// coverage must be settable through a production path. Without this the domain
/// API existed only for its own tests, which is the TEST_ONLY state the
/// reachability analysis flags.
pub fn set_research_coverage(
    scope: &WorkspaceScope,
    task_id: &str,
    requested_scopes: usize,
    covered_scopes: usize,
) -> CommandResult<ResearchTaskView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if task_id.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("task_id is required"),
        );
    }

    let mut task = research::ResearchTask::new(task_id);
    task.set_requested_scopes(requested_scopes);
    if let Err(e) = task.record_coverage(covered_scopes) {
        return CommandResult::failure(correlation, CommandError::validation(e));
    }

    CommandResult::success(
        correlation,
        ResearchTaskView {
            task_id: task.id.clone(),
            status: format!("{:?}", task.status),
            citation_count: task.citations.len(),
            citations: task.citations.clone(),
            requested_scopes: task.requested_scopes,
            covered_scopes: task.covered_scopes,
            uncovered_scopes: task.uncovered_scopes(),
            coverage_complete: task.coverage_complete(),
            checkpoint: task.checkpoint.clone(),
        },
    )
}

/// Finish a research task with incomplete coverage, persisting a checkpoint
/// (REQ-OPS-002).
///
/// "Partial research persists checkpoints and marks incomplete coverage." Both
/// halves are enforced by the domain object: a partial completion without a
/// checkpoint is refused, and the returned view reports the uncovered count so
/// the gap cannot be mistaken for completeness.
pub fn complete_research_partial(
    scope: &WorkspaceScope,
    task_id: &str,
    covered_scopes: usize,
    checkpoint: &str,
) -> CommandResult<ResearchTaskView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if task_id.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("task_id is required"),
        );
    }

    let mut task = research::ResearchTask::new(task_id);
    // A partial completion is only meaningful for a task that was running and
    // had declared a scope, so both are established through real transitions.
    if let Err(e) = task.start() {
        return CommandResult::failure(correlation, CommandError::policy(e));
    }
    if let Err(e) = task.complete_partial(covered_scopes, checkpoint) {
        return CommandResult::failure(correlation, CommandError::policy(e));
    }

    CommandResult::success(
        correlation,
        ResearchTaskView {
            task_id: task.id.clone(),
            status: format!("{:?}", task.status),
            citation_count: task.citations.len(),
            citations: task.citations.clone(),
            requested_scopes: task.requested_scopes,
            covered_scopes: task.covered_scopes,
            uncovered_scopes: task.uncovered_scopes(),
            coverage_complete: task.coverage_complete(),
            checkpoint: task.checkpoint.clone(),
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

/// Outcome of classifying a research claim (REQ-PAT-003).
///
/// `class` is returned verbatim as one of the seven canonical names so the UI
/// cannot relabel a hypothesis as an observation, and `requires` states what the
/// chosen class obliges the author to supply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimClassOutcome {
    /// One of the seven REQ-PAT-003 class names.
    pub class: String,
    /// True only for `OBSERVATION`, the sole class emittable from a source record.
    pub emittable_from_source_record: bool,
    /// Non-empty for every class other than `OBSERVATION`.
    pub requires: Vec<String>,
}

/// Classify a research claim and enforce REQ-PAT-003's provenance rule.
///
/// REQ-PAT-003: "Every research claim carries exactly one class ... Only
/// `OBSERVATION` may be emitted directly from a source record; all others store
/// inference method and contrary evidence."
///
/// This command is the production path for that rule. Without it the rule was
/// enforced only inside the research crate's own tests, which is why the
/// reachability analysis classified it TEST_ONLY: a requirement implemented but
/// unreachable from any user-facing path is not a delivered behaviour.
///
/// A claim whose class obligations are unmet is a VALIDATION failure, not a
/// silent acceptance: storing an unlabelled inference next to observations is
/// the fabricated-certainty failure the requirement exists to prevent.
pub fn classify_research_claim(
    class: &str,
    text: &str,
    source_record: Option<&str>,
    inference_method: Option<&str>,
    contrary_evidence: Option<&str>,
) -> CommandResult<ClaimClassOutcome> {
    let correlation = CorrelationId::new();

    if text.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("claim text cannot be empty"),
        );
    }

    let parsed = match class {
        "OBSERVATION" => research::ClaimClass::Observation,
        "HYPOTHESIS" => research::ClaimClass::Hypothesis,
        "INFERENCE" => research::ClaimClass::Inference,
        "LEGAL_RULE_SUMMARY" => research::ClaimClass::LegalRuleSummary,
        "MARKET_SIGNAL" => research::ClaimClass::MarketSignal,
        "PATENT_THREAT" => research::ClaimClass::PatentThreat,
        "COMMERCIAL_TARGET_ASSERTION" => research::ClaimClass::CommercialTargetAssertion,
        other => {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!(
                    "unknown claim class {other:?}; expected one of the seven REQ-PAT-003 classes"
                )),
            );
        }
    };

    // Build the claim from exactly what the caller declared, then let the domain
    // rule decide. Nothing is inferred or defaulted on the caller's behalf.
    let claim = research::ResearchClaim {
        text: text.to_string(),
        class: parsed,
        source_record: source_record.map(str::to_string),
        inference_method: inference_method.map(str::to_string),
        contrary_evidence: contrary_evidence.map(str::to_string),
    };

    match claim.validate() {
        Ok(()) => {
            let requires = if parsed.may_be_emitted_from_source_record() {
                Vec::new()
            } else {
                vec![
                    "inference_method".to_string(),
                    "contrary_evidence".to_string(),
                ]
            };
            CommandResult::success(
                correlation,
                ClaimClassOutcome {
                    class: parsed.as_str().to_string(),
                    emittable_from_source_record: parsed.may_be_emitted_from_source_record(),
                    requires,
                },
            )
        }
        Err(e) => CommandResult::failure(correlation, CommandError::validation(e.to_string())),
    }
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

/// Result of checking a support matrix (REQ-DOM-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportMatrixView {
    /// Exportable limitations that carry no specification or figure anchor.
    pub unsupported: Vec<String>,
    /// True only when every exportable limitation is anchored.
    pub exportable: bool,
    pub anchor_count: usize,
    /// Support links read back from the canonical store's junction table after
    /// this call persisted them (REQ-DATA-003). Zero when no vault was supplied.
    pub persisted_links: usize,
}

/// Check that every exportable limitation has a spec/figure anchor
/// (REQ-DOM-006).
///
/// REQ-DOM-006: "Support matrix maps every exportable limitation to spec/figure
/// anchors." The clause is a prohibition as much as a mapping: exporting a
/// limitation the specification does not enable asserts subject matter that is
/// not supported, so the shortfall is reported by NAME rather than as a bare
/// boolean, and `exportable` is true only when that list is empty.
///
/// `anchors` is a flat list of `(limitation_label, kind, reference)` triples as
/// the UI holds them; `SPECIFICATION` and `FIGURE` are the only accepted kinds.
pub fn check_support_matrix(
    scope: &WorkspaceScope,
    exportable: &[String],
    anchors: &[(String, String, String)],
    vault_path: Option<&std::path::Path>,
) -> CommandResult<SupportMatrixView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }

    // Map the caller's limitation labels onto real entity ids so the matrix is
    // keyed exactly like the domain object rather than by display string.
    let mut ids: Vec<(String, domain::EntityId)> = Vec::new();
    for label in exportable {
        if label.trim().is_empty() {
            return CommandResult::failure(
                correlation,
                CommandError::validation("an exportable limitation label cannot be empty"),
            );
        }
        ids.push((label.clone(), domain::EntityId(uuid::Uuid::new_v4())));
    }

    let mut matrix = domain::SupportMatrix::new();
    for (label, kind, reference) in anchors {
        let Some((_, id)) = ids.iter().find(|(l, _)| l == label) else {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!(
                    "anchor names limitation {label:?}, which is not in the exportable set"
                )),
            );
        };
        let anchor = match kind.as_str() {
            "SPECIFICATION" => domain::Anchor::specification(reference),
            "FIGURE" => domain::Anchor::figure(reference),
            other => {
                return CommandResult::failure(
                    correlation,
                    CommandError::validation(format!(
                        "unknown anchor kind {other:?}; expected SPECIFICATION or FIGURE"
                    )),
                )
            }
        };
        let anchor = match anchor {
            Ok(a) => a,
            Err(e) => {
                return CommandResult::failure(correlation, CommandError::validation(e.to_string()))
            }
        };
        if let Err(e) = matrix.add_anchor(id.clone(), anchor) {
            return CommandResult::failure(correlation, CommandError::validation(e.to_string()));
        }
    }

    let exportable_ids: Vec<domain::EntityId> = ids.iter().map(|(_, id)| id.clone()).collect();
    let unsupported_ids = matrix.unsupported(&exportable_ids);
    let unsupported: Vec<String> = ids
        .iter()
        .filter(|(_, id)| unsupported_ids.contains(id))
        .map(|(label, _)| label.clone())
        .collect();

    let exportable_ok = matrix.assert_exportable(&exportable_ids).is_ok();

    // REQ-DATA-003: the claim/support relationship is PERSISTED in the canonical
    // store's junction tables, not only computed in memory. Without this the
    // normalized schema would exist but nothing would ever write to it.
    let mut persisted_links = 0usize;
    if let Some(path) = vault_path {
        let vault = match storage::vault::Vault::open(path) {
            Ok(v) => v,
            Err(e) => {
                return CommandResult::failure(
                    correlation,
                    CommandError::policy(format!("cannot open vault: {e}")),
                )
            }
        };
        if let Err(e) = vault.create_workspace(scope.workspace_id.as_str()) {
            return CommandResult::failure(correlation, CommandError::policy(e.to_string()));
        }
        for (label, _) in &ids {
            let claim_id = match vault.put_claim(scope.workspace_id.as_str(), label) {
                Ok(id) => id,
                Err(e) => {
                    return CommandResult::failure(correlation, CommandError::policy(e.to_string()))
                }
            };
            for (a_label, kind, reference) in anchors {
                if a_label != label {
                    continue;
                }
                let anchor_id =
                    match vault.put_support_anchor(scope.workspace_id.as_str(), kind, reference) {
                        Ok(id) => id,
                        Err(e) => {
                            return CommandResult::failure(
                                correlation,
                                CommandError::policy(e.to_string()),
                            )
                        }
                    };
                if let Err(e) = vault.link_claim_support(&claim_id, &anchor_id) {
                    return CommandResult::failure(
                        correlation,
                        CommandError::policy(e.to_string()),
                    );
                }
            }
        }
        persisted_links = match vault.claim_support_rows(scope.workspace_id.as_str()) {
            Ok(rows) => rows.len(),
            Err(e) => {
                return CommandResult::failure(correlation, CommandError::policy(e.to_string()))
            }
        };
    }

    CommandResult::success(
        correlation,
        SupportMatrixView {
            unsupported,
            exportable: exportable_ok,
            anchor_count: matrix.anchor_count(),
            persisted_links,
        },
    )
}

/// Record that a piece of evidence supports a claim (REQ-DATA-003).
///
/// The claim/evidence relationship lives in the `claim_evidence` junction table,
/// keyed on both sides. Recording the same pair twice is a no-op rather than a
/// duplicate row, and the linked content addresses are read back through a join
/// so the caller sees what the store actually holds.
pub fn record_claim_evidence(
    scope: &WorkspaceScope,
    claim_label: &str,
    content_hash: &str,
    vault_path: &std::path::Path,
) -> CommandResult<ClaimEvidenceView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if claim_label.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("claim_label is required"),
        );
    }
    if !content_hash.starts_with("sha256:") || content_hash.len() != "sha256:".len() + 64 {
        return CommandResult::failure(
            correlation,
            CommandError::validation(
                "content_hash must be a sha256: content address (REQ-DATA-002)",
            ),
        );
    }

    let vault = match storage::vault::Vault::open(vault_path) {
        Ok(v) => v,
        Err(e) => {
            return CommandResult::failure(
                correlation,
                CommandError::policy(format!("cannot open vault: {e}")),
            )
        }
    };
    if let Err(e) = vault.create_workspace(scope.workspace_id.as_str()) {
        return CommandResult::failure(correlation, CommandError::policy(e.to_string()));
    }

    let claim_id = match vault.put_claim(scope.workspace_id.as_str(), claim_label) {
        Ok(id) => id,
        Err(e) => return CommandResult::failure(correlation, CommandError::policy(e.to_string())),
    };
    let evidence_id = match vault.put_evidence_record(scope.workspace_id.as_str(), content_hash) {
        Ok(id) => id,
        Err(e) => return CommandResult::failure(correlation, CommandError::policy(e.to_string())),
    };
    if let Err(e) = vault.link_claim_evidence(&claim_id, &evidence_id) {
        return CommandResult::failure(correlation, CommandError::policy(e.to_string()));
    }

    let linked = match vault.claim_evidence_hashes(&claim_id) {
        Ok(h) => h,
        Err(e) => return CommandResult::failure(correlation, CommandError::policy(e.to_string())),
    };
    let still_unsupported = match vault.unsupported_claims(scope.workspace_id.as_str()) {
        Ok(u) => u.iter().any(|c| c.claim_id == claim_id),
        Err(e) => return CommandResult::failure(correlation, CommandError::policy(e.to_string())),
    };

    CommandResult::success(
        correlation,
        ClaimEvidenceView {
            claim_label: claim_label.to_string(),
            linked_hashes: linked,
            still_unsupported,
        },
    )
}

/// Result of linking evidence to a claim (REQ-DATA-003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimEvidenceView {
    pub claim_label: String,
    /// Content addresses linked to the claim, read back through the junction.
    pub linked_hashes: Vec<String>,
    /// True when the claim has no SUPPORT anchor yet. Evidence alone does not
    /// satisfy the support matrix; the two relationships are independent.
    pub still_unsupported: bool,
}

/// Outcome of a vault backup (REQ-REL-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupView {
    pub destination: String,
    /// Digest of the state the backup captured, so it can be reconciled later.
    pub state_digest: String,
}

/// Outcome of a vault restore (REQ-REL-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreView {
    pub source: String,
    /// State digest BEFORE the restore: what was discarded.
    pub digest_before: String,
    /// State digest AFTER the restore: what is now held.
    pub digest_after: String,
    /// True when the restored state matches the backup exactly.
    pub reconciled: bool,
    /// True when no vault existed at the destination and the restore created it.
    /// Disaster recovery is exactly this case, so it is REPORTED rather than
    /// hidden behind an identical success response.
    pub destination_recreated: bool,
    /// Where an unreadable destination was moved before recovery, if one was.
    /// Recovery from a corrupt vault must not begin by deleting the evidence.
    pub destination_quarantined: Option<String>,
}

/// Write a consistent backup of the durable vault (REQ-REL-005).
///
/// Uses SQLite's online-backup API, not a file copy: a copy can capture a torn
/// write-ahead log, and a backup that silently restores to a corrupt state is
/// worse than no backup at all.
pub fn backup_vault(
    scope: &WorkspaceScope,
    destination: &str,
    vault_path: &std::path::Path,
) -> CommandResult<BackupView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if destination.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("destination is required"),
        );
    }
    if !vault_path.exists() {
        return CommandResult::failure(
            correlation,
            CommandError::policy(format!("no vault at {}", vault_path.display())),
        );
    }

    let vault = match storage::vault::Vault::open(vault_path) {
        Ok(v) => v,
        Err(e) => {
            return CommandResult::failure(
                correlation,
                CommandError::policy(format!("cannot open vault: {e}")),
            )
        }
    };
    let digest = match vault.state_digest() {
        Ok(d) => d,
        Err(e) => return CommandResult::failure(correlation, CommandError::policy(e.to_string())),
    };
    let dest = std::path::PathBuf::from(destination);
    if let Err(e) = vault.backup_to(&dest) {
        return CommandResult::failure(correlation, CommandError::policy(e.to_string()));
    }

    CommandResult::success(
        correlation,
        BackupView {
            destination: dest.display().to_string(),
            state_digest: digest,
        },
    )
}

/// Restore the durable vault from a backup, reporting the reconciliation
/// (REQ-REL-005).
///
/// Destructive by design: the current content is replaced. The digest is read
/// BEFORE and AFTER and both are returned, so the caller can see what was
/// discarded rather than being told only that a restore happened.
pub fn restore_vault(
    scope: &WorkspaceScope,
    source: &str,
    vault_path: &std::path::Path,
) -> CommandResult<RestoreView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if source.trim().is_empty() {
        return CommandResult::failure(correlation, CommandError::validation("source is required"));
    }

    let src = std::path::PathBuf::from(source);
    // Defence in depth: `digest_of` also refuses an absent path, but a restore
    // whose SOURCE does not exist must never reach the destructive step, so the
    // check is repeated at the boundary that would discard the live vault. The
    // order matters: the SOURCE is validated before the destination is touched
    // at all, so a mistyped backup leaves a corrupt vault exactly where it was.
    if !src.exists() {
        return CommandResult::failure(
            correlation,
            CommandError::policy(format!("no backup at {}", src.display())),
        );
    }
    let backup_digest = match storage::vault::Vault::digest_of(&src) {
        Ok(d) => d,
        Err(e) => {
            return CommandResult::failure(
                correlation,
                CommandError::policy(format!("cannot read backup {}: {e}", src.display())),
            )
        }
    };

    // RECOVERY, not merely restore. This command is what an operator runs after
    // a hard failure, so it must cope with the two states a disaster leaves
    // behind -- and an earlier revision coped with NEITHER:
    //   * the vault FILE is gone (wiped profile, deleted file, disk loss): the
    //     command refused with "no vault at ...", so recovery was impossible
    //     through the product. It now recreates the destination from the backup
    //     and says so.
    //   * the vault file exists but is UNREADABLE (torn write, bit rot): the
    //     command refused with "cannot open vault". It now moves the unreadable
    //     file aside -- preserved, never deleted -- and recovers into a fresh
    //     vault, reporting where the original went.
    let destination_existed = vault_path.exists();
    let mut destination_quarantined = None;
    if let Some(parent) = vault_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return CommandResult::failure(
                    correlation,
                    CommandError::policy(format!("cannot prepare {}: {e}", parent.display())),
                );
            }
        }
    }

    let (mut vault, before) = match open_for_recovery(vault_path) {
        Ok(pair) => pair,
        Err(first_error) => {
            if !vault_path.exists() {
                // The destination could not be CREATED: absent path under an
                // unwritable or uncreatable parent. That is a real failure, not
                // a disaster to recover from, so it is reported as one.
                return CommandResult::failure(
                    correlation,
                    CommandError::policy(format!(
                        "cannot create a vault at {}: {first_error}",
                        vault_path.display()
                    )),
                );
            }
            let quarantine = quarantine_path(vault_path);
            if let Err(e) = quarantine_unreadable(vault_path, &quarantine) {
                return CommandResult::failure(
                    correlation,
                    CommandError::policy(format!(
                        "vault {} is unreadable ({first_error}) and could not be moved aside: {e}",
                        vault_path.display()
                    )),
                );
            }
            destination_quarantined = Some(quarantine.display().to_string());
            match open_for_recovery(vault_path) {
                Ok(pair) => pair,
                Err(e) => {
                    return CommandResult::failure(
                        correlation,
                        CommandError::policy(format!(
                            "cannot create a recovery vault at {}: {e}",
                            vault_path.display()
                        )),
                    )
                }
            }
        }
    };
    if let Err(e) = vault.restore_from(&src) {
        return CommandResult::failure(correlation, CommandError::policy(e.to_string()));
    }
    let after = match vault.state_digest() {
        Ok(d) => d,
        Err(e) => return CommandResult::failure(correlation, CommandError::policy(e.to_string())),
    };

    CommandResult::success(
        correlation,
        RestoreView {
            source: src.display().to_string(),
            reconciled: after == backup_digest,
            digest_before: before,
            digest_after: after,
            destination_recreated: !destination_existed,
            destination_quarantined,
        },
    )
}

/// One of the five product truth boundaries, as returned to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryView {
    pub id: String,
    pub statement: String,
    pub enforced_by: String,
}

/// One promised capability, including what is honestly missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityView {
    pub requirement_id: String,
    pub uo_id: String,
    pub title: String,
    /// `IMPLEMENTED`, `PARTIAL` or `ABSENT`.
    pub state: String,
    pub realised_by: String,
    pub limitation: String,
}

/// The declared scope and truth boundaries (REQ-SCOPE-001, SPEC-000).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeView {
    pub boundaries: Vec<BoundaryView>,
    pub capabilities: Vec<CapabilityView>,
}

/// Report the declared product scope and the five truth boundaries.
///
/// The UI renders this rather than hard-coding its own copy, so the promise the
/// user reads and the promise the code enforces cannot drift apart. Partial
/// capabilities are reported WITH their limitation: a scope surface that showed
/// twelve ticks would be the over-claim the boundary exists to prevent.
pub fn get_scope_declaration() -> CommandResult<ScopeView> {
    let correlation = CorrelationId::new();

    let boundaries = domain::scope::TRUTH_BOUNDARIES
        .iter()
        .map(|boundary| BoundaryView {
            id: boundary.id.to_string(),
            statement: boundary.statement.to_string(),
            enforced_by: boundary.enforced_by.to_string(),
        })
        .collect();

    let capabilities = domain::scope::SCOPE_MAP
        .iter()
        .map(|entry| CapabilityView {
            requirement_id: entry.requirement_id.to_string(),
            uo_id: entry.uo_id.to_string(),
            title: entry.title.to_string(),
            state: match entry.state {
                domain::scope::CapabilityState::Implemented => "IMPLEMENTED",
                domain::scope::CapabilityState::Partial => "PARTIAL",
                domain::scope::CapabilityState::Absent => "ABSENT",
            }
            .to_string(),
            realised_by: entry.realised_by.to_string(),
            limitation: entry.limitation.to_string(),
        })
        .collect();

    CommandResult::success(
        correlation,
        ScopeView {
            boundaries,
            capabilities,
        },
    )
}

/// Open a vault AND read its state in one step, so an unreadable vault is
/// detected even when the file happens to open.
///
/// A corrupt database can pass `Vault::open` -- journal-mode and migration
/// statements may not touch the damaged page -- and only fail when the content
/// is queried. Treating "opens" as "readable" would leave recovery refusing to
/// run on exactly the vault that needs it.
fn open_for_recovery(
    vault_path: &std::path::Path,
) -> Result<(storage::vault::Vault, String), String> {
    let vault = storage::vault::Vault::open(vault_path).map_err(|e| e.to_string())?;
    let digest = vault.state_digest().map_err(|e| e.to_string())?;
    Ok((vault, digest))
}

/// Where an unreadable vault is preserved during recovery.
/// A sibling path, so the move is a rename on one volume and cannot fail for
/// cross-device reasons, and never the same name twice because the timestamp is
/// seconds since the epoch.
fn quarantine_path(vault_path: &std::path::Path) -> std::path::PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut name = vault_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "vault.db".to_string());
    name.push_str(&format!(".unreadable-{stamp}"));
    vault_path.with_file_name(name)
}

/// Move a vault and its write-ahead-log siblings aside, without deleting them.
fn quarantine_unreadable(
    vault_path: &std::path::Path,
    quarantine: &std::path::Path,
) -> Result<(), std::io::Error> {
    std::fs::rename(vault_path, quarantine)?;
    for suffix in ["-wal", "-shm"] {
        let from = sidecar(vault_path, suffix);
        if from.exists() {
            std::fs::rename(&from, sidecar(quarantine, suffix))?;
        }
    }
    Ok(())
}

/// `<vault>` + `-wal` / `-shm`, matching SQLite's own sidecar naming.
fn sidecar(vault_path: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    let mut name = vault_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "vault.db".to_string());
    name.push_str(suffix);
    vault_path.with_file_name(name)
}

/// Resolved runtime configuration as reported to the UI (REQ-FOUND-002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationView {
    pub app_data_dir: String,
    /// The actual durable vault FILE, not a directory the product never writes
    /// to. An earlier version reported `AppPaths::vault_dir`
    /// (`<app_data>/vaults`), which the Settings surface displayed as the vault
    /// location while the product wrote `<app_data>/linchpin-vault.db` one level
    /// up -- a surface asserting a state the product did not have.
    pub vault_file: String,
    pub runtime: String,
    /// Unknown configuration keys that were ignored in development. Reported
    /// rather than dropped, so a typo stays visible.
    pub warnings: Vec<String>,
}

/// The durable vault file, derived from the resolved app-data directory.
///
/// This mirrors `vault_file()` in the desktop crate, which is the single writer
/// of this path. Kept as one expression so the reported location cannot drift
/// from the written one.
pub fn vault_file_path() -> std::path::PathBuf {
    platform_windows::get_app_paths()
        .app_data_dir
        .join("linchpin-vault.db")
}

/// Report the typed configuration the process actually resolved
/// (REQ-FOUND-002).
///
/// The environment is parsed once into `platform_windows::AppConfig`; this
/// surfaces the RESULT so an operator can see which paths and runtime are in
/// effect instead of inferring them. A configuration that cannot be parsed is a
/// POLICY failure: the process would be running on a fallback, and that is not a
/// state to present as normal.
pub fn get_configuration() -> CommandResult<ConfigurationView> {
    let correlation = CorrelationId::new();
    match platform_windows::AppConfig::from_env() {
        Ok(config) => {
            let paths = config.paths();
            CommandResult::success(
                correlation,
                ConfigurationView {
                    app_data_dir: paths.app_data_dir.display().to_string(),
                    vault_file: vault_file_path().display().to_string(),
                    runtime: format!("{:?}", config.runtime),
                    warnings: config.warnings,
                },
            )
        }
        Err(e) => CommandResult::failure(correlation, CommandError::policy(e.to_string())),
    }
}

/// A valuation output as returned to the UI (REQ-COM-003).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValuationView {
    pub low: f64,
    pub high: f64,
    /// True only when high > low: a point estimate is not a range.
    pub is_range: bool,
    pub scenario_label: String,
    /// Sensitivity table, most influential assumption first.
    pub sensitivity: Vec<(String, f64)>,
    pub dominant_assumption: Option<String>,
}

/// Produce a valuation RANGE tied to explicit assumptions (REQ-COM-003).
///
/// REQ-COM-003: "Valuation outputs are ranges tied to explicit assumptions and
/// labelled planning scenarios, not certified appraisals. Sensitivity tables
/// show which assumptions dominate."
///
/// Enforced at this boundary rather than only in the crate, because a rule
/// reachable only from tests is not a delivered behaviour. A label asserting
/// appraisal authority is a POLICY failure -- the caller is asking the product
/// to present an estimate as an appraisal -- while a range with no assumption
/// behind it is a validation failure, since an unsourced number is exactly what
/// the requirement prohibits.
pub fn evaluate_valuation(
    scope: &WorkspaceScope,
    scenario_label: &str,
    low: f64,
    high: f64,
    assumptions: &[(String, f64, f64)],
) -> CommandResult<ValuationView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if scenario_label.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("a valuation requires a scenario label"),
        );
    }
    if !low.is_finite() || !high.is_finite() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("valuation bounds must be finite numbers"),
        );
    }

    let mut parsed = Vec::with_capacity(assumptions.len());
    for (name, alow, ahigh) in assumptions {
        match commercialization::Assumption::new(name, *alow, *ahigh) {
            Ok(a) => parsed.push(a),
            Err(e) => {
                return CommandResult::failure(correlation, CommandError::validation(e.to_string()))
            }
        }
    }

    match commercialization::ValuationRange::planning_scenario(scenario_label, low, high, parsed) {
        Ok(range) => {
            let dominant = range.dominant_assumption().map(|a| a.name.clone());
            CommandResult::success(
                correlation,
                ValuationView {
                    low: range.low,
                    high: range.high,
                    is_range: range.is_a_range(),
                    scenario_label: range.scenario_label.clone(),
                    sensitivity: range.sensitivity(),
                    dominant_assumption: dominant,
                },
            )
        }
        Err(e @ commercialization::ValuationError::NotAPlanningScenario(_)) => {
            CommandResult::failure(correlation, CommandError::policy(e.to_string()))
        }
        Err(e) => CommandResult::failure(correlation, CommandError::validation(e.to_string())),
    }
}

/// A scheduled docket deadline and whether it is authoritative (REQ-DOM-009).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadlineView {
    pub due_date: String,
    /// True only when a named ruleset source AND version back this date.
    pub authoritative: bool,
    pub ruleset_source: Option<String>,
    pub ruleset_version: Option<String>,
    pub suggested_by_model: bool,
    pub reviewed: bool,
}

/// Schedule a docket deadline, enforcing ruleset authority (REQ-DOM-009).
///
/// REQ-DOM-009: "Docket deadlines require authoritative ruleset source/version."
/// REQ-PAT-004 adds that "a model may suggest a deadline but cannot make it
/// authoritative without a ruleset/source mapping."
///
/// Both are enforced at this boundary, not only inside the domain crate, because
/// a rule reachable only from tests is not a delivered behaviour. A suggested
/// date comes back with `authoritative: false` and NO error: the suggestion is
/// useful, it is simply not a legal deadline until a ruleset backs it. Acting on
/// it as one is the real-world harm this clause exists to prevent.
pub fn schedule_docket_deadline(
    scope: &WorkspaceScope,
    due_date: &str,
    ruleset_source: Option<&str>,
    ruleset_version: Option<&str>,
    suggested_by_model: bool,
) -> CommandResult<DeadlineView> {
    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }
    if due_date.trim().is_empty() {
        return CommandResult::failure(
            correlation,
            CommandError::validation("due_date is required"),
        );
    }

    let mut deadline = if suggested_by_model {
        domain::DocketDeadline::suggested_by_model(due_date)
    } else {
        let (Some(source), Some(version)) = (ruleset_source, ruleset_version) else {
            // A date asserted without a ruleset is a POLICY matter: the caller
            // is asking the docket to treat an unsourced date as authoritative.
            return CommandResult::failure(
                correlation,
                CommandError::policy(
                    "a docket deadline requires both a ruleset source and version \
                     (REQ-DOM-009); use suggested_by_model to record an unbacked date",
                ),
            );
        };
        let authority = match domain::RuleSetAuthority::new(source, version) {
            Ok(a) => a,
            Err(e) => {
                return CommandResult::failure(correlation, CommandError::validation(e.to_string()))
            }
        };
        match domain::DocketDeadline::authoritative(due_date, authority) {
            Ok(d) => d,
            Err(e) => {
                return CommandResult::failure(correlation, CommandError::validation(e.to_string()))
            }
        }
    };

    // A model suggestion becomes authoritative only once a ruleset backs it.
    if suggested_by_model {
        if let (Some(source), Some(version)) = (ruleset_source, ruleset_version) {
            if let Ok(authority) = domain::RuleSetAuthority::new(source, version) {
                if let Err(e) = deadline.confirm_with_ruleset(authority) {
                    return CommandResult::failure(
                        correlation,
                        CommandError::validation(e.to_string()),
                    );
                }
            }
        }
    }

    let authoritative = deadline.is_authoritative();
    CommandResult::success(
        correlation,
        DeadlineView {
            due_date: deadline.due_date.clone(),
            authoritative,
            ruleset_source: deadline.ruleset.as_ref().map(|r| r.source.clone()),
            ruleset_version: deadline.ruleset.as_ref().map(|r| r.version.clone()),
            suggested_by_model: deadline.suggested_by_model,
            reviewed: deadline.reviewed,
        },
    )
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
        Ok(payload) => {
            // REQ-SCOPE-001 / TB-4: the package is text the inventor sends to
            // third parties, so it is the place a promise would do real damage.
            // The truth-boundary guard runs on the ACTUAL payload rather than on
            // the inputs, because the export path is what composes the copy.
            if let Err(violation) = domain::scope::check_claim_text(&payload) {
                return CommandResult::failure(
                    correlation,
                    CommandError::policy(format!(
                        "commercialization package crosses a truth boundary: {violation}"
                    )),
                );
            }
            CommandResult::success(
                correlation,
                CommercializationView {
                    target_count: target_names.len(),
                    redacted: true,
                    payload,
                },
            )
        }
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

/// A chain-of-title record as supplied by the caller (REQ-COM-002).
///
/// Every field is required to be honest about provenance: `source` names where
/// the record came from, and a record with no source is refused by the domain
/// layer rather than accepted as an unattributed assertion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleRecordInput {
    /// `INVENTOR_RECORD`, `OWNER_RECORD`, `ASSIGNMENT_EXECUTED`,
    /// `RECORDATION_EVIDENCE`, `LIEN_OR_SECURITY_INTEREST`, `FILING_EVENT`,
    /// `GRANT_EVENT`, `LEGAL_STATUS_EVENT` or `MAINTENANCE_DEADLINE`.
    pub kind: String,
    pub effective_date: String,
    pub party_from: Option<String>,
    pub party_to: Option<String>,
    pub source: String,
    pub recordation_id: Option<String>,
}

/// One dated record in the rendered timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleRecordView {
    pub kind: String,
    pub effective_date: String,
    pub party_from: Option<String>,
    pub party_to: Option<String>,
    pub source: String,
}

/// One finding a reader must know before relying on the chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleFindingView {
    pub label: String,
    pub detail: String,
}

/// A remaining-life estimate and the assumption it rests on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemainingLifeView {
    pub years: f64,
    pub basis: String,
    pub assumption: String,
}

/// Asset readiness as returned to the UI (REQ-COM-002).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetReadinessView {
    pub asset_label: String,
    pub timeline: Vec<TitleRecordView>,
    pub findings: Vec<TitleFindingView>,
    pub remaining_life: Option<RemainingLifeView>,
    pub related_families: Vec<String>,
    pub know_how_dependencies: Vec<String>,
    pub unresolved_questions: Vec<String>,
    /// Only for a continuous chain with no open ownership, inventorship or
    /// encumbrance finding and no unresolved question.
    pub ready_for_transaction: bool,
    /// Always true. Emitted so no consumer can read a recordation record as a
    /// legal conclusion, even one that ignores `findings`.
    pub recordation_is_evidence_not_validation: bool,
}

fn parse_title_kind(label: &str) -> Option<commercialization::asset_readiness::TitleRecordKind> {
    use commercialization::asset_readiness::TitleRecordKind as Kind;
    match label.trim().to_ascii_uppercase().as_str() {
        "INVENTOR_RECORD" => Some(Kind::InventorRecord),
        "OWNER_RECORD" => Some(Kind::OwnerRecord),
        "ASSIGNMENT_EXECUTED" => Some(Kind::AssignmentExecuted),
        "RECORDATION_EVIDENCE" => Some(Kind::RecordationEvidence),
        "LIEN_OR_SECURITY_INTEREST" => Some(Kind::LienOrSecurityInterest),
        "FILING_EVENT" => Some(Kind::FilingEvent),
        "GRANT_EVENT" => Some(Kind::GrantEvent),
        "LEGAL_STATUS_EVENT" => Some(Kind::LegalStatusEvent),
        "MAINTENANCE_DEADLINE" => Some(Kind::MaintenanceDeadline),
        _ => None,
    }
}

/// Build the chain-of-title timeline and the readiness verdict (REQ-COM-002).
///
/// The verdict is deliberately conservative: recordation is evidence, a missing
/// assignment is a GAP rather than an assumption, a lien is never cleared by
/// this code, and a caller-supplied unresolved question blocks readiness because
/// the product has no standing to answer it.
#[allow(clippy::too_many_arguments)]
pub fn build_asset_readiness(
    scope: &WorkspaceScope,
    asset_label: &str,
    records: &[TitleRecordInput],
    remaining_life_years: Option<f64>,
    remaining_life_basis: &str,
    related_families: &[String],
    know_how_dependencies: &[String],
    unresolved_questions: &[String],
) -> CommandResult<AssetReadinessView> {
    use commercialization::asset_readiness as title;

    let correlation = CorrelationId::new();

    if let Err(err) = scope.validate() {
        return CommandResult::failure(correlation, err);
    }

    let mut parsed = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        let Some(kind) = parse_title_kind(&record.kind) else {
            return CommandResult::failure(
                correlation,
                CommandError::validation(format!(
                    "record {index} has unknown kind {:?}; expected one of INVENTOR_RECORD, \
                     OWNER_RECORD, ASSIGNMENT_EXECUTED, RECORDATION_EVIDENCE, \
                     LIEN_OR_SECURITY_INTEREST, FILING_EVENT, GRANT_EVENT, LEGAL_STATUS_EVENT, \
                     MAINTENANCE_DEADLINE",
                    record.kind
                )),
            );
        };
        parsed.push(title::TitleRecord {
            kind,
            effective_date: record.effective_date.clone(),
            party_from: record.party_from.clone(),
            party_to: record.party_to.clone(),
            source: record.source.clone(),
            recordation_id: record.recordation_id.clone(),
        });
    }

    let remaining_life = remaining_life_years.map(|years| title::RemainingLife {
        years,
        basis: if remaining_life_basis.trim().is_empty() {
            "unspecified basis".to_string()
        } else {
            remaining_life_basis.trim().to_string()
        },
        assumption: "remaining life is a planning assumption, not a legal determination"
            .to_string(),
    });

    let readiness = match title::assess_asset_readiness(
        asset_label,
        &parsed,
        remaining_life,
        related_families,
        know_how_dependencies,
        unresolved_questions,
    ) {
        Ok(readiness) => readiness,
        Err(e) => {
            return CommandResult::failure(correlation, CommandError::validation(e.to_string()))
        }
    };

    let timeline = readiness
        .timeline
        .iter()
        .map(|record| TitleRecordView {
            kind: record.kind.label().to_string(),
            effective_date: record.effective_date.clone(),
            party_from: record.party_from.clone(),
            party_to: record.party_to.clone(),
            source: record.source.clone(),
        })
        .collect();

    let findings = readiness
        .findings
        .iter()
        .map(|finding| {
            let detail = match finding {
                title::TitleFinding::Gap { between, detail } => {
                    format!("{between}: {detail}")
                }
                title::TitleFinding::UnresolvedOwnership { detail }
                | title::TitleFinding::UnresolvedInventorship { detail }
                | title::TitleFinding::LienWarning { detail } => detail.clone(),
                title::TitleFinding::RecordationIsEvidenceOnly { recordation_id } => {
                    format!(
                        "recordation {recordation_id} is evidence that a document was recorded, \
                         not legal validation of title"
                    )
                }
            };
            TitleFindingView {
                label: finding.label().to_string(),
                detail,
            }
        })
        .collect();

    CommandResult::success(
        correlation,
        AssetReadinessView {
            asset_label: readiness.asset_label,
            timeline,
            findings,
            remaining_life: readiness.remaining_life.map(|life| RemainingLifeView {
                years: life.years,
                basis: life.basis,
                assumption: life.assumption,
            }),
            related_families: readiness.related_families,
            know_how_dependencies: readiness.know_how_dependencies,
            unresolved_questions: readiness.unresolved_questions,
            ready_for_transaction: readiness.ready_for_transaction,
            recordation_is_evidence_not_validation: readiness
                .recordation_is_evidence_not_validation,
        },
    )
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
/// PF-011 is now SATISFIED on the verification host: a real loopback inference
/// server (Ollama 0.34.0 on 127.0.0.1:11434) serves `smollm2:135m`, and
/// `scripts/live-fire-local-provider.sh` drives this command against it through
/// the packaged executable, asserting a live completion and the two negative
/// paths below. The earlier note in this comment -- "PF-011 is unmet on this
/// host, so in practice this returns a transport error" -- was accurate when
/// written and is preserved here only as a correction: it is no longer true, and
/// a stale claim in a doc comment is still a false claim.
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

    // REQ-OPS-002: "Retry only EXTERNAL_TRANSIENT with bounded policy."
    //
    // The bound is enforced here rather than left to callers, and only transient
    // failures are retried: an InvalidRequest will be rejected identically on
    // every attempt, and Unimplemented is a code path that does not exist, so
    // retrying either wastes time and buries the real diagnostic. Attempts are
    // reported in the outcome so the caller can see how hard the transport tried
    // rather than only whether it worked.
    let policy = provider_transport::RetryPolicy::default();
    let mut attempts: u32 = 0;
    let mut last_error: Option<provider_transport::TransportError> = None;
    let mut succeeded: Option<provider_transport::ModelResponse> = None;

    while attempts < policy.max_attempts {
        attempts += 1;
        match provider_transport::ProviderTransport::generate(&adapter, request.clone()).await {
            Ok(response) => {
                succeeded = Some(response);
                break;
            }
            Err(e) => {
                let transient = e.is_external_transient();
                last_error = Some(e);
                if !transient {
                    break;
                }
            }
        }
    }

    match succeeded {
        Some(response) => CommandResult::success(
            correlation,
            InferenceOutcome {
                live: true,
                transport,
                text: Some(response.text),
                error_class: None,
                detail: response.metadata,
                attempts,
                retryable_exhausted: false,
            },
        ),
        None => {
            let e = last_error.expect("a failed attempt records its error");
            let class = match e {
                provider_transport::TransportError::InvalidRequest(_) => "INVALID_REQUEST",
                provider_transport::TransportError::Unreachable(_) => "UNREACHABLE",
                provider_transport::TransportError::ProviderFailure { .. } => "PROVIDER_FAILURE",
                provider_transport::TransportError::InvalidResponse(_) => "INVALID_RESPONSE",
                provider_transport::TransportError::Unimplemented(_) => "UNIMPLEMENTED",
            };
            // Retrying is worth suggesting ONLY when the failure was transient
            // AND the bound was already spent. Telling a user to retry a
            // malformed request would be advice that cannot work.
            let retryable_exhausted = e.is_external_transient() && attempts >= policy.max_attempts;
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
                    attempts,
                    retryable_exhausted,
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
    /// Attempts actually made, including the first (REQ-OPS-002).
    pub attempts: u32,
    /// True when the failure was EXTERNAL_TRANSIENT and the policy had already
    /// exhausted its bound. Lets the UI say "try again" only when that is true.
    pub retryable_exhausted: bool,
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

    /// covers: REQ-PAT-003
    /// The rule must hold at the production boundary, not only inside the
    /// research crate's own tests.
    #[test]
    fn test_classify_research_claim_enforces_class_provenance() {
        // OBSERVATION may come straight from a source record.
        let obs = classify_research_claim(
            "OBSERVATION",
            "US1234567B2 discloses a valve.",
            Some("US1234567B2"),
            None,
            None,
        );
        assert!(obs.ok, "observation rejected: {:?}", obs.error);
        let view = obs.value.unwrap();
        assert_eq!(view.class, "OBSERVATION");
        assert!(view.emittable_from_source_record);
        assert!(view.requires.is_empty());

        // A non-observation class without its obligations is rejected, and the
        // message names the missing obligation.
        let bare = classify_research_claim("HYPOTHESIS", "the market is moving", None, None, None);
        assert!(!bare.ok, "hypothesis without provenance was accepted");
        let msg = format!("{:?}", bare.error);
        assert!(
            msg.contains("inference method"),
            "rejection did not name the inference method: {msg}"
        );

        // Supplying only the method is still incomplete: contrary evidence is
        // required too.
        let half = classify_research_claim(
            "HYPOTHESIS",
            "the market is moving",
            None,
            Some("extrapolated from three filings"),
            None,
        );
        assert!(
            !half.ok,
            "hypothesis without contrary evidence was accepted"
        );
        assert!(
            format!("{:?}", half.error).contains("contrary evidence"),
            "rejection did not name the contrary evidence"
        );

        // Full provenance is accepted.
        let full = classify_research_claim(
            "HYPOTHESIS",
            "the market is moving",
            None,
            Some("extrapolated from three filings"),
            Some("one filing contradicts this"),
        );
        assert!(
            full.ok,
            "fully-provenanced claim rejected: {:?}",
            full.error
        );
        let full_view = full.value.unwrap();
        assert_eq!(full_view.class, "HYPOTHESIS");
        assert!(!full_view.emittable_from_source_record);
        assert_eq!(
            full_view.requires,
            vec![
                "inference_method".to_string(),
                "contrary_evidence".to_string()
            ]
        );

        // A non-observation claim may not cite a source record directly: that is
        // the shape that presents an inference as a fact.
        let sneaky = classify_research_claim(
            "INFERENCE",
            "the claim is anticipated",
            Some("US1234567B2"),
            Some("comparison of limitations"),
            Some("one limitation is absent"),
        );
        assert!(!sneaky.ok, "inference citing a source record was accepted");
        assert!(
            format!("{:?}", sneaky.error).contains("source record"),
            "rejection did not name the source-record rule"
        );

        // An unlisted class is rejected rather than coerced into one of the seven.
        let bogus = classify_research_claim("GUESS", "t", None, Some("m"), Some("c"));
        assert!(!bogus.ok, "an unlisted class was accepted");
        assert!(format!("{:?}", bogus.error).contains("unknown claim class"));

        // Empty text asserts nothing.
        assert!(!classify_research_claim("OBSERVATION", "  ", Some("s"), None, None).ok);
    }

    /// covers: REQ-DOM-009
    /// The ruleset-authority rule must hold at the production boundary.
    #[test]
    fn test_schedule_docket_deadline_requires_ruleset_authority() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };

        // A ruleset-backed date is authoritative.
        let backed = schedule_docket_deadline(
            &scope,
            "2026-11-14",
            Some("USPTO-37CFR"),
            Some("2026.1"),
            false,
        );
        assert!(
            backed.ok,
            "ruleset-backed deadline rejected: {:?}",
            backed.error
        );
        let view = backed.value.unwrap();
        assert!(view.authoritative);
        assert_eq!(view.ruleset_source.as_deref(), Some("USPTO-37CFR"));
        assert_eq!(view.ruleset_version.as_deref(), Some("2026.1"));

        // Asserting a date with NO ruleset is a policy failure, not a silent
        // authoritative deadline.
        let bare = schedule_docket_deadline(&scope, "2026-11-14", None, None, false);
        assert!(
            !bare.ok,
            "a ruleset-less deadline was accepted as authoritative"
        );

        // Half a ruleset is a validation failure.
        let half_source =
            schedule_docket_deadline(&scope, "2026-11-14", Some("USPTO-37CFR"), None, false);
        assert!(!half_source.ok);
        let half_version =
            schedule_docket_deadline(&scope, "2026-11-14", None, Some("2026.1"), false);
        assert!(!half_version.ok);
        // Blank is not a value either.
        let blank =
            schedule_docket_deadline(&scope, "2026-11-14", Some("   "), Some("2026.1"), false);
        assert!(!blank.ok, "a blank ruleset source was accepted");

        // A model suggestion is returned but is NOT authoritative, and is not an
        // error: the date is useful, it is just not a legal deadline yet.
        let suggested = schedule_docket_deadline(&scope, "2026-11-14", None, None, true);
        assert!(
            suggested.ok,
            "a suggestion should be accepted as a suggestion"
        );
        let sview = suggested.value.unwrap();
        assert!(sview.suggested_by_model);
        assert!(
            !sview.authoritative,
            "an unbacked model suggestion was reported as authoritative"
        );
        assert!(sview.ruleset_source.is_none());

        // A suggestion CONFIRMED with a ruleset becomes authoritative.
        let confirmed = schedule_docket_deadline(
            &scope,
            "2026-11-14",
            Some("USPTO-37CFR"),
            Some("2026.1"),
            true,
        );
        assert!(confirmed.ok);
        let cview = confirmed.value.unwrap();
        assert!(cview.authoritative);
        assert!(cview.reviewed);

        // Empty dates and empty workspaces are rejected.
        assert!(!schedule_docket_deadline(&scope, "  ", Some("s"), Some("1"), false).ok);
        let no_scope = WorkspaceScope {
            workspace_id: "  ".to_string(),
        };
        assert!(!schedule_docket_deadline(&no_scope, "2026-11-14", Some("s"), Some("1"), false).ok);
    }

    /// covers: REQ-DOM-006
    /// The support-matrix gate must hold at the production boundary.
    #[test]
    fn test_check_support_matrix_blocks_unanchored_limitations() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let exportable = vec![
            "a self-sealing valve".to_string(),
            "a claimed but unspecified coating".to_string(),
        ];

        // Only the first limitation is anchored.
        let anchors = vec![(
            "a self-sealing valve".to_string(),
            "SPECIFICATION".to_string(),
            "[0042]".to_string(),
        )];
        let partial = check_support_matrix(&scope, &exportable, &anchors, None);
        assert!(partial.ok, "check failed: {:?}", partial.error);
        let view = partial.value.unwrap();
        assert!(
            !view.exportable,
            "an unanchored limitation did not block export"
        );
        assert_eq!(
            view.unsupported,
            vec!["a claimed but unspecified coating".to_string()],
            "the unsupported limitation should be named"
        );
        assert_eq!(view.anchor_count, 1);

        // Anchoring both clears the gate.
        let both = vec![
            (
                "a self-sealing valve".to_string(),
                "SPECIFICATION".to_string(),
                "[0042]".to_string(),
            ),
            (
                "a claimed but unspecified coating".to_string(),
                "FIGURE".to_string(),
                "FIG. 7".to_string(),
            ),
        ];
        let complete = check_support_matrix(&scope, &exportable, &both, None);
        assert!(complete.ok);
        let cview = complete.value.unwrap();
        assert!(cview.exportable, "a fully anchored set was not exportable");
        assert!(cview.unsupported.is_empty());
        assert_eq!(cview.anchor_count, 2);

        // An anchor for a limitation outside the exportable set is a validation
        // failure, not a silently ignored row.
        let stray = vec![(
            "not in the set".to_string(),
            "FIGURE".to_string(),
            "FIG. 1".to_string(),
        )];
        assert!(!check_support_matrix(&scope, &exportable, &stray, None).ok);

        // Unknown anchor kinds and blank references are rejected.
        let bad_kind = vec![(
            "a self-sealing valve".to_string(),
            "DRAWING".to_string(),
            "FIG. 1".to_string(),
        )];
        assert!(!check_support_matrix(&scope, &exportable, &bad_kind, None).ok);
        let blank_ref = vec![(
            "a self-sealing valve".to_string(),
            "FIGURE".to_string(),
            "   ".to_string(),
        )];
        assert!(!check_support_matrix(&scope, &exportable, &blank_ref, None).ok);

        // Empty labels and empty workspaces are rejected.
        assert!(!check_support_matrix(&scope, &["  ".to_string()], &[], None).ok);
        let no_scope = WorkspaceScope {
            workspace_id: String::new(),
        };
        assert!(!check_support_matrix(&no_scope, &exportable, &both, None).ok);
    }

    /// covers: REQ-DATA-003
    /// The support relationship must reach the canonical store's junction
    /// tables through the production command, not only exist in the schema.
    #[test]
    fn test_support_matrix_persists_into_junction_tables() {
        let dir = std::env::temp_dir().join(format!(
            "linchpin-cmd-junction-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let vault_path = dir.join("vault.db");

        let scope = WorkspaceScope {
            workspace_id: "ws-junction".to_string(),
        };
        let exportable = vec![
            "a self-sealing valve".to_string(),
            "an unanchored coating".to_string(),
        ];
        let anchors = vec![(
            "a self-sealing valve".to_string(),
            "SPECIFICATION".to_string(),
            "[0042]".to_string(),
        )];

        let result = check_support_matrix(&scope, &exportable, &anchors, Some(&vault_path));
        assert!(result.ok, "command failed: {:?}", result.error);
        let view = result.value.unwrap();
        assert_eq!(
            view.persisted_links, 1,
            "exactly one support link should be stored"
        );
        assert_eq!(
            view.unsupported,
            vec!["an unanchored coating".to_string()],
            "the unanchored limitation must still be reported"
        );

        // Independently read the store back: the relationship lives in the
        // junction table, and an orphaned claim is found by anti-join.
        let vault = storage::vault::Vault::open(&vault_path).unwrap();
        let rows = vault.claim_support_rows("ws-junction").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].claim_label, "a self-sealing valve");
        assert_eq!(rows[0].anchor_reference, "[0042]");
        assert_eq!(rows[0].anchor_kind, "SPECIFICATION");
        let unsupported = vault.unsupported_claims("ws-junction").unwrap();
        assert_eq!(unsupported.len(), 1);
        assert_eq!(unsupported[0].label, "an unanchored coating");

        // Re-running is idempotent: the same pair must not duplicate.
        let again = check_support_matrix(&scope, &exportable, &anchors, Some(&vault_path));
        assert_eq!(
            again.value.unwrap().persisted_links,
            1,
            "re-linking duplicated the relationship"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-003
    /// Evidence is linked to a claim through the `claim_evidence` junction, and
    /// that link is independent of the support relationship.
    #[test]
    fn test_claim_evidence_link_is_persisted_and_independent() {
        let dir = std::env::temp_dir().join(format!(
            "linchpin-cmd-evidence-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let vault_path = dir.join("vault.db");

        let scope = WorkspaceScope {
            workspace_id: "ws-ev".to_string(),
        };
        let hash = format!("sha256:{}", "a".repeat(64));

        let linked = record_claim_evidence(&scope, "a self-sealing valve", &hash, &vault_path);
        assert!(linked.ok, "link failed: {:?}", linked.error);
        let view = linked.value.unwrap();
        assert_eq!(view.linked_hashes, vec![hash.clone()]);
        assert!(
            view.still_unsupported,
            "evidence alone must not satisfy the support matrix"
        );

        // Idempotent.
        let again = record_claim_evidence(&scope, "a self-sealing valve", &hash, &vault_path);
        assert_eq!(again.value.unwrap().linked_hashes, vec![hash.clone()]);

        // A malformed content address is refused (REQ-DATA-002 shape).
        assert!(!record_claim_evidence(&scope, "a self-sealing valve", "deadbeef", &vault_path).ok);
        assert!(!record_claim_evidence(&scope, "  ", &hash, &vault_path).ok);
        let no_scope = WorkspaceScope {
            workspace_id: String::new(),
        };
        assert!(!record_claim_evidence(&no_scope, "a self-sealing valve", &hash, &vault_path).ok);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-REL-005
    /// Backup and restore must reconcile at the production boundary, reporting
    /// what was discarded rather than only that a restore happened.
    #[test]
    fn test_backup_and_restore_commands_reconcile() {
        let dir = std::env::temp_dir().join(format!(
            "linchpin-cmd-backup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let vault_path = dir.join("vault.db");
        let backup_path = dir.join("backup.db");

        // Establish a vault with content through the real command path.
        let scope = WorkspaceScope {
            workspace_id: "ws-backup".to_string(),
        };
        let recorded = commands_put_event(&vault_path, "ws-backup", "ev-1", "first conception");
        assert!(recorded, "could not seed the vault");

        let backup = backup_vault(&scope, backup_path.to_str().unwrap(), &vault_path);
        assert!(backup.ok, "backup failed: {:?}", backup.error);
        let bview = backup.value.unwrap();
        assert!(
            bview.state_digest.starts_with("sha256:"),
            "backup must report a state digest"
        );
        assert!(backup_path.exists());

        // Diverge after the backup.
        assert!(commands_put_event(
            &vault_path,
            "ws-backup",
            "ev-2",
            "second conception"
        ));

        let restore = restore_vault(&scope, backup_path.to_str().unwrap(), &vault_path);
        assert!(restore.ok, "restore failed: {:?}", restore.error);
        let rview = restore.value.unwrap();
        assert!(
            rview.reconciled,
            "the restored state does not match the backup: {} vs {}",
            rview.digest_after, rview.digest_before
        );
        assert_eq!(
            rview.digest_after, bview.state_digest,
            "restored digest must equal the digest the backup captured"
        );
        assert_ne!(
            rview.digest_before, rview.digest_after,
            "test premise: the vault had diverged before the restore"
        );

        // The post-backup work is gone.
        let vault = storage::vault::Vault::open(&vault_path).unwrap();
        let events = vault.list_conception_events("ws-backup").unwrap();
        assert_eq!(events.len(), 1, "post-backup work survived the restore");
        assert_eq!(events[0].content, "first conception");

        // Errors are reported, not silently ignored.
        assert!(!backup_vault(&scope, "  ", &vault_path).ok);
        assert!(!restore_vault(&scope, "  ", &vault_path).ok);

        // A refused restore must leave the vault EXACTLY as it was. Fail-closed
        // means no side effect, not merely a non-ok response.
        let survived = storage::vault::Vault::open(&vault_path)
            .unwrap()
            .state_digest()
            .unwrap();
        let absent = dir.join("absent.db");
        let missing = restore_vault(&scope, absent.to_str().unwrap(), &vault_path);
        assert!(!missing.ok, "restoring from a missing backup was accepted");
        // The defect this guards against: opening a mistyped path CREATED an
        // empty database there, so the phantom must not exist either.
        assert!(
            !absent.exists(),
            "a refused restore created {} as a side effect",
            absent.display()
        );
        // A file that is not a vault is refused, not given a schema.
        let foreign = dir.join("notes.txt");
        std::fs::write(&foreign, "not a database").unwrap();
        let garbage = restore_vault(&scope, foreign.to_str().unwrap(), &vault_path);
        assert!(
            !garbage.ok,
            "restoring from a non-vault file was accepted: {:?}",
            garbage.value
        );
        // Reconciliation measures the backup; it must not change it.
        let backup_after = storage::vault::Vault::digest_of(&backup_path).unwrap();
        assert_eq!(
            backup_after, bview.state_digest,
            "reconciliation mutated the backup it measured"
        );
        let after_refusals = storage::vault::Vault::open(&vault_path)
            .unwrap()
            .state_digest()
            .unwrap();
        assert_eq!(
            after_refusals, survived,
            "a refused restore changed the live vault"
        );

        let no_scope = WorkspaceScope {
            workspace_id: String::new(),
        };
        assert!(!backup_vault(&no_scope, backup_path.to_str().unwrap(), &vault_path).ok);
        assert!(!backup_vault(&scope, backup_path.to_str().unwrap(), &dir.join("none.db")).ok);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-REL-005
    /// Disaster recovery at the product boundary. A recovery command is run
    /// precisely when the destination is already broken, so the two states a
    /// hard failure leaves behind are the two that must work -- and the earlier
    /// revision refused BOTH: a deleted vault hit "no vault at ...", and a
    /// corrupt vault hit "cannot open vault", so recovery was impossible through
    /// the product exactly when it was needed.
    #[test]
    fn test_restore_recovers_from_a_lost_and_from_a_corrupt_vault() {
        let dir = std::env::temp_dir().join(format!(
            "linchpin-recovery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let vault_path = dir.join("vault.db");
        let backup_path = dir.join("backup.db");
        let scope = WorkspaceScope {
            workspace_id: "ws-recovery".to_string(),
        };

        assert!(commands_put_event(
            &vault_path,
            "ws-recovery",
            "ev-1",
            "precious work"
        ));
        let backup = backup_vault(&scope, backup_path.to_str().unwrap(), &vault_path);
        assert!(backup.ok, "backup failed: {:?}", backup.error);
        let captured = backup.value.unwrap().state_digest;

        // F1 -- TOTAL LOSS: the vault file is gone.
        std::fs::remove_file(&vault_path).expect("remove vault");
        assert!(!vault_path.exists());
        let lost = restore_vault(&scope, backup_path.to_str().unwrap(), &vault_path);
        assert!(
            lost.ok,
            "recovery from a lost vault failed: {:?}",
            lost.error
        );
        let lview = lost.value.unwrap();
        assert!(
            lview.destination_recreated,
            "recovery silently created the vault instead of reporting it"
        );
        assert!(
            lview.reconciled && lview.digest_after == captured,
            "recovered state does not reconcile with the backup"
        );
        assert!(lview.destination_quarantined.is_none());
        let events = storage::vault::Vault::open(&vault_path)
            .unwrap()
            .list_conception_events("ws-recovery")
            .unwrap();
        assert_eq!(
            events.len(),
            1,
            "the recovered vault does not hold the backed-up work"
        );
        assert_eq!(events[0].content, "precious work");

        // F2 -- CORRUPTION: the file exists but cannot be read.
        for suffix in ["-wal", "-shm"] {
            let side = dir.join(format!("vault.db{suffix}"));
            std::fs::remove_file(side).ok();
        }
        std::fs::write(&vault_path, vec![0x41u8; 4096]).expect("corrupt the vault");
        assert!(
            storage::vault::Vault::open(&vault_path)
                .and_then(|v| v.state_digest())
                .is_err(),
            "test premise: the corrupted vault must be unreadable"
        );

        let corrupt = restore_vault(&scope, backup_path.to_str().unwrap(), &vault_path);
        assert!(
            corrupt.ok,
            "recovery from a corrupt vault failed: {:?}",
            corrupt.error
        );
        let cview = corrupt.value.unwrap();
        let quarantine = cview
            .destination_quarantined
            .as_ref()
            .expect("the unreadable vault must be preserved, not overwritten");
        let quarantined_bytes =
            std::fs::read(quarantine).expect("the quarantined original must still exist");
        assert_eq!(
            quarantined_bytes.len(),
            4096,
            "the quarantined file is not the vault that was there"
        );
        assert!(cview.reconciled && cview.digest_after == captured);
        assert!(
            !cview.destination_recreated,
            "the destination existed, so it was not recreated"
        );

        // F3 -- NO SIDE EFFECT ON A REFUSED RECOVERY. A bad source must leave a
        // broken destination exactly where it is: quarantining it would destroy
        // the only copy an operator still has.
        std::fs::write(&vault_path, vec![0x42u8; 4096]).expect("corrupt the vault again");
        let before_bytes = std::fs::read(&vault_path).unwrap();
        let refused = restore_vault(&scope, dir.join("absent.db").to_str().unwrap(), &vault_path);
        assert!(!refused.ok, "a missing source was accepted");
        assert_eq!(
            std::fs::read(&vault_path).unwrap(),
            before_bytes,
            "a refused recovery disturbed the destination"
        );
        assert_eq!(
            quarantine_count(&dir),
            1,
            "a refused recovery quarantined the destination"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// How many quarantined vaults exist in `dir`, for the no-side-effect check.
    fn quarantine_count(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_name().to_string_lossy().contains(".unreadable-"))
                    .count()
            })
            .unwrap_or(0)
    }

    /// covers: REQ-SCOPE-001
    /// SPEC-000 promises that REQ-SCOPE-001..012 are UO-01..UO-12 and that
    /// "product claims must preserve five truth boundaries". Both halves are
    /// asserted here at the PRODUCT boundary, because the promise is about what
    /// the commands do, not about a document that says they should.
    #[test]
    fn test_scope_map_and_five_truth_boundaries_hold_at_the_product_boundary() {
        let scope = WorkspaceScope {
            workspace_id: "ws-scope".to_string(),
        };
        let dir = std::env::temp_dir().join(format!(
            "linchpin-scope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let vault_path = dir.join("vault.db");

        // The mapping is complete, ordered and one-to-one, and no entry may
        // claim more than it can show.
        assert_eq!(domain::scope::SCOPE_MAP.len(), 12);
        assert_eq!(domain::scope::TRUTH_BOUNDARIES.len(), 5);
        for (index, entry) in domain::scope::SCOPE_MAP.iter().enumerate() {
            assert_eq!(entry.uo_id, format!("UO-{:02}", index + 1));
            assert_eq!(entry.requirement_id, format!("REQ-SCOPE-{:03}", index + 1));
            assert_eq!(
                domain::scope::entry_for_requirement(entry.requirement_id)
                    .map(|e| e.uo_id)
                    .unwrap_or(""),
                entry.uo_id
            );
        }

        // TB-1: opportunity output carries axes and stated uncertainty, and its
        // own screening copy passes the boundary guard.
        let opportunity = evaluate_opportunity(
            &scope,
            "a self-sealing valve",
            0.7,
            0.6,
            0.3,
            "Medium",
            &["https://example.invalid/evidence/1".to_string()],
        );
        assert!(
            opportunity.ok,
            "opportunity failed: {:?}",
            opportunity.error
        );
        let view = opportunity.value.unwrap();
        assert!(
            domain::scope::check_claim_text(&view.screen_note).is_ok(),
            "the screening note crosses a truth boundary: {}",
            view.screen_note
        );
        assert!(
            !view.screen_note.to_lowercase().contains("patentab"),
            "screening copy must not present patentability: {}",
            view.screen_note
        );

        // TB-2: the claim linter is advisory. It reports findings and never a
        // clearance conclusion, and its findings pass the guard.
        let lint = lint_claims("1. A device comprising a valve seat.");
        assert!(lint.ok, "lint failed: {:?}", lint.error);
        for finding in &lint.value.unwrap().findings {
            assert!(
                domain::scope::check_claim_text(finding).is_ok(),
                "a lint finding asserts a legal conclusion: {finding}"
            );
        }

        // TB-3: a draft package is never FILED, and the handoff stays a human
        // step that cannot be satisfied by the product alone.
        let package = build_filing_package(
            "1. A device comprising a valve seat.",
            "A device with a valve seat that seals under pressure.",
        );
        assert!(package.ok, "package failed: {:?}", package.error);
        let handoff = check_filing_handoff(&["Ads".to_string()], false);
        assert!(handoff.ok, "handoff failed: {:?}", handoff.error);
        let readiness = handoff.value.unwrap();
        assert!(
            !readiness.ready_for_human_submission,
            "handoff declared submission-ready without a paid fee: {readiness:?}"
        );
        assert!(
            !readiness.blockers.is_empty(),
            "an unready handoff must name why: {readiness:?}"
        );

        // TB-4: the commercialization payload is redacted, passes the guard, and
        // the guard is APPLIED on that path rather than merely available -- a
        // target name that promises an outcome is refused by the real command.
        let package = build_commercialization_package(&scope, &["MegaCorp".to_string()]);
        assert!(package.ok, "package failed: {:?}", package.error);
        let payload = package.value.unwrap().payload;
        assert!(payload.contains("MegaCorp"), "the payload names the target");
        assert!(
            domain::scope::check_claim_text(&payload).is_ok(),
            "commercialization payload crosses a boundary: {payload}"
        );
        let promising =
            build_commercialization_package(&scope, &["guaranteed revenue partner".to_string()]);
        assert!(
            !promising.ok,
            "a package promising an outcome was exported: {:?}",
            promising.value
        );
        // The refusal must name the boundary it crossed, so the operator learns
        // which promise was refused rather than only that something failed.
        let refusal = promising.error.expect("a refusal must carry a cause");
        assert!(
            refusal.safe_message().contains("TB-4"),
            "the refusal does not name the boundary it crossed: {refusal}"
        );
        assert!(
            refusal.safe_message().contains("truth boundary"),
            "{refusal}"
        );

        // TB-5: AI assistance never becomes human conception.
        let ai = record_conception(
            &scope,
            "an idea suggested by a model",
            false,
            Some(&vault_path),
        );
        assert!(ai.ok, "record failed: {:?}", ai.error);
        let ai_value = ai.value.unwrap();
        assert_eq!(ai_value.event.origin, "AiSuggestion");
        assert!(ai_value.persisted, "the AI-sourced event was not persisted");
        let human = record_conception(
            &scope,
            "the inventor's own conception",
            true,
            Some(&vault_path),
        );
        assert!(human.ok);
        assert_eq!(human.value.unwrap().event.origin, "HumanConception");
        let events = storage::vault::Vault::open(&vault_path)
            .unwrap()
            .list_conception_events("ws-scope")
            .unwrap();
        assert_eq!(events.len(), 2, "origins must be stored separately");
        assert!(
            events.iter().any(|e| e.origin == "AiSuggestion"),
            "the AI origin was lost in storage"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-COM-002
    /// SPEC-009 requires the chain-of-title timeline at the product boundary,
    /// with recordation represented as evidence and never as legal validation.
    #[test]
    fn test_asset_readiness_reports_gaps_and_never_treats_recordation_as_validation() {
        let scope = WorkspaceScope {
            workspace_id: "ws-title".to_string(),
        };

        let record =
            |kind: &str, date: &str, from: Option<&str>, to: Option<&str>, source: &str| {
                TitleRecordInput {
                    kind: kind.to_string(),
                    effective_date: date.to_string(),
                    party_from: from.map(str::to_string),
                    party_to: to.map(str::to_string),
                    source: source.to_string(),
                    recordation_id: None,
                }
            };

        // A continuous chain, with a recordation record present.
        let mut recordation = record(
            "RECORDATION_EVIDENCE",
            "2024-07-01",
            Some("Dana Inventor"),
            Some("Linchpin Holdings"),
            "USPTO Assignment Search",
        );
        recordation.recordation_id = Some("REEL/FRAME 1234/0567".to_string());

        let records = vec![
            record(
                "INVENTOR_RECORD",
                "2023-01-15",
                None,
                Some("Dana Inventor"),
                "declaration",
            ),
            record(
                "ASSIGNMENT_EXECUTED",
                "2024-05-01",
                Some("Dana Inventor"),
                Some("Linchpin Holdings"),
                "executed assignment",
            ),
            record(
                "OWNER_RECORD",
                "2024-06-01",
                None,
                Some("Linchpin Holdings"),
                "assignment deck",
            ),
            recordation,
        ];

        let ready = build_asset_readiness(
            &scope,
            "Self-sealing valve",
            &records,
            Some(14.5),
            "20-year term from filing",
            &["Family A".to_string()],
            &["weld procedure".to_string()],
            &[],
        );
        assert!(ready.ok, "readiness failed: {:?}", ready.error);
        let view = ready.value.unwrap();
        assert!(view.ready_for_transaction, "findings: {:?}", view.findings);
        assert!(
            view.recordation_is_evidence_not_validation,
            "the output must state that recordation is not validation"
        );
        assert!(
            view.findings
                .iter()
                .any(|f| f.label == "RECORDATION_IS_EVIDENCE_ONLY"),
            "the recordation must be surfaced as evidence: {:?}",
            view.findings
        );
        assert_eq!(
            view.timeline.first().map(|r| r.effective_date.as_str()),
            Some("2023-01-15"),
            "the timeline must be ordered by effective date"
        );
        assert_eq!(view.related_families.len(), 1);

        // A missing assignment is a GAP, not an assumption.
        let broken = vec![
            record(
                "INVENTOR_RECORD",
                "2023-01-15",
                None,
                Some("Dana Inventor"),
                "declaration",
            ),
            record(
                "OWNER_RECORD",
                "2024-06-01",
                None,
                Some("First Assignee"),
                "assignment deck",
            ),
            record(
                "OWNER_RECORD",
                "2025-02-01",
                None,
                Some("Second Assignee"),
                "assignment deck",
            ),
        ];
        let gapped = build_asset_readiness(
            &scope,
            "Self-sealing valve",
            &broken,
            None,
            "",
            &[],
            &[],
            &[],
        );
        assert!(gapped.ok, "{:?}", gapped.error);
        let gapped = gapped.value.unwrap();
        assert_eq!(
            gapped.findings.iter().filter(|f| f.label == "GAP").count(),
            2,
            "both transitions lack evidence: {:?}",
            gapped.findings
        );
        assert!(
            !gapped.ready_for_transaction,
            "a chain with gaps must not be reported ready"
        );
        assert!(
            gapped
                .unresolved_questions
                .iter()
                .any(|q| q.contains("remaining life was not supplied")),
            "absent remaining life must be carried as a question, not invented: {:?}",
            gapped.unresolved_questions
        );

        // An encumbrance blocks readiness and is never cleared here.
        let mut encumbered = records.clone();
        encumbered.push(record(
            "LIEN_OR_SECURITY_INTEREST",
            "2025-01-10",
            Some("Lender"),
            Some("Linchpin Holdings"),
            "UCC filing",
        ));
        let lien = build_asset_readiness(&scope, "Valve", &encumbered, None, "", &[], &[], &[]);
        let lien = lien.value.unwrap();
        assert!(lien.findings.iter().any(|f| f.label == "LIEN_WARNING"));
        assert!(!lien.ready_for_transaction);

        // A caller-supplied open question blocks readiness too: the product has
        // no standing to answer it.
        let open = build_asset_readiness(
            &scope,
            "Valve",
            &records,
            None,
            "",
            &[],
            &[],
            &["who owns the 2025 improvement?".to_string()],
        );
        assert!(!open.value.unwrap().ready_for_transaction);

        // Invalid input is refused with a named cause rather than defaulted.
        let bad_date = vec![record(
            "INVENTOR_RECORD",
            "15/01/2023",
            None,
            Some("Dana Inventor"),
            "declaration",
        )];
        let refused = build_asset_readiness(&scope, "Valve", &bad_date, None, "", &[], &[], &[]);
        assert!(!refused.ok);
        assert!(
            refused.error.expect("cause").safe_message().contains("ISO"),
            "the refusal must name the date requirement"
        );
        let unknown_kind = vec![record("SOMETHING_ELSE", "2023-01-15", None, None, "deck")];
        assert!(!build_asset_readiness(&scope, "Valve", &unknown_kind, None, "", &[], &[], &[]).ok);
        let no_source = vec![record("INVENTOR_RECORD", "2023-01-15", None, None, "  ")];
        assert!(!build_asset_readiness(&scope, "Valve", &no_source, None, "", &[], &[], &[]).ok);
    }

    /// Seed a conception event directly through the vault, for the backup tests.
    ///
    /// The workspace is a parameter, not the hardcoded "ws-backup" it used to be:
    /// a helper that silently writes into a different workspace than its caller
    /// names is how a recovery test came to query an empty vault and look like a
    /// product defect.
    fn commands_put_event(
        vault_path: &std::path::Path,
        workspace_id: &str,
        event_id: &str,
        content: &str,
    ) -> bool {
        let Ok(vault) = storage::vault::Vault::open(vault_path) else {
            return false;
        };
        vault.create_workspace(workspace_id).is_ok()
            && vault
                .put_conception_event(workspace_id, event_id, "HumanConception", content)
                .is_ok()
    }

    /// covers: REQ-OPS-002
    /// "Partial research persists checkpoints and marks incomplete coverage."
    /// The rule must hold at the production boundary, not only in the crate.
    #[test]
    fn test_research_coverage_and_partial_completion_at_the_boundary() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };

        // Declaring a scope with partial coverage marks the gap.
        let partial = set_research_coverage(&scope, "task-1", 5, 2);
        assert!(partial.ok, "coverage rejected: {:?}", partial.error);
        let pview = partial.value.unwrap();
        assert_eq!(pview.requested_scopes, 5);
        assert_eq!(pview.covered_scopes, 2);
        assert_eq!(pview.uncovered_scopes, 3);
        assert!(
            !pview.coverage_complete,
            "partial coverage reported as complete"
        );
        assert!(pview.checkpoint.is_none(), "nothing is checkpointed yet");

        // Over-covering is rejected rather than clamped.
        assert!(!set_research_coverage(&scope, "task-1", 2, 5).ok);

        // A partial completion without a checkpoint is refused.
        assert!(!complete_research_partial(&scope, "task-1", 2, "").ok);
        assert!(!complete_research_partial(&scope, "task-1", 2, "   ").ok);

        // Empty identifiers and workspaces are rejected.
        assert!(!set_research_coverage(&scope, "  ", 1, 1).ok);
        assert!(!complete_research_partial(&scope, "  ", 1, "c").ok);
        let no_scope = WorkspaceScope {
            workspace_id: String::new(),
        };
        assert!(!set_research_coverage(&no_scope, "task-1", 1, 1).ok);
        assert!(!complete_research_partial(&no_scope, "task-1", 1, "c").ok);
    }

    /// covers: REQ-COM-003
    /// The range/assumption/planning-label rule must hold at the boundary.
    #[test]
    fn test_evaluate_valuation_enforces_range_assumptions_and_label() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let assumptions = vec![
            ("market size".to_string(), 100.0, 400.0),
            ("royalty rate".to_string(), 0.02, 0.05),
        ];

        // A well-formed planning scenario comes back as a range with a ranked
        // sensitivity table and the dominant assumption named.
        let ok = evaluate_valuation(
            &scope,
            "Conservative planning scenario",
            1_000_000.0,
            2_500_000.0,
            &assumptions,
        );
        assert!(ok.ok, "valuation rejected: {:?}", ok.error);
        let view = ok.value.unwrap();
        assert!(view.is_range);
        assert_eq!(view.low, 1_000_000.0);
        assert_eq!(view.high, 2_500_000.0);
        assert_eq!(view.dominant_assumption.as_deref(), Some("market size"));
        assert_eq!(view.sensitivity[0].0, "market size");
        assert_eq!(view.sensitivity.len(), 2);

        // No assumptions: an unsourced number is refused.
        assert!(
            !evaluate_valuation(&scope, "Planning scenario", 1.0, 2.0, &[]).ok,
            "a valuation with no explicit assumption was accepted"
        );

        // A label asserting appraisal authority is a POLICY failure, not a
        // validation one: the request itself is what policy forbids.
        let appraisal = evaluate_valuation(&scope, "Certified valuation", 1.0, 2.0, &assumptions);
        assert!(!appraisal.ok, "an appraisal label was accepted");
        assert!(
            matches!(appraisal.error, Some(CommandError::Policy { .. })),
            "appraisal language must be a POLICY failure, got {:?}",
            appraisal.error
        );

        // An inverted range is a validation failure.
        assert!(!evaluate_valuation(&scope, "Planning scenario", 9.0, 1.0, &assumptions).ok);

        // Non-finite bounds are rejected rather than propagating NaN.
        assert!(!evaluate_valuation(&scope, "Planning scenario", f64::NAN, 1.0, &assumptions).ok);
        assert!(
            !evaluate_valuation(
                &scope,
                "Planning scenario",
                0.0,
                f64::INFINITY,
                &assumptions
            )
            .ok
        );

        // Blank labels, unnamed assumptions and empty workspaces are rejected.
        assert!(!evaluate_valuation(&scope, "   ", 1.0, 2.0, &assumptions).ok);
        let unnamed = vec![("   ".to_string(), 1.0, 2.0)];
        assert!(!evaluate_valuation(&scope, "Planning scenario", 1.0, 2.0, &unnamed).ok);
        let no_scope = WorkspaceScope {
            workspace_id: String::new(),
        };
        assert!(!evaluate_valuation(&no_scope, "Planning scenario", 1.0, 2.0, &assumptions).ok);
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

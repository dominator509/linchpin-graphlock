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
        entry(
            "opportunity",
            false,
            "domain OpportunityCandidate exists; no IPC command wired yet",
        ),
        entry(
            "patent",
            false,
            "patent crate exists; no IPC command wired yet",
        ),
        entry("filing", false, "not implemented"),
        entry(
            "docket",
            false,
            "domain DocketRecord exists; no IPC command wired yet",
        ),
        entry(
            "prosecution",
            false,
            "patent OfficeAction exists; no IPC command wired yet",
        ),
        entry(
            "commercialization",
            false,
            "commercialization crate exists; no IPC command wired yet",
        ),
        entry(
            "provider",
            false,
            "provider_transport exists; not reachable and no live provider configured",
        ),
        entry("mcp", false, "mcp_hub exists; no IPC command wired yet"),
        entry(
            "incident",
            false,
            "crash_reporter exists; no IPC command wired yet",
        ),
        entry(
            "export",
            false,
            "the export namespace command is not wired yet",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

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
            implemented,
            vec!["conception", "workspace", "research", "evidence"],
            "implementation claims do not match the four wired namespaces"
        );
    }

    // --- research namespace ------------------------------------------------

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

    #[test]
    fn test_error_serializes_with_spec_006_class() {
        let err = CommandError::validation("bad input");
        let json = serde_json::to_string(&err).expect("serialize");
        assert!(json.contains("VALIDATION"), "got {json}");
        assert_eq!(err.safe_message(), "bad input");
    }

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

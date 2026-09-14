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
pub fn record_conception(
    scope: &WorkspaceScope,
    content: &str,
    author_is_human: bool,
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

    let outcome = RecordConceptionOutcome {
        event: ConceptionEventView {
            event_id: block.id.0.to_string(),
            content_hash: content_hash(content),
            content_bytes: content.len(),
            origin: origin.to_string(),
        },
        // This command does NOT yet write to durable storage: the vault wiring
        // (REQ-DATA-001) is not implemented. Saying so is required by DOD-026;
        // claiming persistence here would be a fabricated side effect.
        persisted: false,
        storage_detail: "not persisted: vault write path is not implemented (REQ-DATA-001 open)"
            .to_string(),
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
        entry(
            "evidence",
            false,
            "domain types exist; no IPC command wired yet",
        ),
        entry(
            "opportunity",
            false,
            "domain OpportunityCandidate exists; no IPC command wired yet",
        ),
        entry(
            "research",
            false,
            "research crate exists; no IPC command wired yet",
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
            "Disclosure Firewall exists; no IPC command wired yet",
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
        let result = record_conception(&scope, "a self-sealing valve", true);
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
        let human = record_conception(&scope, "same text", true).value.unwrap();
        let ai = record_conception(&scope, "same text", false).value.unwrap();

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
        let r = record_conception(&bad_scope, "text", true);
        assert!(!r.ok);
        assert!(matches!(r.error, Some(CommandError::Validation { .. })));

        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let r2 = record_conception(&scope, "   ", true);
        assert!(!r2.ok);
        assert!(matches!(r2.error, Some(CommandError::Validation { .. })));
    }

    /// DOD-026: the command must not claim a side effect it did not perform.
    #[test]
    fn test_record_conception_does_not_claim_unimplemented_persistence() {
        let scope = WorkspaceScope {
            workspace_id: "ws-1".to_string(),
        };
        let outcome = record_conception(&scope, "content", true).value.unwrap();
        assert!(
            !outcome.persisted,
            "command claimed persistence while the vault write path is unimplemented"
        );
        assert!(outcome.storage_detail.contains("not persisted"));
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
            vec!["conception", "workspace"],
            "implementation claims do not match the two wired namespaces"
        );
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

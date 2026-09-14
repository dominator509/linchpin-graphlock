use std::path::PathBuf;

pub mod commands;

/// Where LINCHPIN stores its local data.
///
/// Uses the same resolution as `platform_windows::get_app_paths` so the health
/// probe checks the directory the application actually writes to.
fn storage_root() -> PathBuf {
    platform_windows::get_app_paths().app_data_dir
}

/// Report runtime health from real probes.
///
/// GraphLock context (anti-gaming finding AG-007a): this command previously
/// returned `application::check_system_health()`, a constant
/// `SystemHealth { status: "OK", storage_ok: true }` that checked nothing. This
/// was the ONLY Tauri command the shipped desktop app exposed, so the product's
/// single runtime self-report was a hard-coded success — the exact
/// "health signals must never lie" failure DOD-037 prohibits.
///
/// It now probes the real storage directory. A missing or unwritable data
/// directory produces `status: "DEGRADED"` and `storage_ok: false`.
#[tauri::command]
fn get_system_health() -> Result<application::SystemHealth, String> {
    let root = storage_root();
    let probe = application::probe_storage(&root);
    Ok(application::check_system_health_with(&[probe]))
}

/// Record a human conception event (REQ-DOM-001, REQ-DOM-002).
///
/// The vault is resolved from the platform app-data directory so the event is
/// durably committed rather than merely acknowledged.
#[tauri::command]
fn record_conception(
    workspace_id: String,
    content: String,
    author_is_human: bool,
) -> commands::CommandResult<commands::RecordConceptionOutcome> {
    let scope = commands::WorkspaceScope { workspace_id };
    let vault_path = vault_file();
    commands::record_conception(&scope, &content, author_is_human, Some(&vault_path))
}

/// Path to the durable vault database.
fn vault_file() -> PathBuf {
    let root = storage_root();
    // The vault directory must exist before SQLite can create the file.
    let _ = std::fs::create_dir_all(&root);
    root.join("linchpin-vault.db")
}

/// Report SPEC-003 namespace coverage.
#[tauri::command]
fn get_namespace_status() -> Vec<commands::NamespaceStatus> {
    commands::namespace_status()
}

/// Advance a research task (REQ-RES-002).
#[tauri::command]
fn apply_research_action(
    task_id: String,
    current_status: String,
    citations: Vec<String>,
    action: commands::ResearchAction,
    citation: Option<String>,
) -> commands::CommandResult<commands::ResearchTaskView> {
    commands::apply_research_action(
        &task_id,
        &current_status,
        &citations,
        action,
        citation.as_deref(),
    )
}

/// Evaluate content against the Disclosure Firewall (REQ-DOM-008).
#[tauri::command]
fn evaluate_export(
    workspace_id: String,
    content: String,
    sensitivity: String,
) -> commands::CommandResult<commands::ExportDecision> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::evaluate_export(&scope, &content, &sensitivity)
}

/// Lint a claim set (REQ-PAT-001).
#[tauri::command]
fn lint_claims(claims: String) -> commands::CommandResult<commands::LintOutcome> {
    commands::lint_claims(&claims)
}

/// Build a filing-package manifest (REQ-PAT-002).
#[tauri::command]
fn build_filing_package(
    claims: String,
    specification: String,
) -> commands::CommandResult<commands::FilingPackageView> {
    commands::build_filing_package(&claims, &specification)
}

/// Import a USPTO acknowledgement receipt (REQ-PAT-005).
#[tauri::command]
fn import_receipt(receipt_text: String) -> commands::CommandResult<commands::ReceiptView> {
    commands::import_receipt(&receipt_text)
}

/// Report filing-handoff readiness (REQ-PAT-005, human submission only).
#[tauri::command]
fn check_filing_handoff(
    forms: Vec<String>,
    fee_paid: bool,
) -> commands::CommandResult<commands::HandoffReadiness> {
    commands::check_filing_handoff(&forms, fee_paid)
}

/// Evaluate an opportunity candidate (REQ-DOM-003).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn evaluate_opportunity(
    workspace_id: String,
    description: String,
    technical_feasibility: f32,
    market_potential: f32,
    legal_risk: f32,
    uncertainty: String,
    evidence_uris: Vec<String>,
) -> commands::CommandResult<commands::OpportunityView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::evaluate_opportunity(
        &scope,
        &description,
        technical_feasibility,
        market_potential,
        legal_risk,
        &uncertainty,
        &evidence_uris,
    )
}

/// Advance a docket record (REQ-DOM-007).
#[tauri::command]
fn advance_docket(
    workspace_id: String,
    current_state: String,
    receipt: Option<String>,
    commercialize: bool,
) -> commands::CommandResult<commands::DocketView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::advance_docket(&scope, &current_state, receipt.as_deref(), commercialize)
}

/// Draft an office-action response workspace.
#[tauri::command]
fn draft_office_action_response(
    action_type: String,
    cited_art: Vec<String>,
) -> commands::CommandResult<commands::OfficeActionView> {
    commands::draft_office_action_response(&action_type, &cited_art)
}

/// Build a commercialization package (REQ-COM-004).
#[tauri::command]
fn build_commercialization_package(
    workspace_id: String,
    target_names: Vec<String>,
) -> commands::CommandResult<commands::CommercializationView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::build_commercialization_package(&scope, &target_names)
}

/// Report provider lane availability (never performs inference).
#[tauri::command]
fn provider_status() -> commands::CommandResult<Vec<commands::ProviderLane>> {
    commands::provider_status()
}

/// Check an MCP capability grant (SPEC-005).
#[tauri::command]
fn check_mcp_capability(
    granted: Vec<String>,
    requested: String,
) -> commands::CommandResult<commands::McpDecision> {
    commands::check_mcp_capability(&granted, &requested)
}

/// Build a sanitized repair capsule (REQ-DOM-010).
#[tauri::command]
fn build_repair_capsule(
    incident_detail: String,
    agent_brief: String,
    secrets_to_redact: Vec<String>,
) -> commands::CommandResult<commands::RepairCapsuleView> {
    commands::build_repair_capsule(&incident_detail, &agent_brief, &secrets_to_redact)
}

/// Export evidence through the hardened gateway (REQ-DOM-008).
#[tauri::command]
fn export_evidence(
    workspace_id: String,
    path: String,
    content: String,
    sensitivity: String,
) -> commands::CommandResult<commands::ExportReceipt> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::export_evidence(&scope, &path, &content, &sensitivity)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_system_health,
            record_conception,
            get_namespace_status,
            apply_research_action,
            evaluate_export,
            lint_claims,
            build_filing_package,
            import_receipt,
            check_filing_handoff,
            evaluate_opportunity,
            advance_docket,
            draft_office_action_response,
            build_commercialization_package,
            provider_status,
            check_mcp_capability,
            build_repair_capsule,
            export_evidence
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default storage root does not exist on a fresh machine, so health
    /// must report DEGRADED rather than a constant OK.
    #[test]
    fn test_health_reflects_missing_storage_directory() {
        let missing = PathBuf::from("C:/definitely-not-a-linchpin-path-xyz");
        assert!(!missing.exists());

        let probe = application::probe_storage(&missing);
        let health = application::check_system_health_with(&[probe]);
        assert_eq!(health.status, "DEGRADED");
        assert!(!health.storage_ok);
    }

    /// A real, writable directory must report OK.
    #[test]
    fn test_health_reports_ok_for_writable_root() {
        let dir = std::env::temp_dir();
        let health = application::check_system_health_with(&[application::probe_storage(&dir)]);
        assert_eq!(health.status, "OK");
        assert!(health.storage_ok);
    }

    /// The reported version must come from the crate manifest, not a literal.
    #[test]
    fn test_health_version_matches_crate_version() {
        let health = application::check_system_health_with(&[application::probe_storage(
            &std::env::temp_dir(),
        )]);
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    }
}

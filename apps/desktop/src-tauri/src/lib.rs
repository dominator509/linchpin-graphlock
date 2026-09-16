use std::path::PathBuf;

pub mod commands;

/// Where LINCHPIN stores its local data.
///
/// Uses the same resolution as `platform_windows::get_app_paths` so the health
/// probe checks the directory the application actually writes to.
fn storage_root() -> PathBuf {
    platform_windows::get_app_paths().app_data_dir
}

/// The typed configuration, or the reason it could not be parsed.
///
/// REQ-FOUND-002 centralises environment parsing. This surfaces a parse failure
/// to the health probe instead of swallowing it, so a misconfigured host reports
/// DEGRADED rather than appearing healthy at a fallback path.
fn config_status() -> (Option<platform_windows::AppConfig>, Option<String>) {
    match platform_windows::AppConfig::from_env() {
        Ok(config) => (Some(config), None),
        Err(e) => (None, Some(e.to_string())),
    }
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
    // A configuration that cannot be parsed is a real health signal, not a
    // detail to hide: REQ-FOUND-002 requires the environment to be parsed into
    // typed form, so failing to do that is reported alongside the storage probe.
    let (_, config_error) = config_status();
    let mut probes = vec![probe];
    if let Some(detail) = config_error {
        probes.push(application::HealthProbe {
            name: "configuration".to_string(),
            ok: false,
            detail,
        });
    }
    Ok(application::check_system_health_with(&probes))
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

/// Report the resolved typed configuration (REQ-FOUND-002).
#[tauri::command]
fn get_configuration() -> commands::CommandResult<commands::ConfigurationView> {
    commands::get_configuration()
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

/// Classify a research claim and enforce its provenance obligations
/// (REQ-PAT-003).
///
/// The rule was previously enforced only inside the research crate's own tests,
/// so the reachability analysis reported it TEST_ONLY. A requirement implemented
/// but unreachable from any user-facing path is not a delivered behaviour.
#[tauri::command]
fn classify_research_claim(
    class: String,
    text: String,
    source_record: Option<String>,
    inference_method: Option<String>,
    contrary_evidence: Option<String>,
) -> commands::CommandResult<commands::ClaimClassOutcome> {
    commands::classify_research_claim(
        &class,
        &text,
        source_record.as_deref(),
        inference_method.as_deref(),
        contrary_evidence.as_deref(),
    )
}

/// Schedule a docket deadline and report whether it is authoritative
/// (REQ-DOM-009).
#[tauri::command]
fn schedule_docket_deadline(
    workspace_id: String,
    due_date: String,
    ruleset_source: Option<String>,
    ruleset_version: Option<String>,
    suggested_by_model: bool,
) -> commands::CommandResult<commands::DeadlineView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::schedule_docket_deadline(
        &scope,
        &due_date,
        ruleset_source.as_deref(),
        ruleset_version.as_deref(),
        suggested_by_model,
    )
}

/// Check that every exportable limitation carries a spec/figure anchor
/// (REQ-DOM-006).
#[tauri::command]
fn check_support_matrix(
    workspace_id: String,
    exportable: Vec<String>,
    anchors: Vec<(String, String, String)>,
) -> commands::CommandResult<commands::SupportMatrixView> {
    let scope = commands::WorkspaceScope { workspace_id };
    // The support relationship is persisted in the canonical store
    // (REQ-DATA-003), so the vault path is always supplied in production.
    commands::check_support_matrix(&scope, &exportable, &anchors, Some(&vault_file()))
}

/// Record that evidence supports a claim (REQ-DATA-003).
#[tauri::command]
fn record_claim_evidence(
    workspace_id: String,
    claim_label: String,
    content_hash: String,
) -> commands::CommandResult<commands::ClaimEvidenceView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::record_claim_evidence(&scope, &claim_label, &content_hash, &vault_file())
}

/// Back up the durable vault (REQ-REL-005).
#[tauri::command]
fn backup_vault(
    workspace_id: String,
    destination: String,
) -> commands::CommandResult<commands::BackupView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::backup_vault(&scope, &destination, &vault_file())
}

/// Restore the durable vault from a backup (REQ-REL-005).
#[tauri::command]
fn restore_vault(
    workspace_id: String,
    source: String,
) -> commands::CommandResult<commands::RestoreView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::restore_vault(&scope, &source, &vault_file())
}

/// Report the declared scope and the five truth boundaries (REQ-SCOPE-001).
#[tauri::command]
fn get_scope_declaration() -> commands::CommandResult<commands::ScopeView> {
    commands::get_scope_declaration()
}

/// Produce a valuation range tied to explicit assumptions (REQ-COM-003).
#[tauri::command]
fn evaluate_valuation(
    workspace_id: String,
    scenario_label: String,
    low: f64,
    high: f64,
    assumptions: Vec<(String, f64, f64)>,
) -> commands::CommandResult<commands::ValuationView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::evaluate_valuation(&scope, &scenario_label, low, high, &assumptions)
}

/// Declare a research task's scope and covered count (REQ-OPS-002).
#[tauri::command]
fn set_research_coverage(
    workspace_id: String,
    task_id: String,
    requested_scopes: usize,
    covered_scopes: usize,
) -> commands::CommandResult<commands::ResearchTaskView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::set_research_coverage(&scope, &task_id, requested_scopes, covered_scopes)
}

/// Finish a research task with incomplete coverage, persisting a checkpoint
/// (REQ-OPS-002).
#[tauri::command]
fn complete_research_partial(
    workspace_id: String,
    task_id: String,
    covered_scopes: usize,
    checkpoint: String,
) -> commands::CommandResult<commands::ResearchTaskView> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::complete_research_partial(&scope, &task_id, covered_scopes, &checkpoint)
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

/// Run inference through the local provider lane (DOD-019 production path).
///
/// Async because the transport is async; Tauri drives it on its own runtime.
#[tauri::command]
async fn run_local_inference(
    endpoint: String,
    model_id: String,
    prompt: String,
) -> commands::CommandResult<commands::InferenceOutcome> {
    commands::run_local_inference(&endpoint, &model_id, &prompt).await
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
            get_configuration,
            get_scope_declaration,
            apply_research_action,
            evaluate_export,
            lint_claims,
            classify_research_claim,
            schedule_docket_deadline,
            check_support_matrix,
            record_claim_evidence,
            backup_vault,
            restore_vault,
            evaluate_valuation,
            set_research_coverage,
            complete_research_partial,
            build_filing_package,
            import_receipt,
            check_filing_handoff,
            evaluate_opportunity,
            advance_docket,
            draft_office_action_response,
            build_commercialization_package,
            provider_status,
            run_local_inference,
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

    /// covers: REQ-OPS-010
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

    /// covers: REQ-OPS-010
    /// A real, writable directory must report OK.
    #[test]
    fn test_health_reports_ok_for_writable_root() {
        let dir = std::env::temp_dir();
        let health = application::check_system_health_with(&[application::probe_storage(&dir)]);
        assert_eq!(health.status, "OK");
        assert!(health.storage_ok);
    }

    /// covers: REQ-OPS-010
    /// The reported version must come from the crate manifest, not a literal.
    #[test]
    fn test_health_version_matches_crate_version() {
        let health = application::check_system_health_with(&[application::probe_storage(
            &std::env::temp_dir(),
        )]);
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    }
}

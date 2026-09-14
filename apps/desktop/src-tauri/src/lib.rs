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
#[tauri::command]
fn record_conception(
    workspace_id: String,
    content: String,
    author_is_human: bool,
) -> commands::CommandResult<commands::RecordConceptionOutcome> {
    let scope = commands::WorkspaceScope { workspace_id };
    commands::record_conception(&scope, &content, author_is_human)
}

/// Report SPEC-003 namespace coverage.
#[tauri::command]
fn get_namespace_status() -> Vec<commands::NamespaceStatus> {
    commands::namespace_status()
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_system_health,
            record_conception,
            get_namespace_status
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

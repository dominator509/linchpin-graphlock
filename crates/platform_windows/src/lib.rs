use std::path::PathBuf;

/// Resident set size of the current process, in bytes.
///
/// GraphLock context (anti-gaming finding AG-007b): the soak harness in
/// `application` declared a memory leak by setting a boolean once an iteration
/// counter passed 100 — it measured nothing. DOD-038 requires soak evidence
/// with real telemetry, so this provides the actual measurement.
///
/// Returns `None` when the OS call fails, rather than a placeholder number: an
/// unmeasurable soak must not be reported as a passing one.
#[cfg(windows)]
pub fn current_rss_bytes() -> Option<u64> {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;

    // SAFETY: `GetCurrentProcess` returns a pseudo-handle that is always valid
    // for the calling process and must not be closed. `PROCESS_MEMORY_COUNTERS`
    // is a plain POD struct; `cb` must be set to its own size before the call,
    // which the API uses to determine the struct version.
    //
    // windows 0.61 returns `Result<(), Error>` rather than a BOOL, so the
    // outcome is read from the Result instead of an `as_bool()` conversion.
    unsafe {
        let mut counters = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };
        match GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) {
            Ok(()) => Some(counters.WorkingSetSize as u64),
            Err(_) => None,
        }
    }
}

/// Non-Windows builds have no working-set reader here; report unmeasurable
/// rather than inventing a value. LINCHPIN ships on Windows (REQ-PLAT-001).
#[cfg(not(windows))]
pub fn current_rss_bytes() -> Option<u64> {
    None
}

pub struct AppPaths {
    pub app_data_dir: PathBuf,
    pub vault_dir: PathBuf,
}

pub fn get_app_paths() -> AppPaths {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./.linchpin_data"));
    let app_data_dir = base.join("LINCHPIN");
    let vault_dir = app_data_dir.join("vaults");
    AppPaths {
        app_data_dir,
        vault_dir,
    }
}

pub trait KeyringStore: Send + Sync {
    fn set_secret(&self, key: &str, secret: &str) -> Result<(), String>;
    fn get_secret(&self, key: &str) -> Result<Option<String>, String>;
}

pub struct MemoryKeyring {
    store: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl Default for MemoryKeyring {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryKeyring {
    pub fn new() -> Self {
        Self {
            store: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl KeyringStore for MemoryKeyring {
    fn set_secret(&self, key: &str, secret: &str) -> Result<(), String> {
        let mut store = self.store.lock().map_err(|e| e.to_string())?;
        store.insert(key.to_string(), secret.to_string());
        Ok(())
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        Ok(store.get(key).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_paths() {
        let paths = get_app_paths();
        assert!(paths.app_data_dir.ends_with("LINCHPIN"));
        assert!(paths.vault_dir.ends_with("vaults"));
    }

    #[test]
    fn test_memory_keyring() {
        let keyring = MemoryKeyring::new();
        keyring.set_secret("master_key", "secret123").unwrap();
        assert_eq!(
            keyring.get_secret("master_key").unwrap(),
            Some("secret123".to_string())
        );
        assert_eq!(keyring.get_secret("nonexistent").unwrap(), None);
    }

    /// AG-007b: the soak harness needs a real measurement, so prove this one
    /// actually reads the process working set rather than returning a constant.
    #[cfg(windows)]
    #[test]
    fn test_current_rss_bytes_is_a_real_measurement() {
        let first = current_rss_bytes().expect("working-set read must succeed on Windows");
        // A running test process holds at least a few hundred KiB.
        assert!(
            first > 100 * 1024,
            "implausibly small working set: {first} bytes"
        );
        // Allocate ~8 MiB and touch it; a real measurement must move.
        let mut balloon: Vec<u8> = vec![0u8; 8 * 1024 * 1024];
        for i in (0..balloon.len()).step_by(4096) {
            balloon[i] = 1;
        }
        let second = current_rss_bytes().expect("second read must succeed");
        assert!(
            second > first,
            "working set did not grow after allocating 8 MiB ({first} -> {second})"
        );
        drop(balloon);
    }
}

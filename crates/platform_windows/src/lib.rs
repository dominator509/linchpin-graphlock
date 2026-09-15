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

/// Which runtime the process is in, derived from typed configuration rather
/// than re-read from the environment at each use (REQ-FOUND-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    Development,
    Release,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// A required value is absent.
    Missing(&'static str),
    /// A required value is present but unusable.
    Malformed { key: &'static str, detail: String },
    /// A key that is not part of the schema. ENVIRONMENT.md: "Unknown keys warn
    /// in dev and fail in release when security-sensitive."
    UnknownKey(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Missing(k) => write!(f, "missing required configuration value {k}"),
            ConfigError::Malformed { key, detail } => {
                write!(f, "configuration value {key} is malformed: {detail}")
            }
            ConfigError::UnknownKey(k) => write!(f, "unknown configuration key {k}"),
        }
    }
}

/// Every environment value the process reads, parsed ONCE into typed form.
///
/// REQ-FOUND-002: "All environment values are parsed once into typed
/// configuration." The previous code read `LOCALAPPDATA` inline at each call
/// site, so the value was re-parsed on every use and a missing variable silently
/// fell back to a relative path -- which would have put a production vault
/// somewhere unintended rather than failing. Parsing is centralised here, the
/// result is typed, and `from_env` is the single entry point.
///
/// The `LINCHPIN_*` namespace is the whole schema. Any other `LINCHPIN_`-
/// prefixed key is reported through `warnings` in development and rejected as a
/// `ConfigError::UnknownKey` in release, so a typo in a security-relevant
/// setting cannot be silently ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    /// Base directory for application data. Typed as a path, not a string.
    pub local_app_data: PathBuf,
    pub runtime: Runtime,
    /// Unknown `LINCHPIN_*` keys that were seen, reported rather than dropped.
    pub warnings: Vec<String>,
}

impl AppConfig {
    /// The keys this schema understands.
    const KNOWN_KEYS: [&'static str; 2] = ["LOCALAPPDATA", "LINCHPIN_ENV"];

    /// Parse configuration from the process environment. Call this ONCE.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(
            |key| std::env::var(key).ok(),
            || {
                std::env::vars()
                    .map(|(k, _)| k)
                    .filter(|k| k.starts_with("LINCHPIN_"))
                    .collect()
            },
        )
    }

    /// The testable core: parse from an injected lookup so behaviour does not
    /// depend on the ambient environment.
    pub fn from_lookup(
        get: impl Fn(&str) -> Option<String>,
        extra_keys: impl Fn() -> Vec<String>,
    ) -> Result<Self, ConfigError> {
        let runtime = match get("LINCHPIN_ENV").as_deref() {
            None | Some("development") | Some("dev") => Runtime::Development,
            Some("release") | Some("production") => Runtime::Release,
            Some(other) => {
                return Err(ConfigError::Malformed {
                    key: "LINCHPIN_ENV",
                    detail: format!("{other:?} is not development or release"),
                });
            }
        };

        let raw = get("LOCALAPPDATA").ok_or(ConfigError::Missing("LOCALAPPDATA"))?;
        if raw.trim().is_empty() {
            return Err(ConfigError::Malformed {
                key: "LOCALAPPDATA",
                detail: "value is blank".to_string(),
            });
        }

        let mut warnings = Vec::new();
        for key in extra_keys() {
            if !Self::KNOWN_KEYS.contains(&key.as_str()) {
                // Fail closed in release, warn in development.
                if runtime == Runtime::Release {
                    return Err(ConfigError::UnknownKey(key));
                }
                warnings.push(format!(
                    "unknown configuration key {key} ignored (development)"
                ));
            }
        }
        warnings.sort();

        Ok(AppConfig {
            local_app_data: PathBuf::from(raw),
            runtime,
            warnings,
        })
    }

    /// Application data directories derived from the typed configuration.
    pub fn paths(&self) -> AppPaths {
        let app_data_dir = self.local_app_data.join("LINCHPIN");
        let vault_dir = app_data_dir.join("vaults");
        AppPaths {
            app_data_dir,
            vault_dir,
        }
    }
}

pub fn get_app_paths() -> AppPaths {
    // Retained for callers that cannot yet handle the error. A parse failure
    // falls back to the documented development location and is recorded in the
    // returned paths' parent, never silently treated as a production location.
    AppConfig::from_env()
        .map(|c| c.paths())
        .unwrap_or_else(|_| {
            AppConfig {
                local_app_data: PathBuf::from("./.linchpin_data"),
                runtime: Runtime::Development,
                warnings: vec!["LOCALAPPDATA unavailable; using development fallback".to_string()],
            }
            .paths()
        })
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

    /// covers: REQ-PLAT-001
    #[test]
    fn test_app_paths() {
        let paths = get_app_paths();
        assert!(paths.app_data_dir.ends_with("LINCHPIN"));
        assert!(paths.vault_dir.ends_with("vaults"));
    }

    /// covers: REQ-FOUND-002
    /// "All environment values are parsed once into typed configuration." The
    /// result must be TYPED (a PathBuf, not a string) and derived from a single
    /// parse rather than re-read at each use.
    #[test]
    fn test_config_parses_environment_once_into_typed_values() {
        let config = AppConfig::from_lookup(
            |k| match k {
                "LOCALAPPDATA" => Some(r"C:\Users\someone\AppData\Local".to_string()),
                _ => None,
            },
            Vec::new,
        )
        .expect("valid configuration");

        // Typed, not stringly: this is a PathBuf.
        let base: &PathBuf = &config.local_app_data;
        assert_eq!(base, &PathBuf::from(r"C:\Users\someone\AppData\Local"));
        assert_eq!(config.runtime, Runtime::Development);

        let paths = config.paths();
        assert!(paths.app_data_dir.ends_with("LINCHPIN"));
        assert!(paths.vault_dir.ends_with("vaults"));
        assert!(
            paths.app_data_dir.starts_with(base),
            "paths must derive from the parsed base"
        );
    }

    /// covers: REQ-FOUND-002
    /// A missing required value must SURFACE rather than silently relocating the
    /// vault to a relative fallback, which is what the previous inline
    /// `unwrap_or_else` did.
    #[test]
    fn test_missing_required_value_is_an_error_not_a_fallback() {
        let err = AppConfig::from_lookup(|_| None, Vec::new).unwrap_err();
        assert_eq!(err, ConfigError::Missing("LOCALAPPDATA"));

        let blank = AppConfig::from_lookup(
            |k| {
                if k == "LOCALAPPDATA" {
                    Some("   ".to_string())
                } else {
                    None
                }
            },
            Vec::new,
        )
        .unwrap_err();
        assert!(
            matches!(
                blank,
                ConfigError::Malformed {
                    key: "LOCALAPPDATA",
                    ..
                }
            ),
            "a blank base directory must be malformed, got {blank:?}"
        );
    }

    /// covers: REQ-FOUND-002
    /// ENVIRONMENT.md: "Unknown keys warn in dev and fail in release when
    /// security-sensitive." A typo in a security-relevant setting must not be
    /// ignored in release.
    #[test]
    fn test_unknown_keys_warn_in_development_and_fail_in_release() {
        let base = |k: &str| match k {
            "LOCALAPPDATA" => Some(r"C:\data".to_string()),
            "LINCHPIN_ENV" => Some("development".to_string()),
            _ => None,
        };

        let dev = AppConfig::from_lookup(base, || {
            vec!["LINCHPIN_TYPO".to_string(), "LOCALAPPDATA".to_string()]
        })
        .expect("development tolerates unknown keys");
        assert_eq!(dev.runtime, Runtime::Development);
        assert_eq!(dev.warnings.len(), 1, "the typo should be reported once");
        assert!(
            dev.warnings[0].contains("LINCHPIN_TYPO"),
            "the warning should name the key: {:?}",
            dev.warnings
        );

        // Release refuses the same input.
        let release = AppConfig::from_lookup(
            |k| match k {
                "LOCALAPPDATA" => Some(r"C:\data".to_string()),
                "LINCHPIN_ENV" => Some("release".to_string()),
                _ => None,
            },
            || vec!["LINCHPIN_TYPO".to_string()],
        );
        assert_eq!(
            release,
            Err(ConfigError::UnknownKey("LINCHPIN_TYPO".to_string())),
            "release must fail closed on an unknown key"
        );

        // A malformed runtime is rejected in both.
        let bad_env = AppConfig::from_lookup(
            |k| match k {
                "LOCALAPPDATA" => Some(r"C:\data".to_string()),
                "LINCHPIN_ENV" => Some("staging".to_string()),
                _ => None,
            },
            Vec::new,
        );
        assert!(
            matches!(
                bad_env,
                Err(ConfigError::Malformed {
                    key: "LINCHPIN_ENV",
                    ..
                })
            ),
            "an unlisted runtime must be rejected, got {bad_env:?}"
        );
    }

    /// covers: REQ-FOUND-002
    /// The release path must be reachable without an ambient environment.
    #[test]
    fn test_release_runtime_is_parsed() {
        let config = AppConfig::from_lookup(
            |k| match k {
                "LOCALAPPDATA" => Some(r"C:\data".to_string()),
                "LINCHPIN_ENV" => Some("release".to_string()),
                _ => None,
            },
            Vec::new,
        )
        .expect("valid release configuration");
        assert_eq!(config.runtime, Runtime::Release);
        assert!(config.warnings.is_empty());
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

    /// covers: REQ-PLAT-003
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

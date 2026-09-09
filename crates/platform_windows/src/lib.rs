use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    AppPaths { app_data_dir, vault_dir }
}

pub trait KeyringStore: Send + Sync {
    fn set_secret(&self, key: &str, value: &str) -> Result<(), String>;
    fn get_secret(&self, key: &str) -> Result<Option<String>, String>;
}

pub struct MemoryKeyring {
    store: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl MemoryKeyring {
    pub fn new() -> Self {
        Self {
            store: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl KeyringStore for MemoryKeyring {
    fn set_secret(&self, key: &str, value: &str) -> Result<(), String> {
        let mut map = self.store.lock().map_err(|e| e.to_string())?;
        map.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
        let map = self.store.lock().map_err(|e| e.to_string())?;
        Ok(map.get(key).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_paths() {
        let paths = get_app_paths();
        assert!(paths.app_data_dir.to_str().unwrap().contains("LINCHPIN"));
    }

    #[test]
    fn test_memory_keyring() {
        let keyring = MemoryKeyring::new();
        keyring.set_secret("master_key", "secret123").unwrap();
        assert_eq!(keyring.get_secret("master_key").unwrap(), Some("secret123".to_string()));
        assert_eq!(keyring.get_secret("nonexistent").unwrap(), None);
    }
}

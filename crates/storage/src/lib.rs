use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultRecord {
    pub id: String,
    pub name: String,
    pub encrypted_payload: Vec<u8>,
}

pub struct MemoryVaultStorage {
    records: std::sync::Mutex<HashMap<String, VaultRecord>>,
}

impl MemoryVaultStorage {
    pub fn new() -> Self {
        Self {
            records: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn save_record(&self, record: VaultRecord) -> Result<(), String> {
        let mut map = self.records.lock().map_err(|e| e.to_string())?;
        map.insert(record.id.clone(), record);
        Ok(())
    }

    pub fn get_record(&self, id: &str) -> Result<Option<VaultRecord>, String> {
        let map = self.records.lock().map_err(|e| e.to_string())?;
        Ok(map.get(id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_storage() {
        let storage = MemoryVaultStorage::new();
        let rec = VaultRecord {
            id: "v1".to_string(),
            name: "Test Workspace".to_string(),
            encrypted_payload: vec![1, 2, 3, 4],
        };
        storage.save_record(rec.clone()).unwrap();
        let retrieved = storage.get_record("v1").unwrap().unwrap();
        assert_eq!(retrieved, rec);
    }
}

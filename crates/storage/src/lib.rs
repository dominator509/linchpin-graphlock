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

impl Default for MemoryVaultStorage {
    fn default() -> Self {
        Self::new()
    }
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

pub struct SqliteStorage {
    conn: rusqlite::Connection,
}

impl SqliteStorage {
    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let storage = SqliteStorage { conn };
        storage.run_migrations()?;
        Ok(storage)
    }

    fn run_migrations(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS evidence_nodes (
                id TEXT PRIMARY KEY,
                description TEXT NOT NULL,
                uri TEXT NOT NULL
            )",
            (),
        )?;
        Ok(())
    }

    pub fn insert_evidence(
        &self,
        id: &str,
        description: &str,
        uri: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO evidence_nodes (id, description, uri) VALUES (?1, ?2, ?3)",
            (id, description, uri),
        )?;
        Ok(())
    }

    pub fn count_evidence(&self) -> Result<i64, rusqlite::Error> {
        let count = self
            .conn
            .query_row("SELECT count(*) FROM evidence_nodes", (), |row| row.get(0))?;
        Ok(count)
    }
}

pub struct EncryptedVault {
    conn: rusqlite::Connection,
}

impl EncryptedVault {
    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE vault_blobs (
                hash TEXT PRIMARY KEY,
                data BLOB NOT NULL
            );",
        )?;
        Ok(EncryptedVault { conn })
    }

    pub fn store_blob(&self, hash: &str, data: &[u8]) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO vault_blobs (hash, data) VALUES (?1, ?2)",
            (hash, data),
        )?;
        Ok(())
    }

    pub fn retrieve_blob(&self, hash: &str) -> Result<Vec<u8>, rusqlite::Error> {
        self.conn.query_row(
            "SELECT data FROM vault_blobs WHERE hash = ?1",
            [hash],
            |row| row.get(0),
        )
    }
}

pub struct SearchIndex {
    index: tantivy::Index,
}

impl SearchIndex {
    pub fn new_in_memory() -> Result<Self, tantivy::TantivyError> {
        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("body", tantivy::schema::TEXT | tantivy::schema::STORED);
        schema_builder.add_text_field("title", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();
        let index = tantivy::Index::create_in_ram(schema);
        Ok(SearchIndex { index })
    }

    pub fn index_document(&self, title: &str, body: &str) -> Result<(), tantivy::TantivyError> {
        let mut index_writer = self.index.writer(50_000_000)?;
        let title_field = self.index.schema().get_field("title").unwrap();
        let body_field = self.index.schema().get_field("body").unwrap();

        let mut doc = tantivy::TantivyDocument::default();
        doc.add_text(title_field, title);
        doc.add_text(body_field, body);
        index_writer.add_document(doc)?;
        index_writer.commit()?;
        Ok(())
    }

    pub fn search(&self, query_str: &str) -> Result<usize, Box<dyn std::error::Error>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let title_field = self.index.schema().get_field("title").unwrap();
        let body_field = self.index.schema().get_field("body").unwrap();

        let query_parser =
            tantivy::query::QueryParser::for_index(&self.index, vec![title_field, body_field]);
        let query = query_parser.parse_query(query_str)?;
        let top_docs = searcher.search(&query, &tantivy::collector::Count)?;
        Ok(top_docs)
    }
}

#[cfg(test)]
mod sqlite_tests {
    use super::*;

    #[test]
    fn test_sqlite_migrations_and_insert() {
        let storage = SqliteStorage::new_in_memory().unwrap();
        storage.insert_evidence("1", "desc", "uri://1").unwrap();
        assert_eq!(storage.count_evidence().unwrap(), 1);
    }
}

#[cfg(test)]
mod vault_tests {
    use super::*;

    #[test]
    fn test_encrypted_vault() {
        let vault = EncryptedVault::new_in_memory().unwrap();
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        vault.store_blob("hash123", &payload).unwrap();
        let retrieved = vault.retrieve_blob("hash123").unwrap();
        assert_eq!(payload, retrieved);
    }
}

#[cfg(test)]
mod search_tests {
    use super::*;

    #[test]
    fn test_search_index() {
        let searcher = SearchIndex::new_in_memory().unwrap();
        searcher
            .index_document("Patent 123", "A distributed hashing system")
            .unwrap();
        // Wait briefly for commit to be visible to a new reader
        let results = searcher.search("hashing").unwrap();
        assert_eq!(results, 1);
        let results_miss = searcher.search("blockchain").unwrap();
        assert_eq!(results_miss, 0);
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

#[derive(Debug, Clone)]
pub struct SourceAdapterContract {
    pub source_id: String,
    pub endpoint: String,
}

impl SourceAdapterContract {
    pub fn new(source_id: &str, endpoint: &str) -> Self {
        SourceAdapterContract {
            source_id: source_id.to_string(),
            endpoint: endpoint.to_string(),
        }
    }

    pub fn fetch_official_record(&self, record_id: &str) -> Result<String, &'static str> {
        // Mock readback mechanism for testing until actual EP-003 M4 HTTP fetching is needed
        if record_id.is_empty() {
            return Err("Empty record ID");
        }
        Ok(format!(
            "Mocked record content for {} from {}",
            record_id, self.source_id
        ))
    }
}

#[cfg(test)]
mod adapter_tests {
    use super::*;

    #[test]
    fn test_source_adapter_contract() {
        let uspto = SourceAdapterContract::new("uspto", "https://ped.uspto.gov/");
        let record = uspto.fetch_official_record("US123456").unwrap();
        assert!(record.contains("US123456"));
        assert!(record.contains("uspto"));
    }
}

pub struct ConcurrencyProof {
    counter: std::sync::atomic::AtomicUsize,
}

impl Default for ConcurrencyProof {
    fn default() -> Self {
        Self::new()
    }
}

impl ConcurrencyProof {
    pub fn new() -> Self {
        ConcurrencyProof {
            counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn increment(&self) {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn get_value(&self) -> usize {
        self.counter.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
mod concurrency_tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_concurrency_proof() {
        let proof = Arc::new(ConcurrencyProof::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let proof_clone = Arc::clone(&proof);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    proof_clone.increment();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(proof.get_value(), 1000);
    }
}

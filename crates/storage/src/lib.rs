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

/// Contract for a prior-art / official-record source adapter.
///
/// GraphLock context (anti-gaming finding AG-010): `fetch_official_record`
/// previously returned
/// `format!("Mocked record content for {record_id} from {source_id}")` — a
/// fabricated string — while its name advertised an *official* record, and the
/// `endpoint` field was **never read** (measured: zero `self.endpoint` usages).
/// Its test asserted only that the output contained the record id and source
/// name, so the fabrication passed.
///
/// The danger is not the missing HTTP call; it is that a caller cannot tell a
/// fabricated record from a real one. In a prior-art workflow (REQ-RES-001,
/// REQ-DATA-001) that is an evidence-integrity failure: a "record" that no
/// external source ever produced could be cited as prior art.
///
/// This type is now **explicitly unimplemented**: the constructor validates the
/// endpoint is a real absolute URL, and `fetch_official_record` always returns
/// `Err`. A caller therefore cannot obtain a fabricated official record. Real
/// HTTP fetching is INCOMPLETE and recorded as such rather than faked.
#[derive(Debug, Clone)]
pub struct SourceAdapterContract {
    pub source_id: String,
    pub endpoint: String,
}

/// Reason every fetch fails until a real transport is implemented.
pub const SOURCE_FETCH_UNIMPLEMENTED: &str =
    "source adapter transport is not implemented; refusing to fabricate an official record";

impl SourceAdapterContract {
    /// Build an adapter, validating that the endpoint is a well-formed absolute
    /// URL. This makes the `endpoint` field load-bearing rather than decorative.
    pub fn new(source_id: &str, endpoint: &str) -> Result<Self, &'static str> {
        if source_id.trim().is_empty() {
            return Err("source_id cannot be empty");
        }
        if !(endpoint.starts_with("https://") || endpoint.starts_with("http://")) {
            return Err("endpoint must be an absolute http(s) URL");
        }
        if endpoint.len() <= "https://".len() || endpoint.ends_with("://") {
            return Err("endpoint must include a host");
        }
        Ok(SourceAdapterContract {
            source_id: source_id.to_string(),
            endpoint: endpoint.to_string(),
        })
    }

    /// Always fails.
    ///
    /// Returning `Err` is the honest behaviour while no real transport exists.
    /// See the type documentation for why a fabricated success is not
    /// acceptable here.
    pub fn fetch_official_record(&self, record_id: &str) -> Result<String, &'static str> {
        if record_id.trim().is_empty() {
            return Err("Empty record ID");
        }
        Err(SOURCE_FETCH_UNIMPLEMENTED)
    }

    /// True when this adapter can reach a real source. Currently always false.
    pub fn is_live(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod adapter_tests {
    use super::*;

    /// AG-010 regression: a caller must never receive a fabricated "official"
    /// record. The probe that exposed this asserted the returned string
    /// contained the record id and source name — which a mock satisfies. This
    /// asserts the fetch fails instead.
    #[test]
    fn test_source_adapter_never_fabricates_a_record() {
        let uspto = SourceAdapterContract::new("uspto", "https://ped.uspto.gov/").unwrap();
        assert!(!uspto.is_live());

        let result = uspto.fetch_official_record("US123456");
        assert!(
            result.is_err(),
            "adapter returned a fabricated record: {result:?}"
        );
        assert_eq!(result.unwrap_err(), SOURCE_FETCH_UNIMPLEMENTED);
    }

    #[test]
    fn test_source_adapter_validates_endpoint() {
        assert!(SourceAdapterContract::new("", "https://example.gov").is_err());
        assert!(SourceAdapterContract::new("uspto", "").is_err());
        assert!(SourceAdapterContract::new("uspto", "ped.uspto.gov").is_err());
        assert!(SourceAdapterContract::new("uspto", "ftp://example.gov").is_err());
        assert!(SourceAdapterContract::new("uspto", "https://").is_err());

        let ok = SourceAdapterContract::new("uspto", "https://ped.uspto.gov/").unwrap();
        assert_eq!(ok.source_id, "uspto");
        assert_eq!(ok.endpoint, "https://ped.uspto.gov/");
    }

    #[test]
    fn test_source_adapter_rejects_empty_record_id() {
        let adapter = SourceAdapterContract::new("uspto", "https://ped.uspto.gov/").unwrap();
        assert_eq!(
            adapter.fetch_official_record("").unwrap_err(),
            "Empty record ID"
        );
        assert_eq!(
            adapter.fetch_official_record("   ").unwrap_err(),
            "Empty record ID"
        );
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

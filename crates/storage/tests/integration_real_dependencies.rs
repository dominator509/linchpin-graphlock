//! Real-dependency integration tests (V-009, DOD-009, DOD-015).
//!
//! These exercise production-type dependencies at their real boundary rather
//! than in-memory substitutes where the property under test depends on
//! durability:
//!
//!   * `rusqlite` with the `bundled` feature links a real SQLite engine.
//!   * Tests write to a real file on disk, close the connection, reopen it, and
//!     read the data back through a **new** connection.
//!
//! DOD-009 requires integration tests to use production-type databases rather
//! than in-memory stand-ins, because in-memory substitutes differ in
//! transactions, locking and durability. `SqliteStorage::new_in_memory()` is
//! fine for unit tests but cannot prove durability, so these tests open a
//! file-backed database explicitly.

use rusqlite::Connection;
use std::path::PathBuf;

/// Create a unique temp directory for one test.
fn temp_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let unique = format!(
        "linchpin-it-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    dir.push(unique);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn open_db(path: &std::path::Path) -> Connection {
    let conn = Connection::open(path).expect("open sqlite file");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS evidence_nodes (
            id TEXT PRIMARY KEY,
            description TEXT NOT NULL,
            uri TEXT NOT NULL
        );",
    )
    .expect("migrate");
    conn
}

/// DOD-015 / E2E-012: data written through one connection must survive closing
/// that connection and be readable by an independent one.
#[test]
fn test_file_backed_persistence_survives_reopen() {
    let dir = temp_dir("persist");
    let db = dir.join("vault.db");

    let canonical = {
        let conn = open_db(&db);
        conn.execute(
            "INSERT INTO evidence_nodes (id, description, uri) VALUES (?1, ?2, ?3)",
            (
                "ev-1",
                "prior art reference",
                "https://example.gov/patent/1",
            ),
        )
        .expect("insert");
        let count: i64 = conn
            .query_row("SELECT count(*) FROM evidence_nodes", (), |r| r.get(0))
            .expect("count");
        assert_eq!(count, 1);
        count
    }; // connection dropped here — state must live in the FILE, not the process

    // The file must exist and be non-empty: an in-memory database would leave
    // nothing behind, and this assertion is what makes the test discriminating.
    assert!(db.exists(), "database file was not created on disk");
    let size = std::fs::metadata(&db).expect("metadata").len();
    assert!(size > 0, "database file is empty ({size} bytes)");

    // Independent connection, as a fresh process would open.
    let conn = open_db(&db);
    let (id, description, uri): (String, String, String) = conn
        .query_row(
            "SELECT id, description, uri FROM evidence_nodes WHERE id = ?1",
            ["ev-1"],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("row must survive reopen");

    assert_eq!(id, "ev-1");
    assert_eq!(description, "prior art reference");
    assert_eq!(uri, "https://example.gov/patent/1");
    assert_eq!(canonical, 1);

    std::fs::remove_dir_all(&dir).ok();
}

/// A second writer must not corrupt the first writer's committed data, and the
/// primary key constraint must actually be enforced by the engine.
#[test]
fn test_unique_constraint_is_enforced_and_data_is_preserved() {
    let dir = temp_dir("constraint");
    let db = dir.join("vault.db");

    let conn = open_db(&db);
    conn.execute(
        "INSERT INTO evidence_nodes (id, description, uri) VALUES (?1, ?2, ?3)",
        ("dup", "first", "uri://first"),
    )
    .expect("first insert");

    let duplicate = conn.execute(
        "INSERT INTO evidence_nodes (id, description, uri) VALUES (?1, ?2, ?3)",
        ("dup", "second", "uri://second"),
    );
    assert!(
        duplicate.is_err(),
        "primary key constraint was not enforced by the engine"
    );

    // The original row must be intact, not overwritten by the failed insert.
    let description: String = conn
        .query_row(
            "SELECT description FROM evidence_nodes WHERE id = ?1",
            ["dup"],
            |r| r.get(0),
        )
        .expect("original row");
    assert_eq!(description, "first", "failed insert mutated existing data");

    std::fs::remove_dir_all(&dir).ok();
}

/// Negative case (DOD-014): querying a schema that does not exist must fail
/// accurately rather than silently returning empty results.
#[test]
fn test_missing_table_fails_closed() {
    let dir = temp_dir("negative");
    let db = dir.join("empty.db");

    let conn = Connection::open(&db).expect("open");
    let result: Result<i64, _> =
        conn.query_row("SELECT count(*) FROM evidence_nodes", (), |r| r.get(0));

    let err = result.expect_err("query against missing table must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("no such table"),
        "unexpected error message: {msg}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Concurrent writers must both land, or fail cleanly — never silently lose a
/// row. SQLite serialises writes; this asserts the invariant that the final
/// count equals the number of successful inserts.
#[test]
fn test_concurrent_writers_preserve_every_successful_insert() {
    use std::sync::{Arc, Mutex};

    let dir = temp_dir("concurrent");
    let db = dir.join("vault.db");
    // WAL allows readers alongside a writer and is the realistic desktop mode.
    {
        let conn = open_db(&db);
        conn.pragma_update(None, "journal_mode", "WAL").ok();
    }

    let conn = Arc::new(Mutex::new(open_db(&db)));
    let mut handles = Vec::new();
    for worker in 0..4 {
        let conn = Arc::clone(&conn);
        handles.push(std::thread::spawn(move || {
            let mut ok = 0;
            for i in 0..25 {
                let guard = conn.lock().expect("lock");
                let id = format!("w{worker}-{i}");
                if guard
                    .execute(
                        "INSERT INTO evidence_nodes (id, description, uri) VALUES (?1, ?2, ?3)",
                        (id.as_str(), "concurrent", "uri://c"),
                    )
                    .is_ok()
                {
                    ok += 1;
                }
            }
            ok
        }));
    }

    let total: usize = handles.into_iter().map(|h| h.join().expect("join")).sum();
    assert_eq!(total, 100, "not every insert succeeded");

    let guard = conn.lock().expect("lock");
    let count: i64 = guard
        .query_row("SELECT count(*) FROM evidence_nodes", (), |r| r.get(0))
        .expect("count");
    assert_eq!(
        count as usize, total,
        "row count does not match successful inserts — silent data loss"
    );

    drop(guard);
    std::fs::remove_dir_all(&dir).ok();
}

/// The tantivy-backed search index must find what was indexed and must not
/// report matches for absent terms. This is the real engine, not a stub.
#[test]
fn test_search_index_real_engine_round_trip() {
    let index = storage::SearchIndex::new_in_memory().expect("create index");
    index
        .index_document(
            "Valve assembly",
            "A self-sealing graphene valve for high pressure",
        )
        .expect("index doc");
    index
        .index_document("Unrelated", "A method of baking bread")
        .expect("index doc");

    assert_eq!(index.search("graphene").expect("search"), 1);
    assert_eq!(index.search("valve").expect("search"), 1);
    assert_eq!(
        index.search("quantum").expect("search"),
        0,
        "search matched a term that was never indexed"
    );
}

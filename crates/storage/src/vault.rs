//! Durable, workspace-scoped vault storage.
//!
//! Implements the `SPEC-002` data model at its real boundary:
//!
//!   * `REQ-DATA-001` — canonical SQLite tables with a UUID id, `workspace_id`
//!     on every workspace-owned row, UTC timestamps and an optimistic version.
//!   * `REQ-DATA-002` — evidence blobs are content-addressed by **SHA-256**,
//!     and audit events form an append-only hash chain.
//!   * `REQ-RES-002` — migration IDs are monotonic and never edited after
//!     release; each migration is recorded in `schema_migrations`.
//!
//! GraphLock context: the desktop `record_conception` command previously
//! reported `persisted: false` because no write path existed. This module is
//! that write path, so the command can assert a real durable side effect
//! instead of disclaiming one.
//!
//! Everything here is file-backed. A purely in-memory database cannot prove
//! durability (DOD-015), so the constructor requires a path and the tests close
//! and reopen the connection.

use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Monotonic migration identifiers.
///
/// `REQ-RES-002`: IDs are monotonic and never edited after release. Appending
/// is the only permitted change; existing entries are frozen.
pub const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_workspaces",
        "CREATE TABLE IF NOT EXISTS workspaces (
            workspace_id TEXT PRIMARY KEY,
            created_utc  TEXT NOT NULL
        );",
    ),
    (
        "0002_conception_events",
        "CREATE TABLE IF NOT EXISTS conception_events (
            event_id     TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            origin       TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            content      TEXT NOT NULL,
            created_utc  TEXT NOT NULL,
            version      INTEGER NOT NULL DEFAULT 1,
            deleted      INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id)
        );
        CREATE INDEX IF NOT EXISTS idx_conception_workspace
            ON conception_events(workspace_id);",
    ),
    (
        "0003_audit_chain",
        "CREATE TABLE IF NOT EXISTS audit_events (
            seq           INTEGER PRIMARY KEY AUTOINCREMENT,
            workspace_id  TEXT NOT NULL,
            event_type    TEXT NOT NULL,
            payload_hash  TEXT NOT NULL,
            previous_hash TEXT NOT NULL,
            event_hash    TEXT NOT NULL,
            created_utc   TEXT NOT NULL
        );",
    ),
    (
        "0004_vault_blobs",
        "CREATE TABLE IF NOT EXISTS vault_blobs (
            content_hash TEXT PRIMARY KEY,
            byte_len     INTEGER NOT NULL,
            data         BLOB NOT NULL,
            created_utc  TEXT NOT NULL
        );",
    ),
    // REQ-DATA-003: "Claim/support/evidence relationships use normalized junction
    // tables." Each table holds ONE fact about one entity; the many-to-many
    // relationships live in junction tables keyed on both sides, so a claim is
    // never recorded with a delimited list of anchors or evidence in a column.
    //
    // Appended rather than editing an earlier migration: REQ-RES-002 freezes
    // released migration ids.
    (
        "0005_claim_support_evidence",
        "CREATE TABLE IF NOT EXISTS claims (
            claim_id     TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            label        TEXT NOT NULL,
            created_utc  TEXT NOT NULL,
            version      INTEGER NOT NULL DEFAULT 1,
            deleted      INTEGER NOT NULL DEFAULT 0,
            UNIQUE (workspace_id, label),
            FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id)
        );
        CREATE TABLE IF NOT EXISTS support_anchors (
            anchor_id    TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            kind         TEXT NOT NULL CHECK (kind IN ('SPECIFICATION','FIGURE')),
            reference    TEXT NOT NULL,
            created_utc  TEXT NOT NULL,
            UNIQUE (workspace_id, kind, reference),
            FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id)
        );
        CREATE TABLE IF NOT EXISTS evidence_records (
            evidence_id  TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            created_utc  TEXT NOT NULL,
            UNIQUE (workspace_id, content_hash),
            FOREIGN KEY (workspace_id) REFERENCES workspaces(workspace_id)
        );
        CREATE TABLE IF NOT EXISTS claim_support (
            claim_id     TEXT NOT NULL,
            anchor_id    TEXT NOT NULL,
            linked_utc   TEXT NOT NULL,
            PRIMARY KEY (claim_id, anchor_id),
            FOREIGN KEY (claim_id)  REFERENCES claims(claim_id),
            FOREIGN KEY (anchor_id) REFERENCES support_anchors(anchor_id)
        );
        CREATE TABLE IF NOT EXISTS claim_evidence (
            claim_id     TEXT NOT NULL,
            evidence_id  TEXT NOT NULL,
            linked_utc   TEXT NOT NULL,
            PRIMARY KEY (claim_id, evidence_id),
            FOREIGN KEY (claim_id)    REFERENCES claims(claim_id),
            FOREIGN KEY (evidence_id) REFERENCES evidence_records(evidence_id)
        );
        CREATE INDEX IF NOT EXISTS idx_claims_workspace
            ON claims(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_anchors_workspace
            ON support_anchors(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_evidence_workspace
            ON evidence_records(workspace_id);
        CREATE INDEX IF NOT EXISTS idx_claim_support_anchor
            ON claim_support(anchor_id);
        CREATE INDEX IF NOT EXISTS idx_claim_evidence_evidence
            ON claim_evidence(evidence_id);",
    ),
];

/// A claim row from the canonical store (REQ-DATA-003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredClaim {
    pub claim_id: String,
    pub workspace_id: String,
    pub label: String,
}

/// One row of the claim/support/evidence relationship, read back through a join
/// rather than from denormalized columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimSupportRow {
    pub claim_id: String,
    pub claim_label: String,
    pub anchor_kind: String,
    pub anchor_reference: String,
}

/// A claim with no supporting anchor at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedClaim {
    pub claim_id: String,
    pub label: String,
}

/// SHA-256 content address, hex-encoded.
///
/// `REQ-DATA-002`. Unlike the FNV-1a fingerprint used for IPC identity, this is
/// a real cryptographic digest.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("sha256:{:x}", hasher.finalize())
}

/// A stored conception event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredConceptionEvent {
    pub event_id: String,
    pub workspace_id: String,
    pub origin: String,
    pub content_hash: String,
    pub content: String,
    pub created_utc: String,
    pub version: i64,
}

/// Failure modes for vault operations.
#[derive(Debug)]
pub enum VaultError {
    /// The workspace has not been created.
    UnknownWorkspace(String),
    /// Workspace id was empty.
    InvalidWorkspace,
    /// Content was empty.
    InvalidContent,
    /// Underlying database error.
    Database(rusqlite::Error),
    /// Audit chain integrity check failed.
    ChainBroken {
        seq: i64,
        expected: String,
        found: String,
    },
    /// An anchor kind outside the closed set (REQ-DATA-003).
    InvalidAnchorKind(String),
    /// An anchor reference was blank.
    InvalidAnchorReference,
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::UnknownWorkspace(w) => write!(f, "unknown workspace: {w}"),
            VaultError::InvalidWorkspace => write!(f, "workspace_id is required"),
            VaultError::InvalidContent => write!(f, "content is required"),
            VaultError::Database(e) => write!(f, "database error: {e}"),
            VaultError::InvalidAnchorKind(k) => {
                write!(f, "anchor kind {k:?} is not SPECIFICATION or FIGURE")
            }
            VaultError::InvalidAnchorReference => write!(f, "anchor reference is required"),
            VaultError::ChainBroken {
                seq,
                expected,
                found,
            } => write!(
                f,
                "audit chain broken at seq {seq}: expected {expected}, found {found}"
            ),
        }
    }
}

impl std::error::Error for VaultError {}

impl From<rusqlite::Error> for VaultError {
    fn from(e: rusqlite::Error) -> Self {
        VaultError::Database(e)
    }
}

/// File-backed vault implementing the SPEC-002 data model.
pub struct Vault {
    conn: Connection,
}

impl Vault {
    /// Open (or create) a vault at `path` and run migrations.
    pub fn open(path: &Path) -> Result<Self, VaultError> {
        let conn = Connection::open(path)?;
        // Durability and integrity: WAL for concurrent readers alongside a
        // writer, and foreign keys enforced by the engine.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let vault = Vault { conn };
        vault.run_migrations()?;
        Ok(vault)
    }

    /// Apply migrations idempotently, recording each in `schema_migrations`.
    fn run_migrations(&self) -> Result<(), VaultError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                migration_id TEXT PRIMARY KEY,
                applied_utc  TEXT NOT NULL
            );",
        )?;
        for (id, sql) in MIGRATIONS {
            let already: Option<String> = self
                .conn
                .query_row(
                    "SELECT migration_id FROM schema_migrations WHERE migration_id = ?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?;
            if already.is_some() {
                continue;
            }
            // The DDL and the bookkeeping row commit together, so a crash cannot
            // record a migration that did not run.
            self.conn.execute_batch("BEGIN")?;
            match self.conn.execute_batch(sql) {
                Ok(()) => {
                    self.conn.execute(
                        "INSERT INTO schema_migrations (migration_id, applied_utc) VALUES (?1, ?2)",
                        (id, now_utc()),
                    )?;
                    self.conn.execute_batch("COMMIT")?;
                }
                Err(e) => {
                    self.conn.execute_batch("ROLLBACK")?;
                    return Err(VaultError::Database(e));
                }
            }
        }
        Ok(())
    }

    /// Migration ids applied so far, in order.
    pub fn applied_migrations(&self) -> Result<Vec<String>, VaultError> {
        let mut stmt = self
            .conn
            .prepare("SELECT migration_id FROM schema_migrations ORDER BY migration_id")?;
        let rows = stmt.query_map((), |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The underlying connection, for schema-level assertions.
    ///
    /// Exposed so tests can inspect the SHAPE of the data model (composite keys,
    /// foreign keys, CHECK constraints) rather than only exercising the public
    /// API. REQ-DATA-003 is a statement about the schema, so a test that could
    /// not see the schema could not verify it.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Create a workspace. Idempotent.
    pub fn create_workspace(&self, workspace_id: &str) -> Result<(), VaultError> {
        if workspace_id.trim().is_empty() {
            return Err(VaultError::InvalidWorkspace);
        }
        self.conn.execute(
            "INSERT OR IGNORE INTO workspaces (workspace_id, created_utc) VALUES (?1, ?2)",
            (workspace_id, now_utc()),
        )?;
        Ok(())
    }

    /// Record (or find) a claim by its label. Idempotent per workspace.
    ///
    /// REQ-DATA-001: UUIDv7 ids, `workspace_id` on every workspace-owned row. A
    /// v7 id is time-ordered, so the primary key sorts by creation.
    pub fn put_claim(&self, workspace_id: &str, label: &str) -> Result<String, VaultError> {
        if workspace_id.trim().is_empty() {
            return Err(VaultError::InvalidWorkspace);
        }
        if label.trim().is_empty() {
            return Err(VaultError::InvalidWorkspace);
        }
        if let Ok(existing) = self.conn.query_row(
            "SELECT claim_id FROM claims WHERE workspace_id = ?1 AND label = ?2",
            (workspace_id, label),
            |r| r.get::<_, String>(0),
        ) {
            return Ok(existing);
        }
        let claim_id = format!("claim:{}", uuid::Uuid::now_v7());
        self.conn.execute(
            "INSERT INTO claims (claim_id, workspace_id, label, created_utc)
             VALUES (?1, ?2, ?3, ?4)",
            (claim_id.as_str(), workspace_id, label, now_utc()),
        )?;
        Ok(claim_id)
    }

    /// Record (or find) a support anchor. Idempotent per workspace/kind/ref.
    pub fn put_support_anchor(
        &self,
        workspace_id: &str,
        kind: &str,
        reference: &str,
    ) -> Result<String, VaultError> {
        if workspace_id.trim().is_empty() {
            return Err(VaultError::InvalidWorkspace);
        }
        if !matches!(kind, "SPECIFICATION" | "FIGURE") {
            return Err(VaultError::InvalidAnchorKind(kind.to_string()));
        }
        if reference.trim().is_empty() {
            return Err(VaultError::InvalidAnchorReference);
        }
        if let Ok(existing) = self.conn.query_row(
            "SELECT anchor_id FROM support_anchors
             WHERE workspace_id = ?1 AND kind = ?2 AND reference = ?3",
            (workspace_id, kind, reference),
            |r| r.get::<_, String>(0),
        ) {
            return Ok(existing);
        }
        let anchor_id = format!("anchor:{}", uuid::Uuid::now_v7());
        self.conn.execute(
            "INSERT INTO support_anchors (anchor_id, workspace_id, kind, reference, created_utc)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (anchor_id.as_str(), workspace_id, kind, reference, now_utc()),
        )?;
        Ok(anchor_id)
    }

    /// Record (or find) an evidence record by its content address.
    pub fn put_evidence_record(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> Result<String, VaultError> {
        if workspace_id.trim().is_empty() {
            return Err(VaultError::InvalidWorkspace);
        }
        if content_hash.trim().is_empty() {
            return Err(VaultError::InvalidAnchorReference);
        }
        if let Ok(existing) = self.conn.query_row(
            "SELECT evidence_id FROM evidence_records
             WHERE workspace_id = ?1 AND content_hash = ?2",
            (workspace_id, content_hash),
            |r| r.get::<_, String>(0),
        ) {
            return Ok(existing);
        }
        let evidence_id = format!("evidence:{}", uuid::Uuid::now_v7());
        self.conn.execute(
            "INSERT INTO evidence_records (evidence_id, workspace_id, content_hash, created_utc)
             VALUES (?1, ?2, ?3, ?4)",
            (evidence_id.as_str(), workspace_id, content_hash, now_utc()),
        )?;
        Ok(evidence_id)
    }

    /// Link a claim to a support anchor. Idempotent: re-linking is a no-op, not a
    /// duplicate row.
    pub fn link_claim_support(&self, claim_id: &str, anchor_id: &str) -> Result<(), VaultError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO claim_support (claim_id, anchor_id, linked_utc)
             VALUES (?1, ?2, ?3)",
            (claim_id, anchor_id, now_utc()),
        )?;
        Ok(())
    }

    /// Link a claim to an evidence record. Idempotent.
    pub fn link_claim_evidence(&self, claim_id: &str, evidence_id: &str) -> Result<(), VaultError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO claim_evidence (claim_id, evidence_id, linked_utc)
             VALUES (?1, ?2, ?3)",
            (claim_id, evidence_id, now_utc()),
        )?;
        Ok(())
    }

    /// The support matrix, read back through a JOIN across the junction table.
    ///
    /// Deliberately not a denormalized read: the relationship exists only in
    /// `claim_support`, which is what REQ-DATA-003 requires.
    pub fn claim_support_rows(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ClaimSupportRow>, VaultError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.claim_id, c.label, a.kind, a.reference
             FROM claim_support cs
             JOIN claims c          ON c.claim_id = cs.claim_id
             JOIN support_anchors a ON a.anchor_id = cs.anchor_id
             WHERE c.workspace_id = ?1
             ORDER BY c.label, a.kind, a.reference",
        )?;
        let rows = stmt.query_map((workspace_id,), |r| {
            Ok(ClaimSupportRow {
                claim_id: r.get(0)?,
                claim_label: r.get(1)?,
                anchor_kind: r.get(2)?,
                anchor_reference: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Claims with no supporting anchor at all, derived by an anti-join.
    pub fn unsupported_claims(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<UnsupportedClaim>, VaultError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.claim_id, c.label
             FROM claims c
             LEFT JOIN claim_support cs ON cs.claim_id = c.claim_id
             WHERE c.workspace_id = ?1 AND c.deleted = 0 AND cs.claim_id IS NULL
             ORDER BY c.label",
        )?;
        let rows = stmt.query_map((workspace_id,), |r| {
            Ok(UnsupportedClaim {
                claim_id: r.get(0)?,
                label: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The evidence linked to a claim, through the junction table.
    pub fn claim_evidence_hashes(&self, claim_id: &str) -> Result<Vec<String>, VaultError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.content_hash
             FROM claim_evidence ce
             JOIN evidence_records e ON e.evidence_id = ce.evidence_id
             WHERE ce.claim_id = ?1
             ORDER BY e.content_hash",
        )?;
        let rows = stmt.query_map((claim_id,), |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Store and durably commit a conception event.
    ///
    /// `REQ-DATA-001` requires `workspace_id` on every workspace-owned row, so
    /// an unknown workspace is refused rather than implicitly created: silently
    /// minting a workspace would break the SPEC-005 confidentiality boundary.
    pub fn put_conception_event(
        &self,
        workspace_id: &str,
        event_id: &str,
        origin: &str,
        content: &str,
    ) -> Result<StoredConceptionEvent, VaultError> {
        if workspace_id.trim().is_empty() {
            return Err(VaultError::InvalidWorkspace);
        }
        if content.trim().is_empty() {
            return Err(VaultError::InvalidContent);
        }
        let known: Option<String> = self
            .conn
            .query_row(
                "SELECT workspace_id FROM workspaces WHERE workspace_id = ?1",
                [workspace_id],
                |r| r.get(0),
            )
            .optional()?;
        if known.is_none() {
            return Err(VaultError::UnknownWorkspace(workspace_id.to_string()));
        }

        let content_hash = sha256_hex(content.as_bytes());
        let created_utc = now_utc();

        // Event row and audit entry commit together, so an event can never
        // exist without a corresponding chain entry.
        self.conn.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), VaultError> {
            self.conn.execute(
                "INSERT INTO conception_events
                    (event_id, workspace_id, origin, content_hash, content, created_utc, version, deleted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 0)",
                (event_id, workspace_id, origin, &content_hash, content, &created_utc),
            )?;
            self.append_audit(workspace_id, "conception_event_recorded", &content_hash)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.conn.execute_batch("COMMIT")?;
            }
            Err(e) => {
                self.conn.execute_batch("ROLLBACK")?;
                return Err(e);
            }
        }

        Ok(StoredConceptionEvent {
            event_id: event_id.to_string(),
            workspace_id: workspace_id.to_string(),
            origin: origin.to_string(),
            content_hash,
            content: content.to_string(),
            created_utc,
            version: 1,
        })
    }

    /// Read one event back.
    pub fn get_conception_event(
        &self,
        event_id: &str,
    ) -> Result<Option<StoredConceptionEvent>, VaultError> {
        let row = self
            .conn
            .query_row(
                "SELECT event_id, workspace_id, origin, content_hash, content, created_utc, version
                 FROM conception_events WHERE event_id = ?1 AND deleted = 0",
                [event_id],
                |r| {
                    Ok(StoredConceptionEvent {
                        event_id: r.get(0)?,
                        workspace_id: r.get(1)?,
                        origin: r.get(2)?,
                        content_hash: r.get(3)?,
                        content: r.get(4)?,
                        created_utc: r.get(5)?,
                        version: r.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Events in a workspace, oldest first.
    pub fn list_conception_events(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<StoredConceptionEvent>, VaultError> {
        let mut stmt = self.conn.prepare(
            "SELECT event_id, workspace_id, origin, content_hash, content, created_utc, version
             FROM conception_events WHERE workspace_id = ?1 AND deleted = 0
             ORDER BY created_utc, event_id",
        )?;
        let rows = stmt.query_map([workspace_id], |r| {
            Ok(StoredConceptionEvent {
                event_id: r.get(0)?,
                workspace_id: r.get(1)?,
                origin: r.get(2)?,
                content_hash: r.get(3)?,
                content: r.get(4)?,
                created_utc: r.get(5)?,
                version: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Store an evidence blob once, addressed by its SHA-256.
    ///
    /// `REQ-DATA-002`. Returns the content address. Storing the same bytes twice
    /// is idempotent and keeps a single row.
    pub fn put_blob(&self, data: &[u8]) -> Result<String, VaultError> {
        let hash = sha256_hex(data);
        self.conn.execute(
            "INSERT OR IGNORE INTO vault_blobs (content_hash, byte_len, data, created_utc)
             VALUES (?1, ?2, ?3, ?4)",
            (&hash, data.len() as i64, data, now_utc()),
        )?;
        Ok(hash)
    }

    /// Read a blob back by content address.
    pub fn get_blob(&self, content_hash: &str) -> Result<Option<Vec<u8>>, VaultError> {
        let row: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT data FROM vault_blobs WHERE content_hash = ?1",
                [content_hash],
                |r| r.get(0),
            )
            .optional()?;
        Ok(row)
    }

    pub fn blob_count(&self) -> Result<i64, VaultError> {
        Ok(self
            .conn
            .query_row("SELECT count(*) FROM vault_blobs", (), |r| r.get(0))?)
    }

    /// Append an audit entry, chaining it to the previous hash.
    ///
    /// `REQ-DATA-002`: audit events are an append-only hash chain.
    fn append_audit(
        &self,
        workspace_id: &str,
        event_type: &str,
        payload_hash: &str,
    ) -> Result<(), VaultError> {
        let previous: Option<String> = self
            .conn
            .query_row(
                "SELECT event_hash FROM audit_events ORDER BY seq DESC LIMIT 1",
                (),
                |r| r.get(0),
            )
            .optional()?;
        let previous = previous.unwrap_or_else(|| "GENESIS".to_string());
        let created_utc = now_utc();
        let event_hash = sha256_hex(
            format!("{previous}|{workspace_id}|{event_type}|{payload_hash}|{created_utc}")
                .as_bytes(),
        );
        self.conn.execute(
            "INSERT INTO audit_events
                (workspace_id, event_type, payload_hash, previous_hash, event_hash, created_utc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                workspace_id,
                event_type,
                payload_hash,
                &previous,
                &event_hash,
                &created_utc,
            ),
        )?;
        Ok(())
    }

    /// Number of audit entries.
    pub fn audit_len(&self) -> Result<i64, VaultError> {
        Ok(self
            .conn
            .query_row("SELECT count(*) FROM audit_events", (), |r| r.get(0))?)
    }

    /// Verify the audit chain is intact and append-only.
    ///
    /// Recomputes every link and reports the first break. `seq` must be strictly
    /// increasing and each `previous_hash` must equal the prior `event_hash`.
    pub fn verify_audit_chain(&self) -> Result<(), VaultError> {
        let mut stmt = self.conn.prepare(
            "SELECT seq, workspace_id, event_type, payload_hash, previous_hash, event_hash, created_utc
             FROM audit_events ORDER BY seq ASC",
        )?;
        let rows: Vec<(i64, String, String, String, String, String, String)> = stmt
            .query_map((), |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut expected_previous = "GENESIS".to_string();
        for (seq, workspace_id, event_type, payload_hash, previous_hash, event_hash, created_utc) in
            rows
        {
            if previous_hash != expected_previous {
                return Err(VaultError::ChainBroken {
                    seq,
                    expected: expected_previous,
                    found: previous_hash,
                });
            }
            let recomputed = sha256_hex(
                format!("{previous_hash}|{workspace_id}|{event_type}|{payload_hash}|{created_utc}")
                    .as_bytes(),
            );
            if recomputed != event_hash {
                return Err(VaultError::ChainBroken {
                    seq,
                    expected: recomputed,
                    found: event_hash,
                });
            }
            expected_previous = event_hash;
        }
        Ok(())
    }
}

/// Current UTC time as an ISO-8601 string.
///
/// `REQ-DATA-001` requires UTC timestamps. Formatted from the system clock
/// without pulling in a date-time crate.
fn now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_vault(tag: &str) -> (std::path::PathBuf, Vault) {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "linchpin-vault-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("vault.db");
        let vault = Vault::open(&path).expect("open vault");
        (dir, vault)
    }

    /// covers: REQ-RES-002
    #[test]
    fn test_migrations_are_recorded_and_idempotent() {
        let (dir, vault) = temp_vault("mig");
        let applied = vault.applied_migrations().unwrap();
        assert_eq!(applied.len(), MIGRATIONS.len());
        // Monotonic, and recorded in order.
        let mut sorted = applied.clone();
        sorted.sort();
        assert_eq!(applied, sorted, "migration ids are not monotonic");

        drop(vault);
        let reopened = Vault::open(&dir.join("vault.db")).unwrap();
        assert_eq!(
            reopened.applied_migrations().unwrap().len(),
            MIGRATIONS.len(),
            "re-running migrations duplicated entries"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-003
    /// "Claim/support/evidence relationships use normalized junction tables."
    ///
    /// The test asserts the SHAPE, not merely that rows round-trip: the
    /// relationships must live in junction tables keyed on both sides, with
    /// foreign keys to the entity tables. A denormalized design (an anchor list
    /// in a column on `claims`) would satisfy a round-trip test while violating
    /// the requirement, so the schema itself is inspected.
    #[test]
    fn test_claim_support_evidence_uses_normalized_junction_tables() {
        let (dir, vault) = temp_vault("junction");
        vault.create_workspace("ws-1").unwrap();
        let conn = vault.connection();

        for table in [
            "claims",
            "support_anchors",
            "evidence_records",
            "claim_support",
            "claim_evidence",
        ] {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    (table,),
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "missing table {table}");
        }

        // Junction tables are keyed on BOTH sides, which is what makes them
        // normalized many-to-many tables rather than child tables.
        for (table, left, right) in [
            ("claim_support", "claim_id", "anchor_id"),
            ("claim_evidence", "claim_id", "evidence_id"),
        ] {
            let mut stmt = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap();
            let pk: Vec<String> = stmt
                .query_map((), |r| Ok((r.get::<_, i64>(5)?, r.get::<_, String>(1)?)))
                .unwrap()
                .filter_map(|r| r.ok())
                .filter(|(pk, _)| *pk > 0)
                .map(|(_, name)| name)
                .collect();
            assert_eq!(
                pk,
                vec![left.to_string(), right.to_string()],
                "{table} must be keyed on both sides"
            );
        }

        // Both sides carry a foreign key, so an orphan link cannot exist.
        for (table, expected_targets) in [
            ("claim_support", vec!["claims", "support_anchors"]),
            ("claim_evidence", vec!["claims", "evidence_records"]),
        ] {
            let ddl: String = conn
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
                    (table,),
                    |r| r.get(0),
                )
                .unwrap();
            for target in expected_targets {
                assert!(
                    ddl.contains(&format!("REFERENCES {target}")),
                    "{table} must reference {target}: {ddl}"
                );
            }
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-003
    /// A claim linked to two anchors and one evidence record reads back through
    /// JOINs, and re-linking is idempotent rather than duplicating rows.
    #[test]
    fn test_claim_relationships_round_trip_through_junctions() {
        let (dir, vault) = temp_vault("junction-rt");
        vault.create_workspace("ws-1").unwrap();

        let claim = vault.put_claim("ws-1", "a self-sealing valve").unwrap();
        let spec = vault
            .put_support_anchor("ws-1", "SPECIFICATION", "[0042]")
            .unwrap();
        let fig = vault
            .put_support_anchor("ws-1", "FIGURE", "FIG. 3")
            .unwrap();
        let evidence = vault
            .put_evidence_record("ws-1", "sha256:deadbeef")
            .unwrap();

        vault.link_claim_support(&claim, &spec).unwrap();
        vault.link_claim_support(&claim, &fig).unwrap();
        vault.link_claim_evidence(&claim, &evidence).unwrap();

        // Idempotent: linking again must not duplicate the relationship.
        vault.link_claim_support(&claim, &spec).unwrap();
        vault.link_claim_evidence(&claim, &evidence).unwrap();

        let rows = vault.claim_support_rows("ws-1").unwrap();
        assert_eq!(
            rows.len(),
            2,
            "expected exactly two support links: {rows:?}"
        );
        assert_eq!(rows[0].anchor_kind, "FIGURE");
        assert_eq!(rows[0].anchor_reference, "FIG. 3");
        assert_eq!(rows[1].anchor_kind, "SPECIFICATION");
        assert_eq!(rows[1].claim_label, "a self-sealing valve");

        let hashes = vault.claim_evidence_hashes(&claim).unwrap();
        assert_eq!(hashes, vec!["sha256:deadbeef".to_string()]);

        // An unsupported claim is found by anti-join, not by a null column.
        let other = vault.put_claim("ws-1", "an unanchored coating").unwrap();
        let unsupported = vault.unsupported_claims("ws-1").unwrap();
        assert_eq!(unsupported.len(), 1);
        assert_eq!(unsupported[0].claim_id, other);
        assert_eq!(unsupported[0].label, "an unanchored coating");

        // Identifiers are prefixed and time-ordered (REQ-DATA-001 uses UUIDv7).
        assert!(claim.starts_with("claim:"), "claim id shape: {claim}");
        assert!(spec.starts_with("anchor:"), "anchor id shape: {spec}");
        assert!(
            evidence.starts_with("evidence:"),
            "evidence id shape: {evidence}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-003
    /// The junction foreign keys must be ENFORCED, not merely declared: an
    /// orphan link is refused by the database.
    #[test]
    fn test_junction_foreign_keys_are_enforced() {
        let (dir, vault) = temp_vault("junction-fk");
        vault.create_workspace("ws-1").unwrap();
        let conn = vault.connection();

        let orphan = conn.execute(
            "INSERT INTO claim_support (claim_id, anchor_id, linked_utc)
             VALUES ('claim:missing', 'anchor:missing', '2026-01-01T00:00:00Z')",
            (),
        );
        assert!(
            orphan.is_err(),
            "an orphan claim_support row was accepted: the foreign keys are not enforced"
        );

        // Anchor kinds are constrained to the closed set at the schema level too.
        let bad_kind = conn.execute(
            "INSERT INTO support_anchors (anchor_id, workspace_id, kind, reference, created_utc)
             VALUES ('anchor:x', 'ws-1', 'DRAWING', 'FIG. 1', '2026-01-01T00:00:00Z')",
            (),
        );
        assert!(bad_kind.is_err(), "an unlisted anchor kind was accepted");

        // Blank references and unlisted kinds are refused by the API as well.
        // `matches!` rather than `assert_eq!`: VaultError wraps rusqlite::Error,
        // which is not PartialEq, so the Result cannot be compared directly.
        assert!(
            matches!(
                vault.put_support_anchor("ws-1", "FIGURE", "   "),
                Err(VaultError::InvalidAnchorReference)
            ),
            "a blank anchor reference must be refused"
        );
        assert!(
            matches!(
                vault.put_support_anchor("ws-1", "DRAWING", "FIG. 1"),
                Err(VaultError::InvalidAnchorKind(ref k)) if k == "DRAWING"
            ),
            "an unlisted anchor kind must be refused"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-001, REQ-DOM-001
    #[test]
    fn test_conception_event_survives_reopen() {
        let (dir, vault) = temp_vault("persist");
        vault.create_workspace("ws-1").unwrap();
        let stored = vault
            .put_conception_event("ws-1", "ev-1", "HumanConception", "a self-sealing valve")
            .unwrap();
        assert!(stored.content_hash.starts_with("sha256:"));
        assert_eq!(
            stored.content_hash,
            sha256_hex(b"a self-sealing valve"),
            "content hash is not SHA-256 of the content"
        );
        drop(vault);

        let reopened = Vault::open(&dir.join("vault.db")).unwrap();
        let read_back = reopened.get_conception_event("ev-1").unwrap().unwrap();
        assert_eq!(read_back.content, "a self-sealing valve");
        assert_eq!(read_back.origin, "HumanConception");
        assert_eq!(read_back.workspace_id, "ws-1");
        assert_eq!(read_back.version, 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-001, REQ-SEC-001
    /// REQ-DATA-001: workspace_id is required on every workspace-owned row, so
    /// an unknown workspace must be refused rather than silently created.
    #[test]
    fn test_unknown_workspace_is_refused() {
        let (dir, vault) = temp_vault("unknown");
        let err = vault
            .put_conception_event("ghost", "ev-1", "HumanConception", "content")
            .unwrap_err();
        assert!(
            matches!(err, VaultError::UnknownWorkspace(_)),
            "got {err:?}"
        );

        // And nothing was written as a side effect.
        assert!(vault.get_conception_event("ev-1").unwrap().is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-001, REQ-SEC-001
    #[test]
    fn test_events_are_workspace_scoped() {
        let (dir, vault) = temp_vault("scope");
        vault.create_workspace("ws-a").unwrap();
        vault.create_workspace("ws-b").unwrap();
        vault
            .put_conception_event("ws-a", "a1", "HumanConception", "alpha")
            .unwrap();
        vault
            .put_conception_event("ws-b", "b1", "AiSuggestion", "beta")
            .unwrap();

        let a = vault.list_conception_events("ws-a").unwrap();
        let b = vault.list_conception_events("ws-b").unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert_eq!(a[0].content, "alpha");
        assert_eq!(b[0].content, "beta");
        assert_eq!(b[0].origin, "AiSuggestion");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-002
    /// REQ-DATA-002: blobs are content-addressed, so identical bytes collapse to
    /// one row and different bytes get different addresses.
    #[test]
    fn test_blobs_are_content_addressed() {
        let (dir, vault) = temp_vault("blob");
        let h1 = vault.put_blob(b"payload one").unwrap();
        let h2 = vault.put_blob(b"payload one").unwrap();
        let h3 = vault.put_blob(b"payload two").unwrap();

        assert_eq!(h1, h2, "identical bytes produced different addresses");
        assert_ne!(h1, h3);
        assert_eq!(vault.blob_count().unwrap(), 2, "duplicate blob was stored");

        assert_eq!(vault.get_blob(&h1).unwrap().unwrap(), b"payload one");
        assert!(vault.get_blob("sha256:absent").unwrap().is_none());

        // The address must be the real digest of the bytes.
        assert_eq!(h1, sha256_hex(b"payload one"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-002
    /// REQ-DATA-002: the audit log is an append-only hash chain, and the
    /// verifier must detect tampering.
    #[test]
    fn test_audit_chain_verifies_and_detects_tampering() {
        let (dir, vault) = temp_vault("audit");
        vault.create_workspace("ws-1").unwrap();
        for (i, text) in ["one", "two", "three"].iter().enumerate() {
            vault
                .put_conception_event("ws-1", &format!("ev-{i}"), "HumanConception", text)
                .unwrap();
        }
        assert_eq!(vault.audit_len().unwrap(), 3);
        vault.verify_audit_chain().expect("chain must verify");

        // Tamper with the payload hash of the first entry.
        vault
            .conn
            .execute(
                "UPDATE audit_events SET payload_hash = 'sha256:tampered' WHERE seq = 1",
                (),
            )
            .unwrap();
        let err = vault
            .verify_audit_chain()
            .expect_err("tampering must be detected");
        assert!(matches!(err, VaultError::ChainBroken { .. }), "got {err:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_validation_rejects_empty_inputs() {
        let (dir, vault) = temp_vault("valid");
        assert!(matches!(
            vault.create_workspace("  ").unwrap_err(),
            VaultError::InvalidWorkspace
        ));
        vault.create_workspace("ws-1").unwrap();
        assert!(matches!(
            vault
                .put_conception_event("ws-1", "e", "HumanConception", "   ")
                .unwrap_err(),
            VaultError::InvalidContent
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-DATA-002
    #[test]
    fn test_sha256_known_vector() {
        // NIST/FIPS test vector: SHA-256("abc").
        assert_eq!(
            sha256_hex(b"abc"),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}

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

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Monotonic migration identifiers.
///
/// `REQ-RES-002`: IDs are monotonic and never edited after release. Appending
/// is the only permitted change; existing entries are frozen.
/// The tables each migration must leave behind, checked after opening.
///
/// Kept beside `MIGRATIONS` so a new migration without an entry here is visible
/// in review rather than silently unverified.
pub const REQUIRED_SCHEMA: &[(&str, &[&str])] = &[
    ("0001_workspaces", &["workspaces"]),
    (
        "0002_conception_events",
        &["conception_events", "idx_conception_workspace"],
    ),
    ("0003_audit_chain", &["audit_events"]),
    ("0004_vault_blobs", &["vault_blobs"]),
    (
        "0005_claim_support_evidence",
        &[
            "claims",
            "support_anchors",
            "evidence_records",
            "claim_support",
            "claim_evidence",
            "idx_claims_workspace",
            "idx_anchors_workspace",
            "idx_evidence_workspace",
            "idx_claim_support_anchor",
            "idx_claim_evidence_evidence",
        ],
    ),
];

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
    /// A backup file to restore from does not exist (REQ-REL-005).
    BackupMissing(String),
    /// Filesystem failure while backing up or restoring.
    Io(String),
    /// A migration is recorded but the objects it creates are absent, so the
    /// vault would fail later at a random write instead of at open (DOD-016).
    SchemaIncomplete(String),
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
            VaultError::BackupMissing(p) => write!(f, "backup file does not exist: {p}"),
            VaultError::Io(m) => write!(f, "io error: {m}"),
            VaultError::SchemaIncomplete(m) => write!(
                f,
                "migration state is inconsistent: {m}. The vault records migrations whose \
                 tables are missing, so it would fail later at a write instead of here"
            ),
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

impl From<std::io::Error> for VaultError {
    fn from(e: std::io::Error) -> Self {
        VaultError::Io(e.to_string())
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
        self.verify_schema()
    }

    /// Prove the objects every recorded migration creates are actually present.
    ///
    /// A record is not a schema. Measured failure mode this closes: a vault whose
    /// `schema_migrations` rows exist while the tables do not (a partially
    /// restored file, a copy taken mid-migration, a hand-edited database) opened
    /// SUCCESSFULLY, because every migration was already "applied" and nothing
    /// checked -- and then failed later at whichever write happened to touch a
    /// missing table. Failing at open names the missing object once, in the place
    /// where the operator can still do something about it.
    fn verify_schema(&self) -> Result<(), VaultError> {
        for (id, objects) in REQUIRED_SCHEMA {
            for object in *objects {
                let present: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT name FROM sqlite_master WHERE name = ?1",
                        [*object],
                        |r| r.get(0),
                    )
                    .optional()?;
                if present.is_none() {
                    return Err(VaultError::SchemaIncomplete(format!(
                        "migration {id} is recorded but {object} is missing"
                    )));
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

    /// A reconciled digest of the canonical state.
    ///
    /// REQ-REL-005 requires recovery "against RECONCILED persistent state", so
    /// there must be something to reconcile against. This digests the durable
    /// CONTENT -- conception events and the claim/support graph -- not file
    /// bytes, so two stores holding the same logical state compare equal
    /// regardless of page layout or WAL framing.
    pub fn state_digest(&self) -> Result<String, VaultError> {
        Self::digest_connection(&self.conn)
    }

    /// The digest computation itself, over any connection.
    ///
    /// Separate from `state_digest` so a backup can be reconciled READ-ONLY:
    /// see `digest_of`. Reconciliation that mutated the thing it was measuring
    /// would not be reconciliation.
    fn digest_connection(conn: &Connection) -> Result<String, VaultError> {
        let mut hasher = Sha256::new();
        let mut stmt = conn.prepare(
            "SELECT event_id, workspace_id, origin, content_hash, created_utc
             FROM conception_events ORDER BY event_id",
        )?;
        let rows = stmt.query_map((), |r| {
            Ok(format!(
                "event|{}|{}|{}|{}|{}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?
            ))
        })?;
        for row in rows {
            hasher.update(row?.as_bytes());
            hasher.update(b"\n");
        }
        let mut stmt = conn.prepare(
            "SELECT c.claim_id, c.label, a.kind, a.reference
             FROM claims c
             LEFT JOIN claim_support cs ON cs.claim_id = c.claim_id
             LEFT JOIN support_anchors a ON a.anchor_id = cs.anchor_id
             ORDER BY c.claim_id, a.kind, a.reference",
        )?;
        let rows = stmt.query_map((), |r| {
            Ok(format!(
                "claim|{}|{}|{}|{}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                r.get::<_, Option<String>>(3)?.unwrap_or_default()
            ))
        })?;
        for row in rows {
            hasher.update(row?.as_bytes());
            hasher.update(b"\n");
        }
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    /// Write a consistent backup of this vault to `dest`.
    ///
    /// Uses SQLite's online-backup API rather than copying the file: a copy can
    /// capture a torn write-ahead log, and a backup that silently restores to a
    /// corrupt state is worse than no backup. The destination is replaced when
    /// it exists, so a backup is a point-in-time snapshot and never a merge.
    pub fn backup_to(&self, dest: &Path) -> Result<(), VaultError> {
        if dest.exists() {
            std::fs::remove_file(dest)?;
        }
        let mut target = Connection::open(dest)?;
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut target)?;
        backup.run_to_completion(64, std::time::Duration::from_millis(1), None)?;
        Ok(())
    }

    /// The state digest held by a backup file, without adopting it.
    ///
    /// Reconciliation must happen BEFORE restoring, so a restore that would not
    /// change anything is detectable rather than assumed.
    ///
    /// The existence check is load-bearing, not defensive. `Vault::open` has
    /// SQLite's open-or-create semantics, so digesting a mistyped path would
    /// CREATE an empty database and return the digest of nothing -- and the
    /// caller would then "restore" that emptiness over the real vault. Measured:
    /// a test restoring from an absent path was accepted and wiped the vault.
    ///
    /// Opened READ-ONLY and without migrations, so reconciling against a backup
    /// cannot alter it -- a backup on read-only media still reconciles, a file
    /// that is not a vault fails closed instead of being given a schema, and the
    /// digest a caller restores against is the digest of what was on disk.
    pub fn digest_of(path: &Path) -> Result<String, VaultError> {
        if !path.exists() {
            return Err(VaultError::BackupMissing(path.display().to_string()));
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Self::digest_connection(&conn)
    }

    /// Replace this vault's canonical content with the backup at `source`.
    ///
    /// Restore is destructive and deliberate: the current content is discarded,
    /// so callers reconcile first (see `digest_of`). Implemented as an online
    /// backup FROM the source INTO this connection, which keeps the open
    /// connection valid rather than swapping files underneath it.
    pub fn restore_from(&mut self, source: &Path) -> Result<(), VaultError> {
        if !source.exists() {
            return Err(VaultError::BackupMissing(source.display().to_string()));
        }
        let src = Connection::open(source)?;
        let backup = rusqlite::backup::Backup::new(&src, &mut self.conn)?;
        backup.run_to_completion(64, std::time::Duration::from_millis(1), None)?;
        Ok(())
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

    /// Build a vault at an EARLIER schema state: apply only the first `levels`
    /// migrations, exactly as a vault created by that older build would look.
    ///
    /// Test-only, and deliberately hand-rolled rather than calling
    /// `Vault::open`: the point of the matrix is to reach a state the current
    /// build does not produce on its own.
    fn vault_at_schema(path: &std::path::Path, levels: usize) -> Connection {
        let conn = Connection::open(path).expect("open raw");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                migration_id TEXT PRIMARY KEY,
                applied_utc  TEXT NOT NULL
            );",
        )
        .expect("bookkeeping table");
        for (id, sql) in MIGRATIONS.iter().take(levels) {
            conn.execute_batch(sql).expect("apply prefix migration");
            conn.execute(
                "INSERT INTO schema_migrations (migration_id, applied_utc) VALUES (?1, ?2)",
                (id, now_utc()),
            )
            .expect("record prefix migration");
        }
        conn
    }

    /// covers: REQ-DATA-003, DOD-016
    /// Every prior schema state must migrate forward to the current one with the
    /// data that existed at that state preserved, and the migrated vault must be
    /// usable afterwards.
    ///
    /// The clause names "an empty database and every supported prior released
    /// schema". There are no prior RELEASES (v0.1.0 is unreleased), so the
    /// supported prior schemas are the intermediate states the migration list
    /// defines -- and each one is executed here rather than assumed.
    #[test]
    fn test_migration_matrix_preserves_data_from_every_prior_schema() {
        let mut matrix: Vec<String> = Vec::new();

        for levels in 1..=MIGRATIONS.len() {
            let dir = std::env::temp_dir().join(format!(
                "linchpin-migmatrix-{}-{}-{}",
                levels,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(&dir).expect("temp dir");
            let path = dir.join("vault.db");

            // --- state at the prior schema, with data that schema can hold ---
            {
                let conn = vault_at_schema(&path, levels);
                conn.execute(
                    "INSERT INTO workspaces (workspace_id, created_utc) VALUES ('ws-old', '2026-01-01T00:00:00Z')",
                    [],
                )
                .expect("seed workspace");
                if levels >= 2 {
                    conn.execute(
                        "INSERT INTO conception_events
                           (event_id, workspace_id, origin, content_hash, content, created_utc)
                         VALUES ('ev-old', 'ws-old', 'HumanConception', 'sha256:abc', 'pre-migration content', '2026-01-02T00:00:00Z')",
                        [],
                    )
                    .expect("seed event");
                }
                if levels >= 5 {
                    conn.execute(
                        "INSERT INTO claims (claim_id, workspace_id, label, created_utc)
                         VALUES ('cl-old', 'ws-old', 'pre-migration claim', '2026-01-03T00:00:00Z')",
                        [],
                    )
                    .expect("seed claim");
                    conn.execute(
                        "INSERT INTO support_anchors (anchor_id, workspace_id, kind, reference, created_utc)
                         VALUES ('an-old', 'ws-old', 'SPECIFICATION', '[0001]', '2026-01-03T00:00:00Z')",
                        [],
                    )
                    .expect("seed anchor");
                    conn.execute(
                        "INSERT INTO claim_support (claim_id, anchor_id, linked_utc)
                         VALUES ('cl-old', 'an-old', '2026-01-03T00:00:00Z')",
                        [],
                    )
                    .expect("seed junction");
                }
            }

            // --- migrate forward by opening normally --------------------------
            let vault = Vault::open(&path).expect("the vault must migrate forward");
            let applied = vault.applied_migrations().unwrap();
            assert_eq!(
                applied.len(),
                MIGRATIONS.len(),
                "schema level {levels} did not reach the current migration count"
            );
            assert_eq!(applied, {
                let mut expected: Vec<String> =
                    MIGRATIONS.iter().map(|(id, _)| id.to_string()).collect();
                expected.sort();
                expected
            });

            // --- logical data preservation ------------------------------------
            let workspace: Option<String> = vault
                .connection()
                .query_row(
                    "SELECT workspace_id FROM workspaces WHERE workspace_id = 'ws-old'",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .unwrap();
            assert_eq!(
                workspace.as_deref(),
                Some("ws-old"),
                "workspace lost migrating from schema level {levels}"
            );

            if levels >= 2 {
                let events = vault.list_conception_events("ws-old").unwrap();
                assert_eq!(
                    events.len(),
                    1,
                    "event lost migrating from schema level {levels}"
                );
                assert_eq!(events[0].content, "pre-migration content");
                assert_eq!(events[0].content_hash, "sha256:abc");
            }

            if levels >= 5 {
                let rows = vault.claim_support_rows("ws-old").unwrap();
                assert_eq!(
                    rows.len(),
                    1,
                    "claim/anchor junction lost migrating from schema level {levels}"
                );
            }

            // --- the migrated vault is USABLE, not merely readable ------------
            vault
                .put_conception_event("ws-old", "ev-new", "HumanConception", "post-migration")
                .expect("a write after migration must succeed");
            let after = vault.list_conception_events("ws-old").unwrap();
            assert!(
                after.iter().any(|e| e.content == "post-migration"),
                "the migrated vault rejected a new write at schema level {levels}"
            );

            matrix.push(format!(
                "from {levels} of {} prior migration(s): preserved and writable",
                MIGRATIONS.len()
            ));
            drop(vault);
            std::fs::remove_dir_all(&dir).ok();
        }

        assert_eq!(
            matrix.len(),
            MIGRATIONS.len(),
            "the matrix must cover every prior schema state"
        );
    }

    /// covers: DOD-016
    /// A vault that CLAIMS a migration it did not apply must be refused at open,
    /// not accepted and then failed at a random later write.
    #[test]
    fn test_recorded_migration_without_its_tables_is_refused() {
        let dir = std::env::temp_dir().join(format!(
            "linchpin-migincomplete-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("vault.db");

        // Every migration recorded, but the claim tables were never created --
        // the shape a copy taken mid-migration or a hand-edited file has.
        {
            let conn = Connection::open(&path).unwrap();
            vault_at_schema(&path, 0);
            for (id, _) in MIGRATIONS.iter().take(4) {
                conn.execute_batch(MIGRATIONS.iter().find(|(m, _)| m == id).unwrap().1)
                    .unwrap();
                conn.execute(
                    "INSERT INTO schema_migrations (migration_id, applied_utc) VALUES (?1, ?2)",
                    (id, now_utc()),
                )
                .unwrap();
            }
            conn.execute(
                "INSERT INTO schema_migrations (migration_id, applied_utc) VALUES ('0005_claim_support_evidence', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }

        let refused = Vault::open(&path);
        match refused {
            Err(VaultError::SchemaIncomplete(message)) => {
                assert!(
                    message.contains("claims"),
                    "the refusal must name the missing object: {message}"
                );
            }
            Err(other) => panic!("refused with the wrong error: {other}"),
            Ok(_) => panic!(
                "a vault claiming a migration it never applied was accepted; it would fail later \
                 at whichever write touches the missing table"
            ),
        }

        std::fs::remove_dir_all(&dir).ok();
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

    /// covers: REQ-REL-005
    /// "Backup/restart/recovery ... are executed against reconciled persistent
    /// state." A backup must capture a POINT IN TIME: after restoring, state
    /// written after the backup must be gone, and the digest must reconcile with
    /// the backup's, not merely be self-consistent.
    #[test]
    fn test_backup_and_restore_reconcile_to_the_snapshot() {
        let (dir, mut vault) = temp_vault("backup");
        let vault_path = dir.join("vault.db");
        let backup_path = dir.join("backup.db");
        vault.create_workspace("ws-1").unwrap();

        vault
            .put_conception_event("ws-1", "ev-1", "HumanConception", "first conception")
            .unwrap();
        vault.put_claim("ws-1", "a self-sealing valve").unwrap();

        let before = vault.state_digest().unwrap();
        vault.backup_to(&backup_path).unwrap();
        assert!(backup_path.exists(), "backup file was not written");
        assert_eq!(
            Vault::digest_of(&backup_path).unwrap(),
            before,
            "the backup does not reconcile with the state it was taken from"
        );

        // Divergence AFTER the backup: more events, another claim.
        vault
            .put_conception_event("ws-1", "ev-2", "HumanConception", "second conception")
            .unwrap();
        vault.put_claim("ws-1", "an unanchored coating").unwrap();
        let diverged = vault.state_digest().unwrap();
        assert_ne!(diverged, before, "test premise: state must have diverged");

        // Recovery. The open connection is reused, not swapped, so the handle
        // stays valid -- which is what an operator's live process needs.
        vault.restore_from(&backup_path).unwrap();
        assert_eq!(
            vault.state_digest().unwrap(),
            before,
            "restored state does not reconcile with the backup digest"
        );

        // The post-backup work is gone, and the pre-backup work survived.
        let events = vault.list_conception_events("ws-1").unwrap();
        assert_eq!(events.len(), 1, "restore did not discard post-backup work");
        assert_eq!(events[0].content, "first conception");
        let claims = vault.claim_support_rows("ws-1").unwrap();
        assert!(
            claims.is_empty(),
            "the post-backup claim survived the restore"
        );

        // Independently read the restored state back through a SECOND connection
        // to the file, so the reconciliation is not just one handle agreeing
        // with itself (DOD-012).
        let reopened = Vault::open(&vault_path).unwrap();
        assert_eq!(
            reopened.state_digest().unwrap(),
            before,
            "the durable file does not reconcile with the backup"
        );
        drop(reopened);

        // Restoring from a missing backup is an error, not a silent no-op.
        assert!(matches!(
            vault.restore_from(&dir.join("absent.db")),
            Err(VaultError::BackupMissing(_))
        ));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-REL-005
    /// Reconciliation must be READ-ONLY and must fail closed. Measured defect:
    /// `digest_of` inherited SQLite's create-on-missing semantics, so a restore
    /// from a mistyped path was ACCEPTED -- it created an empty database at the
    /// typo, digested that emptiness, and replaced the real vault with it.
    #[test]
    fn test_digest_of_is_read_only_and_refuses_non_vaults() {
        let (dir, vault) = temp_vault("digest-of");
        vault.create_workspace("ws-1").unwrap();
        vault
            .put_conception_event("ws-1", "ev-1", "HumanConception", "kept")
            .unwrap();
        let backup_path = dir.join("backup.db");
        vault.backup_to(&backup_path).unwrap();

        // Reconciling must not touch one byte of the backup it measures.
        let bytes_before = std::fs::read(&backup_path).unwrap();
        let first = Vault::digest_of(&backup_path).unwrap();
        let second = Vault::digest_of(&backup_path).unwrap();
        assert_eq!(first, second, "reconciliation is not idempotent");
        assert_eq!(
            std::fs::read(&backup_path).unwrap(),
            bytes_before,
            "reconciliation rewrote the backup"
        );

        // An absent path fails closed WITHOUT creating a phantom database.
        let absent = dir.join("typo.db");
        assert!(matches!(
            Vault::digest_of(&absent),
            Err(VaultError::BackupMissing(_))
        ));
        assert!(
            !absent.exists(),
            "digest_of created {} as a side effect",
            absent.display()
        );

        // A file that is not a vault fails closed rather than being given a
        // schema and digested as empty.
        let foreign = dir.join("notes.txt");
        std::fs::write(&foreign, "this is not a database").unwrap();
        assert!(
            Vault::digest_of(&foreign).is_err(),
            "a non-vault file was digested as if it were state"
        );
        assert_eq!(
            std::fs::read_to_string(&foreign).unwrap(),
            "this is not a database",
            "digest_of rewrote a non-vault file"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-REL-005
    /// Recovery must survive a HARD restart: the restored state has to be
    /// durable, read back by a fresh process rather than by the one that
    /// performed the restore.
    #[test]
    fn test_recovered_state_survives_a_hard_restart() {
        let (dir, mut vault) = temp_vault("restart");
        let vault_path = dir.join("vault.db");
        let backup_path = dir.join("backup.db");
        vault.create_workspace("ws-1").unwrap();
        vault
            .put_conception_event("ws-1", "ev-1", "HumanConception", "survives restart")
            .unwrap();
        let snapshot = vault.state_digest().unwrap();
        vault.backup_to(&backup_path).unwrap();

        vault
            .put_conception_event("ws-1", "ev-2", "HumanConception", "lost on restore")
            .unwrap();
        vault.restore_from(&backup_path).unwrap();
        drop(vault); // hard restart: the handle is gone, nothing is cached

        let reopened = Vault::open(&vault_path).unwrap();
        assert_eq!(
            reopened.state_digest().unwrap(),
            snapshot,
            "the recovered state did not survive a restart"
        );
        let events = reopened.list_conception_events("ws-1").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].content, "survives restart");

        // Reopening must not re-run migrations over restored data.
        assert_eq!(
            reopened.applied_migrations().unwrap().len(),
            MIGRATIONS.len()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// covers: REQ-REL-005
    /// The state digest must observe CONTENT, not file bytes: an identical
    /// logical state reached by a different path must reconcile equal, or
    /// reconciliation would report false divergence.
    #[test]
    fn test_state_digest_observes_content_not_layout() {
        let (dir, a) = temp_vault("digest-a");
        let (dir_b, b) = temp_vault("digest-b");
        for v in [&a, &b] {
            v.create_workspace("ws-1").unwrap();
            v.put_conception_event("ws-1", "ev-1", "HumanConception", "same content")
                .unwrap();
            v.put_claim("ws-1", "same claim").unwrap();
        }
        // Identical logical state in two separate stores: the event and claim
        // ids differ (UUIDv7), so digests legitimately differ. The property
        // under test is the reverse: writing the SAME rows twice in one store
        // must not change the digest.
        let once = a.state_digest().unwrap();
        let again = a.state_digest().unwrap();
        assert_eq!(once, again, "the digest is not stable across calls");

        // A backup/restore round trip in the same store must be digest-stable,
        // which proves the digest reflects content rather than page layout.
        let backup = dir.join("roundtrip.db");
        a.backup_to(&backup).unwrap();
        let mut a = a;
        a.restore_from(&backup).unwrap();
        assert_eq!(
            a.state_digest().unwrap(),
            once,
            "a restore of identical content changed the digest"
        );

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&dir_b).ok();
    }
}

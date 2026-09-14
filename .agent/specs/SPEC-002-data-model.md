# SPEC-002 Data Model

REQ-DATA-001 Canonical SQLite tables mirror ARCHITECTURE entities with UUIDv7 IDs, workspace_id on every workspace-owned row, created/updated UTC timestamps, optimistic version, deletion tombstone where needed.

REQ-DATA-002 Evidence blobs are content-addressed SHA-256; audit events are append-only hash-chain.

REQ-DATA-003 Claim/support/evidence relationships use normalized junction tables.

REQ-RES-001 DuckDB/Parquet/Tantivy/vector data is derived and rebuildable from the canonical SQLite store; no derived index is a source of truth.

REQ-RES-002 Migration IDs are monotonic and never edited after release.

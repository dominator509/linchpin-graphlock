# DOD-015 / DOD-016 — restart persistence and the migration matrix

Two clauses that were PARTIAL on stale notes, re-measured this round. Both now
have executed evidence rather than a corrected narrative.

## DOD-015 — persistent state survives a full PROCESS restart

DOD-015 RULE: "Persistent state survives full process and container restart and
is readable by the supported runtime."
REQUIRED EVIDENCE: "Pre-restart state hash, hard restart evidence, post-restart
independent read, and reconciliation."

The earlier disposition was honest about its own gap: a **connection-level
reopen inside one process** is not a full process restart. The exact-artifact
lane — the only lane driving the packaged executable — now performs the real
thing, and the read-back goes through the product rather than the filesystem:

| Requirement | Measurement |
| --- | --- |
| Pre-restart state | the ledger is read through `list_conception_events` BEFORE the kill (2 events, including the run's canary) |
| Hard restart | the executable is killed (`SIGKILL` fallback) and the harness **waits until the debug endpoint stops answering**, so the relaunch cannot be mistaken for the old process still running |
| Relaunch | the packaged artifact is started again and a page target is acquired |
| Post-restart independent read | the ledger is read again through the SAME product command in the NEW process |
| Reconciliation | the count is unchanged (2 → 2) and the runtime canary from before the restart is present in the ledger content |

Measured assertions in `.agent/evidence/artifact-e2e/STATUS.md`:

```
PASS: hard restart: the previous process is gone (DOD-015) -- debug endpoint stopped answering
PASS: hard restart: the artifact relaunches (DOD-015) -- page target available after restart
PASS: hard restart: the ledger is readable by the runtime after relaunch (DOD-015) -- pre-restart 2 event(s), post-restart 2
PASS: hard restart: the pre-restart canary is still present (DOD-015, DOD-013) -- ARTIFACT-CANARY-… found in the ledger read back through the runtime
```

### The defect this closed: the ledger was WRITE-ONLY

`record_conception` durably stored events and **no product path ever read them
back**. The "Human Conception Ledger" promised by UO-01 could be written and not
consulted — a ledger in name only. `commands::list_conception_events` now reads
it back with origins counted separately (`human_count` / `ai_count`), the
Conception Lab surface renders it, and the restart proof above depends on it.

Test coverage: `test_conception_ledger_reads_durable_state_and_separates_origins`
(product boundary: empty/missing vault refused, three events read back, origins
never merged, workspace isolation, and the empty case reported as empty rather
than as an error), the browser-lane assertion that the control exists and reports
the disconnected state honestly, and `MUT-DATA-001-a`, which makes the read
return an empty list and is CAUGHT.

**Limits, stated:** the restart proof covers the application process on one
machine; it is not a container restart and not a power-loss test. The ledger read
returns **all** non-deleted events for the workspace, so a very large ledger
produces an unbounded response and the UI does not paginate.

## DOD-016 — migrations from an empty database and every SUPPORTED PRIOR schema

DOD-016 RULE: "Required migrations work from an empty database and every
supported prior released schema with logical data preservation."
REQUIRED EVIDENCE: "Baseline schema/data hashes, migration logs, post-migration
invariants, retry/rollback evidence, and supported-version matrix."

There are no prior **releases** (v0.1.0 is unreleased), so the supported prior
schemas are the intermediate states the migration list defines. Each one is now
EXECUTED rather than assumed, by
`test_migration_matrix_preserves_data_from_every_prior_schema`:

| From schema state | Data present at that state | Post-migration assertion |
| --- | --- | --- |
| `0001` only | workspace | workspace preserved; all five migrations recorded; a new write succeeds |
| `0001..0002` | + conception event | event content and `content_hash` preserved; new write succeeds |
| `0001..0003` | + same | preserved; new write succeeds |
| `0001..0004` | + same | preserved; new write succeeds |
| `0001..0005` | + claim, anchor, junction | junction preserved; new write succeeds |

Every state also asserts the migrated vault is **usable**, not merely readable —
a migration that leaves a schema which rejects writes is a failure the matrix
would otherwise miss.

### The defect this closed: a record is not a schema

A database whose `schema_migrations` rows exist while the tables do not (a copy
taken mid-migration, a partially restored file, a hand-edited database) opened
**successfully**, because every migration was already recorded as applied and
nothing verified the objects. It then failed at whichever later write touched a
missing table — the furthest possible point from the cause.

`Vault::open` now verifies, after migrating, that every object every migration
creates is present (`REQUIRED_SCHEMA`), and refuses with `SchemaIncomplete`
naming the migration and the missing object. That is the clause's
**retry/rollback evidence**: the failure is reported at open, where it can be
acted on, instead of surfacing as a random write error.

`MUT-DOD-016-a` removes that check and is CAUGHT by
`test_recorded_migration_without_its_tables_is_refused`.

**Limits, stated:** the matrix covers the migration states this repository
defines, not schemas from any released or third-party build; there is no
downgrade path, because the project has never shipped a schema it must migrate
backwards from; and `REQUIRED_SCHEMA` is maintained by hand beside `MIGRATIONS`,
so a future migration that forgets its entry is caught in review rather than by
the compiler.

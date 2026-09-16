# DOD-038 — abbreviated STRESS trial (concurrency at a defined workload)

`apps/desktop/src-tauri/tests/stress_concurrency.rs` — labeled **ABBREVIATED**,
because no stress concurrency level, duration or throughput target is specified
anywhere in the repository, so "their specified scale" has no value to complete.
DOD-038 remains PARTIAL and this trial does not claim it.

## The defined workload

| Element | Value |
| --- | --- |
| Writers | 8 threads, distinct clients keys, each write opening its own connection exactly as the IPC layer does per request |
| Readers | 4 threads reading the whole ledger back through the product's own read path |
| Backup | 1 thread taking online snapshots concurrently |
| Events | 250 per writer = 2 000 submissions |
| Bound | 300 s wall clock, encoded p95 ≤ 2 000 ms and ≥ 5 events/s floors |

**Measured** (`.agent/evidence/stress/report.json`): 2 000 submissions, **0
failures, 0 lost**, ledger holds 2 000 events; 4 readers and 1 backup thread with
**0 failures**; throughput **106 events/s**; write p50/p95/max recorded; peak
working set sampled by an external observer. The trial asserts zero lost events
(a submission that neither succeeded nor reported a failure), ledger == successful
writes, zero read/backup failures, no hang, and the encoded floors.

## Three defects the trial found, and one it could not pin down

Two of these were measured under the concurrency the trial creates and are
mutation-proven; the third was observed **once** and is recorded as such rather
than dressed up.

1. **`database is locked` on concurrent first open** — every product command opens
   its own connection and ran an unconditional journal-mode switch, which takes a
   brief exclusive lock and does not honour `busy_timeout`; two of 100 submissions
   failed while the vault was being created. Fixed with a busy timeout, a
   mode query so the switch only happens on a non-WAL file, and a bounded retry
   around the switch. Readers were hit too: **two concurrent ledger reads failed**
   and the UI would have reported a broken vault.
2. **`UNIQUE constraint failed: schema_migrations.migration_id`** — two
   connections opening a brand-new vault at once both saw migration 0001 as
   unapplied and both inserted the bookkeeping row; the loser's open failed, and
   the caller saw *"cannot open vault"* on a vault being created correctly. Fixed
   with `INSERT OR IGNORE`, which is correct because losing that race means
   another writer applied the same (idempotent, `IF NOT EXISTS`) DDL.
   **Provenance, stated precisely:** this failure was observed once, in the short
   stress workload, before the fix. Two attempts to build a deterministic
   regression test (8 threads opening a fresh vault; 8 threads writing into one)
   did **not** reproduce the interleaving, so **no mutation proof is claimed for
   this fix**. `MUT-STRESS-001` was withdrawn from the mutation manifest rather
   than left as a failing entry or reported as caught when it survived.
3. **Backup failed while writers were active** (`database is locked`, once in six
   backups). The online-backup API's contract is to restart when its source
   changes; the copy is now restarted on a lock error. **Its necessity is also not
   demonstrated**: `MUT-STRESS-002` (removing the restart) SURVIVED with four
   writers and thirty backups, so the restart is recorded as a documented
   defensive measure rather than a proven fix, and that mutation entry was
   withdrawn too.

A fourth issue was in the **trial itself**: readers start before the first write
commits, so a read is legitimately refused with "no vault at …". Counting that as
a concurrency defect accused the product of doing the right thing, and
re-classifying it by re-checking `exists()` still raced (one legitimate refusal
was counted even after the vault existed). The classification is now by the error
message, which cannot race.

## Why two mutations were withdrawn instead of kept

DOD-018 requires a controlled defect whose removal makes the guarding test fail.
For the two fixes above I could not make the test discriminate, and the harness
reports `SURVIVED` rather than a pass. Leaving those entries in place would either
fail the harness forever or tempt a weakened test; removing them and stating the
provenance in the evidence is the honest option. The two fixes stay in the code
because they match observed failures and are safe (`OR IGNORE` on an idempotent
DDL, and a bounded restart around a documented API contract).

## Tests added

* `test_concurrent_first_open_of_a_fresh_vault_succeeds` (storage) — eight
  simultaneous first opens, each yielding the full migration set.
* `test_backup_succeeds_while_another_connection_writes` (storage) — four writers
  and thirty backups, with the last backup reopened as a usable vault.
* `concurrent_writes_into_a_fresh_vault_all_succeed` (desktop) — eight threads
  writing into a new vault; no failures and no lost events.

Each asserts an invariant that matters (no lost work, no spurious failure) even
though none of them reliably reproduces the narrow bookkeeping race.

## Limits — stated, not implied

* One machine, one disk, **threads** rather than separate processes or hosts; the
  product's real concurrency is a single UI process plus its own commands.
* Throughput and latency are regression bounds on this host, not capacity claims.
* The workload is fixed at 2 000 events; no specified scale exists to reach.
* Busy/locked failures are only excused when the message says so — every other
  failure fails the trial outright.

# DOD-017 — duplicate, retried, reordered, delayed and concurrent operations

DOD-017 RULE: "Duplicate, retried, reordered, delayed, and concurrent operations
preserve idempotency, integrity, and the documented delivery semantics."
REQUIRED EVIDENCE: "Idempotency keys, concurrent traces, commit/ack fault cases,
final reconciliation, and invariant results."

## The defect this closes

`record_conception` minted a **new UUID per call**, so a transport-level retry —
the client never learned whether the first attempt landed — stored the same
invention text **twice**, under two ids and two audit entries, and nothing in the
response said so. A client could not tell a fresh write from a replay, which makes
reconciliation impossible and inflates the ledger with duplicate conception
events: exactly the integrity loss the clause forbids.

## What exists now

| Layer | Artifact | Semantics |
| --- | --- | --- |
| Storage | `Vault::put_conception_event_keyed` → `IdempotentWrite::{Recorded, Replayed}` | a key never seen is recorded; a key replayed with the SAME content is a no-op returning the event the first attempt stored; a key replayed with DIFFERENT content is refused (`VaultError::IdempotencyConflict`, naming the key and both fingerprints) |
| Storage | `Vault::find_conception_event` | the replay lookup, by event identity |
| Command | `commands::record_conception_keyed` | the product boundary: reports `replayed` in the outcome and says "nothing written" in the storage detail, so a caller cannot double-count |
| IPC | `lib.rs` `record_conception_keyed` | registered and recorded in the operator diagnostics log like every other command |

The key is the **event identity the client chooses**, which is why the ledger
shows the key as the event id: identity is what makes a replay recognisable, and a
separate random token plus a resource id would be two things to keep consistent
for no additional safety. The payload fingerprint (`sha256` of the content) is
what makes the key safe to reuse-check.

## Executed evidence, property by property

| Clause term | Test | Assertion |
| --- | --- | --- |
| **retried** | `test_keyed_writes_are_idempotent_and_conflicts_are_refused` | the retry is a replay, names the first event, and does **not** restamp `created_utc` (no new work) |
| **duplicate** | same test | after three submissions of one key, `list_conception_events` returns exactly **1** |
| **reordered + delayed** | same test | keys arriving out of order (`key-c` then `key-b`) plus a later duplicate of `key-b` yield exactly **3** events for 3 keys |
| **concurrent** | `test_concurrent_same_key_produces_one_event` | four threads released from a barrier write one key: one event, every caller names it |
| **commit/ack fault** | first test | a write refused before commit (unknown workspace) leaves **no** row, and the same key is then usable successfully |
| **integrity** | first test | the same key with a different payload is refused, and the refusal names the key and both fingerprints |
| **reconciliation at the boundary** | `test_keyed_recording_reports_replays_and_refuses_key_reuse` | the command reports `replayed=true` with "nothing written", and the ledger stays at 1 event across three submissions |
| **across the packaged artifact** | `apps/desktop/e2e-artifact.mjs` | over real IPC: first write `replayed=false`; retry `replayed=true` naming the **same** event; ledger `3 -> 3`; different content under the same key **refused** |

A keyed write with **no vault** is refused rather than pretending to be
idempotent, because without durable state a replay cannot be distinguished from a
first attempt.

## Mutation proofs

- `MUT-DOD-017-a` — `classify_replay` reports every keyed write as freshly
  recorded: **CAUGHT** by the vault test, which asserts `is_replay()`.
- `MUT-DOD-017-b` — a key replayed with different content is accepted as a
  replay: **CAUGHT** by the product-boundary test, which asserts the refusal.

Both mutants were restored byte-for-byte and the suite reran green.

## Limits — stated, not implied

* The concurrent evidence is a **barrier-synchronised 4-thread test**, not a
  captured distributed trace: it demonstrates that the primary key plus
  re-read-on-conflict resolves a race, on one machine and one engine.
* Idempotency covers the **conception-write path** only. Other state-changing
  commands (claim links, docket advances, receipts) are individually idempotent by
  construction — junction inserts use `INSERT OR IGNORE`, and blob storage is
  addressed by content hash — but they do **not** accept a client key, so a retry
  there is not distinguishable from a second intentional action.
* There is no **in-doubt transaction resolution**: if a process dies mid-commit,
  SQLite's own atomicity decides, and the client's recourse is to resend the same
  key — which is precisely what the replay path handles.
* No **time-based de-duplication window** is implemented; keys are kept
  indefinitely, which is a storage cost rather than a correctness gap.

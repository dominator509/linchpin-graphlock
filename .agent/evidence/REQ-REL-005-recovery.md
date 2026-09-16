# REQ-REL-005 — backup, restart, recovery and rollback against reconciled state

Status: **IMPLEMENTED AND EXECUTED**, with the limits stated at the bottom.

REQ-REL-005 requires that "Backup/restart/recovery and update/rollback are
executed against reconciled persistent state." The parent clauses are DOD-015
(persistent state survives a hard restart), DOD-035 (update/rollback paths) and
DOD-036 (backup, restore, disaster recovery, hard-failure recovery, RPO, RTO,
MTTR — "executed against reconciled state").

## What exists

| Layer | Artifact | What it does |
| --- | --- | --- |
| Storage | `crates/storage/src/vault.rs` — `state_digest`, `backup_to`, `digest_of`, `restore_from` | Content digest; online-backup snapshot; read-only reconciliation; non-destructive restore into a live connection |
| Command | `apps/desktop/src-tauri/src/commands.rs` — `backup_vault`, `restore_vault` | The product boundary, reachable over IPC |
| IPC | `apps/desktop/src-tauri/src/lib.rs` — `backup_vault`, `restore_vault` | Registered Tauri commands resolving the real vault path from `AppConfig` |
| UI | `apps/desktop/src/App.tsx` — Settings → *Durable state recovery* | Operator-reachable backup, and a confirmation-gated restore |
| Tests | `crates/storage/src/vault.rs`, `commands.rs` tests, `apps/desktop/src-tauri/tests/recovery_drill.rs`, `apps/desktop/e2e/shell.spec.ts` | Unit, boundary, executed drill and UI-lane evidence |
| Harness | `scripts/mutation-proof.py` | Runs the controlled defects and refuses to accept a non-compiling mutant as a catch |

Backups use SQLite's **online-backup API**, not a file copy: a copy can capture a
torn write-ahead log, and a backup that silently restores to a corrupt state is
worse than no backup.

Reconciliation is a digest of durable **content** (conception events and the
claim/support graph), ordered and normalised, not file bytes — so it compares
logical state and is unaffected by page layout or WAL framing.

## Defects found and fixed while producing this evidence

Every one of these was found by executing something, not by reading code.

1. **A refused restore was accepted and destroyed the vault.** `Vault::digest_of`
   used `Vault::open`, which has SQLite's open-or-create semantics. A mistyped
   backup path was therefore *created* as an empty database, digested as "no
   state", and reported as a valid reconciliation target — so `restore_vault`
   accepted it and replaced the real vault with emptiness. Measured by
   `test_backup_and_restore_commands_reconcile`, which failed with
   `restoring from a missing backup was accepted`. Fixed by refusing an absent
   path (`VaultError::BackupMissing`) and by opening read-only.
2. **Reconciliation mutated the thing it measured.** `digest_of` ran migrations
   against the backup file. It now opens with `SQLITE_OPEN_READ_ONLY` and does
   not migrate, so a backup is never altered by being inspected and a file that
   is not a vault fails closed instead of being given a schema.
3. **Recovery was impossible exactly when it was needed.** `restore_vault`
   refused when the destination vault did not exist ("no vault at …") and
   refused again when it existed but was unreadable ("cannot open vault"). Those
   are the two states a hard failure leaves behind, so the product could not
   recover from either. The command now recreates an absent destination and
   **quarantines** an unreadable one — renamed aside and preserved, never
   deleted — reporting both in `RestoreView` (`destination_recreated`,
   `destination_quarantined`).
4. **"Opens" is not "readable".** A corrupt database can survive `Vault::open`
   and only fail when the content is queried. `open_for_recovery` opens *and*
   digests, so the corruption case is detected rather than refused.
5. **A refused recovery has no side effects.** The source is validated before the
   destination is touched, so a mistyped backup leaves a broken vault exactly
   where it was; quarantining it would destroy the operator's only remaining
   copy. Asserted directly in `test_restore_recovers_from_a_lost_and_from_a_corrupt_vault`.
6. **A test helper wrote to a different workspace than its caller named**, which
   made a recovery test query an empty vault and look like a product defect. The
   helper now takes the workspace id.
7. **The drill wrote its report outside the repository.** `../../.agent/…` from
   `apps/desktop/src-tauri` is `apps/.agent/…`; the run passed while writing
   evidence nothing reads. The path is now three levels up and asserted.
8. **A mutation that does not compile would have been counted as a catch.**
   Discovered while building the harness: a build error exits non-zero exactly
   like a failing test. `scripts/mutation-proof.py` separates `CAUGHT` from
   `BUILD_ERROR` and fails on the latter, and refuses a mutation whose anchor text
   does not match exactly once (a no-op mutant would otherwise be reported as
   `SURVIVED`).

## Executed evidence

**Unit / boundary**

* `cargo test -p storage` → 23 lib + 5 integration, including
  `test_backup_and_restore_reconcile_to_the_snapshot`,
  `test_recovered_state_survives_a_hard_restart` (state survives a full close and
  reopen) and `test_digest_of_is_read_only_and_refuses_non_vaults`.
* `cargo test -p linchpin-desktop --lib` → 47 passed, including
  `test_backup_and_restore_commands_reconcile` and
  `test_restore_recovers_from_a_lost_and_from_a_corrupt_vault`.
* `cargo test -p linchpin-desktop --test recovery_drill` → 1 passed.

**Measured objectives** — `.agent/evidence/recovery-drill/report.json`

Five full disaster/recovery cycles, alternating fault classes, with a fresh
per-run canary in the state and 25 events written through the production
`record_conception` command before each backup:

| Objective | Measured |
| --- | --- |
| RPO (backed-up state lost) | **0 events** |
| RPO (work committed after the last backup) | **unbounded** — no off-device replication; the drill *asserts* that this work is lost |
| RTO (recovery command, wall clock) | median **38.89 ms**, min 37.26 ms, max 52.30 ms over 5 cycles |
| MTTR (same fault class) | median **38.89 ms**; the repair is the recovery command itself |
| Backup duration | median 13.86 ms |
| Automation threshold | 10 000 ms per cycle; exceeding it fails the drill rather than being reported |
| Reconciliation | exact digest match, both fault classes, every cycle |
| Fault detection | observed, not assumed: absence for total loss, unreadable-for-digest for corruption |

**Mutation proofs** — `.agent/evidence/mutation-proof/REPORT.json`

| ID | Injected defect | Result |
| --- | --- | --- |
| MUT-REL-005-a | `digest_of` reverts to open-or-create semantics | CAUGHT by `test_digest_of_is_read_only_and_refuses_non_vaults` |
| MUT-REL-005-b | `restore_from` reports success without copying a page | CAUGHT by `test_backup_and_restore_reconcile_to_the_snapshot` |
| MUT-REL-005-c | both guards between a mistyped path and the destructive step removed (the original defect) | CAUGHT by `test_backup_and_restore_commands_reconcile` |

Each mutant was compiled, caught, restored byte-for-byte, and the guarding test
rerun green — the harness fails if restoration or the green rerun does not hold.

**UI lane** — `apps/desktop/e2e/shell.spec.ts` → 19 passed, including
"exposes backup and recovery and states the point-in-time limit", which asserts
the controls exist, that the surface states a backup is a point-in-time snapshot
whose restore discards later work, and that the restore requires a
consequence-specific confirmation before it acts.

**Installer lane** — `scripts/vault-preservation-e2e.sh` (lane 3 of
`scripts/test-e2e.sh`) proves realistic persistent state survives installing over
an existing product, uninstalling and reinstalling, at the product's real
app-data path, with a per-run canary and a mutation proof.

## Limits — stated, not implied

* The drill is a **single machine, single process, small vault on local disk**. It
  does not measure recovery across machines, across versions, or at production
  data volumes.
* There is **no off-device replication**. RPO after the last backup is unbounded:
  losing the device loses everything committed since that backup. The drill
  asserts the loss rather than describing the backup as continuous protection.
* The measured RTO/MTTR are wall-clock observations on this machine under this
  workload, with a 10 s automation threshold; they are not a service-level
  guarantee for other hardware.
* **Cross-version update/rollback remains unexecuted** (DOD-035): the update and
  rollback lanes install and remove the *same* v0.1.0 MSI, because no second
  released version exists. Same-version reinstall is weaker evidence than a
  cross-version upgrade, and this document says so rather than implying a matrix.
* The quarantine path preserves an unreadable vault but makes **no attempt to
  salvage** it; a corrupt database is moved aside, not repaired.

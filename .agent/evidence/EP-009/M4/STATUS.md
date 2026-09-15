# EP-009 / M4 — Update, rollback and uninstall data preservation

Status: **PARTIAL — data preservation EXECUTED; the signing half is a documented limitation (ADR-003), and cross-version rollback is not yet possible.**

Milestone: M4 "Signed update/rollback/uninstall-vault-preservation"
Gate: `scripts/vault-preservation-e2e.sh`, wired as **lane 3 of `scripts/test-e2e.sh`**

## Why this was previously NOT executed

M4 was never run because it was read as depending on a signature, and PF-015
(signing identity) is unavailable. That reasoning was too broad: ADR-003 resolves
the *signature* question, but "update/rollback/uninstall-vault-preservation" is a
**data-integrity** requirement that can be executed unsigned. DOD-035 requires
upgrade/rollback paths be executed "with realistic persistent state" and DOD-036
requires recovery claims be "executed against reconciled state" — neither waits
on a certificate.

## What the gate does

Against the **real MSI** on the real host, using the product's real app-data path
(`%LOCALAPPDATA%\LINCHPIN`, where `linchpin-vault.db` lives):

1. Install the MSI silently.
2. Seed **realistic persistent state**: a real SQLite database at the product's
   real vault filename containing a random canary row, plus a content-addressed
   blob in the vault directory. The canary is unpredictable per run (DOD-013).
3. **Update** — install over the existing installation.
4. **Uninstall** — remove the product.
5. **Rollback** — reinstall after removal.

After steps 2–5 the canary must be byte-intact and the row readable: database
present, blob present and unchanged, canary row present.

## Measured result

```text
=== 1/5 install the artifact ===          install exit 0
=== 2/5 seed realistic persistent state === canary intact (1 row(s))
=== 3/5 UPDATE: install over the existing === reinstall (update) exit 0
                                            canary intact (1 row(s))
=== 4/5 UNINSTALL ===                       uninstall exit 0
                                            canary intact (1 row(s))
=== 5/5 ROLLBACK ===                        reinstall (rollback) exit 0
                                            canary intact (1 row(s))
vault-preservation: ok
```

Whole-suite run: `sh scripts/test-e2e.sh` exit 0 with all three lanes green.

## Mutation proof (DOD-018)

A gate that always passes proves nothing, so the destructive case was injected:
a step deleting the seeded database and blob immediately before the uninstall
check — i.e. simulating an uninstaller that wipes user data. Result:

```text
MUTATION 'destructive uninstaller': ok -- gate FAILED as required
    canary intact (1 row(s))     <- steps 2-3 still fine
    vault database is GONE       <- step 4 caught it
```

Restoring the script returned it to green. The gate therefore discriminates
between a preserving installer and a destructive one.

## Safety properties of the gate itself

- The canary is removed on **every** exit path via a `trap ... EXIT`, so even a
  red run cannot leave invented state inside the product's real app-data
  directory. Verified after the mutation run: `%LOCALAPPDATA%\LINCHPIN` does not
  exist.
- It does **not** weaken the `storage` crate's guard that forbids unit tests from
  writing underneath the production vault root. The canary is placed by this
  installer-level harness, not by a test utilising that root.

## What this does NOT prove — stated so it is not read as more than it is

1. **Cross-version rollback is not tested.** Both the "update" and the "rollback"
   install the *same* MSI (v0.1.0). No second version exists, so version-skew,
   mixed-fleet and downgrade-to-older-schema paths remain unexecuted.
   DOD-035 stays FAIL for those paths.
2. **No live vault write through the product's own command layer.** The installed
   release build has no debug port, so `record_conception` was not driven against
   the installed binary here. Vault durability and schema migration are covered
   separately by DOD-015/DOD-016 evidence.
3. **No signature.** Covered by ADR-003 as an accepted limitation, not by this
   gate.

## Effect on clause status

- **DOD-036** — the uninstall/reinstall reconciliation is now executed, but RPO/
   RTO/MTTR and fault injection are still absent, so the clause does not become
  PASS.
- **DOD-035** — the update and rollback *mechanism* is exercised; because only one
  version exists, the compatibility matrix cannot be executed. Stays FAIL, but
  its recorded reason is now precise.

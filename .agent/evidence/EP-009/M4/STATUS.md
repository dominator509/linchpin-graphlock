# EP-009 / M4 — Update, rollback and uninstall data preservation

> ## CURRENT STATE (added later; the body below is the historical record at
> ## candidate `1604012` and is preserved, not rewritten)
>
> M4 is **DONE for every path this product claims**. The historical body ends by
> naming two open items; both are now closed or resolved:
>
> | Item the historical body lists as open | Current state |
> | --- | --- |
> | Cross-version rollback untested / DOD-035 stays FAIL | **closed by execution.** A second release exists and the matrix now installs it: `scripts/version-matrix.py` builds v0.1.0 and v0.2.0 through the production build path (manifests bumped, `cargo update --workspace --offline` so `--locked` still holds, manifests and lockfiles restored byte-for-byte with `git diff --exit-code` asserted), then runs install → upgrade → downgrade attempt → uninstall → rollback against realistic persistent state at the product's real app-data path. DOD-035 is PASS |
> | No live vault write through the installed binary | covered at the boundary: the exact-artifact lane drives the packaged executable's real WebView2 over CDP with a real vault write, and the matrix launches the upgraded binary from its installed location and observes it alive |
> | No signature | ADR-003: unsigned release accepted as a documented limitation, not a skipped step |
>
> ### Measured result at the current candidate
>
> Re-provisioned and re-run against source `62da949ba` (build-input fingerprint
> `d5bb1c86337a` over 82 tracked inputs), all eight steps PASS:
>
> | Step | Measured |
> | --- | --- |
> | clean_start | nothing installed, `msiexec` exit 0 |
> | install_old | DisplayVersion `0.1.0`, installed exe `305328ddfa3074fe…` equals the package's **own** payload digest |
> | seed_state | canary database + content-addressed blob written at `%LOCALAPPDATA%\LINCHPIN`, intact |
> | upgrade | DisplayVersion `0.2.0`, binary **replaced** (`305328dd…` → `dddc99c8bab6d013…`), matches payload, upgraded binary **launched and observed alive**, canary intact |
> | downgrade_attempt | applied back to `0.1.0`, installed version agrees with the outcome, canary intact |
> | uninstall | product removed, canary intact |
> | rollback | `0.1.0` reinstalled, payload digest matches, canary intact |
> | cleanup | canary removed, nothing left installed |
>
> Live evidence: `.agent/evidence/version-matrix/report.json` and `STATUS.md`.
>
> ### A defect in this harness was found and fixed while producing this evidence
>
> The matrix was **stale and did not know it**. `staged()` only checked that the
> artifact files existed, so the run on 2026-09-17 reused packages built on
> 2026-09-16 while production code had changed in between: the recorded PASS
> described a candidate that no longer existed. DOD-040 requires a changed
> candidate to invalidate and **rerun** affected results, so this was a real
> accounting defect, not a cosmetic one.
>
> The fix binds the staged artifacts to the source that produced them:
> `target/version-matrix/STAGE.json` records the commit and a fingerprint over the
> 82 tracked build inputs; the report and `STATUS.md` carry both; a mismatch makes
> the lane **re-provision instead of reusing**, and `--check-source` fails outright
> naming both revisions. The lane is wired into `verify.sh`, so a stale matrix can
> no longer sit in the tree claiming a PASS for a moved candidate.
>
> Both branches were proven rather than assumed: injecting a mismatching
> fingerprint made `--check-source` fail with
> `staged artifacts were built from 000000000 (fingerprint deadbeefdead) but the
> current source is 62da949ba (fingerprint d5bb1c86337a)`, and the same injected
> state made the matrix print `STALE STAGE … re-provisioning both versions
> (DOD-040 rerun obligation)` and rebuild. (The first revision of the fix read the
> provenance at the wrong level and compared `None` against `None`, printing
> `built from ? (fingerprint ?)`; that was caught by running it.)
>
> ### Mutation proof, refreshed against the current artifacts
>
> `python3 scripts/version-matrix.py --mutate destructive-upgrade` injects an
> upgrade that **destroys** the seeded state. With the same staged artifacts as the
> green run, the matrix fails at four steps — `upgrade`, `downgrade_attempt`,
> `uninstall`, `rollback` all report `state_intact: false` — so the green run
> discriminates between an upgrade that preserves user data and one that does not.
> Record: `.agent/evidence/version-matrix/report-mutation-destructive-upgrade.json`.
>
> ### Still not claimed
>
> - mixed-fleet and multi-machine version skew: **NOT APPLICABLE** — a single-user
>   desktop product with device-local storage, no server fleet, and no such support
>   claimed. Recorded in the report's `applicability_notes` rather than dropped.
> - A signed update channel: ADR-003.
>
> ### Milestone verdict
>
> **DONE.** Update, downgrade, rollback and uninstall preservation are executed
> across two real versions with realistic persistent state, mutation-proven,
> source-bound and currency-enforced.

Status at the time of writing: **PARTIAL — data preservation EXECUTED; the signing half is a documented limitation (ADR-003), and cross-version rollback is not yet possible.**

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

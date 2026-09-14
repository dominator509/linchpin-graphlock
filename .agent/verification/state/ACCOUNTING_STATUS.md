# Verification Accounting Status (honest, measured)

Measured 2026-09-10 at candidate `f4f41cc`.

## Headline

```
$ sh scripts/harness-accounting.sh
registry 484 accounted 484
[exit=0]

$ sh scripts/harness-validate.sh
generated pack validation: ok
hash-ledger validation: ok
accounting check: ok (484 IDs, one status each)
traceability check: ok (22 requirement rows)
dod-status check: ok (42 clauses, one disposition each)
collection guard manifest: ok (min 85 passed)
harness-validate: ok
```

**All 484 registry IDs carry exactly one accounted status.** At base revision
this was `registry 484 accounted 0`, exit 2.

**Read this correctly: 484/484 accounted is NOT 484/484 passing.** Current:
477 `NOT_RUN_BLOCKED_MATERIAL` + 7 `PARTIAL`. **Zero IDs have PASS.**

## Gate matrix at this candidate

| Gate | Exit | Note |
| --- | --- | --- |
| `preflight.sh` | 0 | `preflight: ok` |
| `harness-validate.sh` | 0 | 6 enforced checks |
| `lint.sh` | 0 | clippy `-D warnings` + eslint |
| `format-check.sh` | 0 | prettier across 3 packages |
| `typecheck.sh` | 0 | tsc across 3 packages |
| `test-integration.sh` | **0** | was exit 2 — real file-backed SQLite + tantivy, 5 tests |
| `test-e2e.sh` | **0** | was exit 1 — 5 Playwright tests in real Edge |
| `smoke-test.sh` | **0** | was exit 2 — MSI install/launch/uninstall with pinned digest |
| `cargo test --workspace` | 0 | 26 suites, 85 passed, 0 failed, 0 ignored |
| `cargo deny check advisories` | 0 | 5 unmaintained notices ignored by exact RUSTSEC id |
| `cargo deny check bans` | 0 | duplicate-version warnings retained |
| `cargo deny check sources` | 0 | — |
| `cargo deny check licenses` | **4** | 5 MPL-2.0 transitive crates pending ADR-002 legal review |
| `production-readiness-check.sh` | 1 | correct — no GO verdict exists |
| `live-fire.sh` | 2 | `run-live-fire-real.sh` still absent |

## Executed registry results (CASE_RESULTS.json, 7)

| ID | Status | What ran |
| --- | --- | --- |
| SUP-004 | PARTIAL | Two release builds compared; 19 differing bytes isolated to PE timestamp + PDB GUID/age; normalized comparison identical |
| SUP-003 | PARTIAL | MSI built, identity read back via Windows Installer API, installed/launched/uninstalled |
| E2E-011 | PARTIAL | Exact installer installed, launched, cleanly removed with verified pre/post state |
| E2E-002 | PARTIAL | 5 Playwright tests in real Edge against the production bundle |
| E2E-012 | PARTIAL | File-backed SQLite survives connection close; mutation-proven |
| SUP-006 | PARTIAL | 4×25 concurrent inserts; final count equals successful inserts |
| GEN-016 | PARTIAL | Missing schema fails closed with `no such table` |

## Defects found and repaired this session

**12 fabrication/bypass defects** (all with failing-then-passing proofs), plus
**10 harness/infrastructure defects**: icons blocking all Windows builds, dead
accounting, a validator that never skipped anything, cargo-deny with no config,
zero-test masking, masked ledger validation, non-reproducible builds, and the
three previously-absent runners (integration, E2E, smoke).

Nine of the twelve were found by writing **probe tests** against existing APIs
rather than by reading code.

## Verified-red / blocking gaps (unchanged)

- **ADR-002**: 5 MPL-2.0 transitive crates need the legal review
  `LICENSE_ALLOWLIST.md` requires. Left intentionally red rather than
  allowlisted.
- **PF-015/PF-016/PF-017/PF-018**: no signing identity, no clean-room VM, no
  human UAT or accessibility sign-off. DOD-039 forbids an agent impersonating
  sign-off.
- **DOD-041**: applicability matrix still holds one ALL-484 placeholder row.
- **live-fire.sh**: `run-live-fire-real.sh` absent; UO-01..12 have no real runs.
- **DOD-034/035/036**: no clean room, no upgrade/rollback, no backup/restore.
- Graph still `RUN_BLOCKED dependencies prevent: EP-010`.



# Verification Accounting Status (honest, measured)

Measured 2026-09-10 at candidate `627c82b`.

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
collection guard manifest: ok (min 49 passed)
harness-validate: ok
```

**All 484 registry IDs carry exactly one accounted status** (484 rows, 484
unique, 0 missing, 0 duplicated). At base revision this was
`registry 484 accounted 0`, exit 2.

**Read this correctly: 484/484 accounted is NOT 484/484 passing.** Current:
481 `NOT_RUN_BLOCKED_MATERIAL` + 3 `PARTIAL`. **Zero IDs have PASS.**

## DoD clause dispositions (DOD_STATUS.jsonl, 42 clauses)

| Status | Count |
| --- | --- |
| PASS | 7 |
| PARTIAL | 15 |
| FAIL | 19 |
| EXTERNAL_REQUIRED | 1 |

The single EXTERNAL_REQUIRED is DOD-039 (human UAT / assistive-technology /
legal / signing) — an AI may not impersonate it. `scripts/build-dod-status.py`
refuses to emit PASS when the named evidence file is absent, and all 7 PASS
clauses were checked to have existing evidence.

DOD-042 is FAIL: `RELEASE_GATE.json` is `NOT_EVALUATED`. **NO_GO** is the only
lawful verdict at this state.

## Executed registry results (CASE_RESULTS.json)

| ID | Status | What ran |
| --- | --- | --- |
| SUP-004 | PARTIAL | Two release builds compared; non-identical; 19 differing bytes isolated to PE timestamp + PDB GUID/age; normalized comparison identical |
| SUP-003 | PARTIAL | MSI + NSIS built; digests pinned; identity read back via Windows Installer COM API; installed, launched, uninstalled |
| E2E-011 | PARTIAL | Exact installer installed to `C:\Program Files\LINCHPIN`, launched to a responsive window, cleanly uninstalled |

## Defects found and repaired this session (all with before/after proof)

1. **No Windows build possible** — all five icon assets were one identical PNG with `.ico`/`.icns` extensions; `cargo build` died with RC2175 (exit 101).
2. **Fabricated provider output** — `provider_transport` returned literal mock strings with no network call; tests asserted the mock text.
3. **No-op redaction** — `crash_reporter` set a boolean without transforming content, so unredacted invention text passed the export guard.
4. **Vacuous fuzz test** — discarded results, asserted nothing.
5. **Harness accounting dead** — `accounted 0`, exit 2.
6. **Pack validator never skipped anything** — `any(part not in SKIP_DIRS)` is true of every absolute path; scanned 37,604 files, 53.8s, thousands of false errors.
7. **cargo-deny had no config** — empty allowlist rejected ~700 crates unconditionally.
8. **Zero-test masking** — all four `package.json` files used `vitest run --passWithNoTests`; the JS lane passed green with **zero test files**.
9. **Ledger validation masked** — `harness-validate.sh` ended with `|| true`, bypassing hash-chain tamper detection.
10. **Non-reproducible builds** — measured, root-caused, remediation attempted (added the missing `[profile.release]`), did not succeed; recorded PARTIAL.

Icons, DOD-006, DOD-007 and DOD-024 statuses were **downgraded** where the
evidence did not support them.

## Verified-green gates

`preflight.sh`, `harness-validate.sh` (6 checks), `lint.sh`, `format-check.sh`,
`typecheck.sh`, `cargo build --workspace`, `cargo test --workspace --locked`
(25 suites, 49 passed, 0 failed, 0 ignored), `cargo deny check advisories`,
`cargo deny check bans`, `cargo deny check sources`, artifact build + install +
launch + uninstall.

## Verified-red gates (honest gaps)

| Gate | Cause |
| --- | --- |
| `cargo deny check licenses` | 5 MPL-2.0 transitive crates pending the legal review ADR-002 requests |
| `production-readiness-check.sh` | release verdict is not GO — correct |
| `test-integration.sh` | real-dependency runner absent |
| `test-e2e.sh` | no package defines `test:e2e` |
| `smoke-test.sh` | exact-artifact smoke runner absent |
| `live-fire.sh` | `run-live-fire-real.sh` absent |
| `test-unit.sh` | JS lane now fails loudly because it has zero test files |

## Blocking gaps (unchanged)

- **DOD-001**: 20 of 22 requirement IDs are declared in no spec. `REQ-REL-001`/`REQ-REL-005` remain undefined, so no acceptance test can be written for them. Fixing needs an S1 spec ADR.
- No coverage measurement, no restart-persistence proof, no migration matrix, no idempotency/concurrency run, no soak/stress.
- No SBOM/notices/provenance/signing (PF-015).
- No clean room (PF-016), no Windows 11, no UAT/accessibility sign-off (PF-017/018).
- DOD-041: the applicability matrix still holds one ALL-484 placeholder row; 449 conditional IDs have no evidence-attached decision.
- Graph still `RUN_BLOCKED dependencies prevent: EP-010`.


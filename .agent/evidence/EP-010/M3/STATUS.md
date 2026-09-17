# EP-010 / M3 — Architecture, claim and spec-to-code drift reconciliation

Status: **DONE — a real drift gate now exists, it found real drift, the drift was
repaired, and the gate proves it can find drift again.**

## The gate

`scripts/architecture-drift.py`, run as a lane of `scripts/verify.sh`, implements the
nine-question review of `ARCHITECTURE.md` section 145. Mechanical checks:

| Check | What it does |
| --- | --- |
| `truth_boundaries` | the five statements in `ARCHITECTURE.md` section 5 must be transcribed **verbatim**, id for id, into `crates/domain/src/scope.rs`; a missing or reworded boundary fails |
| `import_law` | domain must not depend on Tauri/SQL/HTTP/OS/provider/MCP/UI packages; inward layers (domain, application) must not depend on concrete adapter crates; a declared workspace dependency nothing references is a dead edge and fails |
| `local_confidentiality` | production source must contain no non-loopback URL literal outside an explicit, reasoned allowlist (test files and `#[cfg(test)]` modules excluded) |
| `bindings` | the remaining review items are bound to the executed gate or symbol that proves them and fail when a binding disappears |

Bound (existence-verified, not re-run by this gate): evidence provenance,
high-impact authorization (`McpServer::check_capability`, deny-by-default),
rebuildable derived state (recovery drill), terms-gated provider behavior
(`commands::provider_status` — the local lane is measured, every remote lane reports
`configured=false` with the reason it is not wired), deterministic filing manifests,
exact-artifact proof.

Evidence: `.agent/evidence/architecture-drift/STATUS.md` and `REPORT.json`.

## Drift that was found and repaired (not documented away)

1. **`crates/application` depended on concrete adapters.** `[dependencies]` listed
   `storage`, `domain` and `platform_windows`, while `ARCHITECTURE.md` section 3 says
   application "may not depend on concrete provider or storage implementation types"
   and "inward layers never import adapters".
   - `storage` and `domain` were referenced by **nothing** in the crate (measured: zero
     occurrences of `storage::` or `domain::`): dead dependency edges. Removed.
   - `platform_windows` was a real production edge, used by the soak instrumentation to
     read the process working set. It is now a port the caller injects
     (`OperationsSoakTest::with_probe`), and the adapter moved to `[dev-dependencies]`,
     where the tests that assert a **real** memory measurement still use it. All ten
     `cargo test -p application` tests pass, including
     `test_soak_measures_real_memory_growth` and the real-app-data-path health probe.
   - `Cargo.lock` was refreshed offline for the workspace members only and every lane
     still builds with `--locked`.
2. **Three false positives in the first version of the gate itself.** `crates/*/tests/*.rs`
   URL literals were reported as shipped defaults before the check excluded
   integration-test, bench, example and fixture trees. Fixed in the gate, with the
   measured false positive recorded in the source comment.

## Discrimination is proven, not assumed

`python3 scripts/architecture-drift.py --self-test` builds a fixture tree containing a
missing truth boundary, a **reworded** truth boundary, a forbidden `reqwest` dependency
in domain, an `application -> storage` adapter edge, a dead dependency edge and a
non-loopback default URL, and requires every rule to catch its own violation; it then
requires the same rules to report nothing on a clean fixture. It passes. A drift gate
that has never failed would be decoration.

## Residual limits, stated

- The bound items are **existence-verified**: this gate proves the binding is present,
  not that the bound lane passed. Their own lanes (listed in the generated evidence)
  run in `verify.sh` and fail independently.
- "Claim drift" is checked structurally (product claims are rendered from the declared
  scope in `crates/domain/src/scope.rs`, guarded on the export path, and asserted
  against the packaged executable — `REQ-SCOPE-001`), not by reading prose for
  overstatement. Prose review remains a human act and is part of DOD-039.

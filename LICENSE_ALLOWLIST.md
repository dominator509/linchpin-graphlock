# License Policy

Default redistributed-core allowlist: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib. MPL-2.0 requires explicit file-level review. GPL, LGPL dynamic-link questions, AGPL, SSPL, BSL, FSL and custom/source-available licenses require an explicit legal/license ADR before inclusion and are not allowed by default in distributable core.

**MPL-2.0 — permitted by ADR-002 (decided 2026-09-14).** The file-level review was completed for the six crates in the dependency graph: `cssparser` 0.36.0, `cssparser-macros` 0.6.1, `dtoa-short` 0.3.5, `selectors` 0.36.1, `option-ext` 0.2.0 and `htmlescape` 0.3.1. Five arrive through the Tauri foundation; `htmlescape` arrives through Tantivy, which this project's own `storage` crate depends on. None is modified, and the notice obligation ships in `.agent/evidence/sbom/THIRD_PARTY_NOTICES.md`. Full analysis: `.agent/evidence/ADR-002-mpl-dependencies.md`. This entry is a change-control record, not a default: any *new* MPL-2.0 crate, and any modification to these six, requires a fresh ADR.

Every dependency addition records SPDX ID, version, source URL, direct/transitive status, shipped/not-shipped, notices, security status and why it is needed. `cargo-deny`, JavaScript license scanning, Python license inventory and SBOM generation must agree before release.

Candidate foundations: Tauri, DuckDB, Tantivy, Playwright and PQAI are accepted only after exact-version and transitive dependency audit. PQAI is optional and benchmark-gated rather than architectural core.

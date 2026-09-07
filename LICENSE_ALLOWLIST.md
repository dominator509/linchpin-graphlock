# License Policy

Default redistributed-core allowlist: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib. MPL-2.0 requires explicit file-level review. GPL, LGPL dynamic-link questions, AGPL, SSPL, BSL, FSL and custom/source-available licenses require an explicit legal/license ADR before inclusion and are not allowed by default in distributable core.

Every dependency addition records SPDX ID, version, source URL, direct/transitive status, shipped/not-shipped, notices, security status and why it is needed. `cargo-deny`, JavaScript license scanning, Python license inventory and SBOM generation must agree before release.

Candidate foundations: Tauri, DuckDB, Tantivy, Playwright and PQAI are accepted only after exact-version and transitive dependency audit. PQAI is optional and benchmark-gated rather than architectural core.

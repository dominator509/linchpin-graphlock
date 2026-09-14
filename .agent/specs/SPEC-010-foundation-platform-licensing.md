# SPEC-010 Foundation, Platform and Licensing

REQ-FOUND-001 The product is a local-first desktop application on a Tauri 2.x
foundation with a Rust workspace of domain, application, storage, evidence,
research, patent, commercialization, provider_transport, mcp_hub,
crash_reporter and platform_windows crates, plus a React desktop client.
Workspace membership and crate boundaries are declared in `Cargo.toml`.

REQ-FOUND-002 All environment values are parsed once into typed configuration.
Unknown keys warn in development and fail in release when security-sensitive.
Secrets are never echoed. Provider subscription auth is never an environment
variable; the first-party provider tool owns it (`ENVIRONMENT.md`).

REQ-PLAT-001 Windows 10/11 x64 is the primary build and support target
(`x86_64-pc-windows-msvc`). The toolchain is pinned in `rust-toolchain.toml`
and `TOOLCHAIN_PINS.md`; `local-dev`, `integration`, `e2e-windows` and
`release-candidate` are the declared environments (`ENVIRONMENT.md`).

REQ-PLAT-002 The local-first core remains functional with a local model and
cached/user evidence, with no mandatory network dependency for core workflows
(`PROJECT_BRIEF.md`, `PROVIDER_TRANSPORT_MATRIX.md` "Local" row).

REQ-PLAT-003 Supported distribution formats are the Windows MSI and NSIS
installers produced by `sh scripts/build.sh`, each pinned by SHA-256 digest.

REQ-LIC-001 Redistributed core uses the permissive allowlist in
`LICENSE_ALLOWLIST.md`: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC,
Zlib. MPL-2.0 requires explicit file-level review. GPL, LGPL dynamic-link
questions, AGPL, SSPL, BSL, FSL and custom/source-available licenses require an
explicit legal/license ADR before inclusion and are not allowed by default.

REQ-LIC-002 Every dependency addition records SPDX ID, version, source URL,
direct/transitive status, shipped/not-shipped, notices, security status and why
it is needed. `cargo-deny`, JavaScript license scanning, Python license
inventory and SBOM generation must agree before release.

REQ-LIC-003 Dependency versions are pinned exactly and never floated. A
dependency is added only when the capability cannot be built safely with
existing code or the standard library.

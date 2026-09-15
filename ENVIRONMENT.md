# Environment

Bootstrap toolchain pins verified from first-party release sources on 2026-08-28: Rust 1.98.0; Node.js 24.20.0 LTS; pnpm 11.21.0; React 19.2.0; Python 3.13.15; uv 0.12.0; Tauri core 2.11.5, Tauri CLI 2.11.4, `@tauri-apps/api` 2.11.1 and `@tauri-apps/cli` 2.11.4. EP-000 MUST re-verify these exact pins and current security advisories immediately before repository bootstrap; any change requires an ADR plus clean compatibility proof. The accepted versions are then frozen into `rust-toolchain.toml`, `package.json` `packageManager`, lockfiles, `.python-version`, `uv.lock`, and `TOOLCHAIN_PINS.md`. Never write "latest" into executable configuration.

## Environments
- `local-dev`: real encrypted disposable workspace; local provider mandatory; cloud provider lanes optional.
- `integration`: isolated accounts/endpoints, disposable datasets, real service adapters.
- `e2e-windows`: clean Windows 10 or higher target (ADR-004 scoped the matrix to Windows 10+; Windows 11 is not verified), **unsigned** candidate per ADR-003, installer-level execution. A zero-state/virgin target is still EXTERNAL_REQUIRED for DOD-034.
- `release-candidate`: frozen artifact digest; no production-code changes.

## Config
All env values are parsed once into typed configuration. Unknown keys warn in dev and fail in release when security-sensitive. Secrets are never echoed. Provider subscription auth is not an env var; the first-party tool owns it.

## Local data directories
Production user vault and test vault are physically distinct. Tests reject paths underneath the production vault root. Derived indexes can be deleted and rebuilt; canonical DB/evidence cannot.

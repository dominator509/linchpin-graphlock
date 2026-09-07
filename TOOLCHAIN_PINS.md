# Toolchain Pins

Verified bootstrap pins as of 2026-08-28 from first-party release sources. EP-000 re-verifies before creating lockfiles; a different version requires an ADR and clean compatibility/security proof.

| Tool | Exact bootstrap pin | Role |
| --- | --- | --- |
| Rust | 1.98.0 | canonical core/tooling |
| Node.js | 24.20.0 LTS | desktop JS toolchain |
| pnpm | 11.21.0 | package manager |
| React | 19.2.0 | desktop UI |
| Python | 3.13.15 | optional research sidecar |
| uv | 0.12.0 | Python environment/lock runner |
| Tauri core crate | 2.11.5 | desktop runtime |
| tauri-cli crate | 2.11.4 | Rust CLI |
| @tauri-apps/api | 2.11.1 | JS Tauri API |
| @tauri-apps/cli | 2.11.4 | JS CLI wrapper |

Do not silently float any of these versions. Transitive dependencies are lockfile-pinned and SBOM/license scanned. Provider CLIs are separately version-pinned in EP-000 only after current first-party auth/terms verification because their release cadence and terms can change independently of this blueprint.

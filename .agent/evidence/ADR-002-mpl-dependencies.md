# ADR-002: MPL-2.0 transitive dependencies from the Tauri foundation

Status: **PROPOSED — requires human legal decision (AGENTS.md §5(c) STOP condition)**
Date: 2026-09-10
Candidate: `147d854` / `x86_64-pc-windows-msvc`

## Context

`scripts/security-check.sh` runs `cargo deny check`. At base revision the
repository had **no `deny.toml` at all**, so cargo-deny applied an empty
allowlist and rejected every third-party dependency
(`error[rejected]: failed to satisfy license requirements` for ~700 crates)
plus flagged all 12 first-party workspace crates as `unlicensed`. The gate was
unconditionally red irrespective of the real dependency set — a broken gate,
not a supply-chain result.

Restoring the gate to a meaningful state (adding `deny.toml` transcribing
`LICENSE_ALLOWLIST.md`, and adding `license = "MIT OR Apache-2.0"` to the 12
workspace manifests) reduced the failure surface to a **real** finding.

## Finding

Five crates in the dependency graph are **MPL-2.0**:

| Crate | Version | Reached via |
| --- | --- | --- |
| `cssparser` | 0.36.0 | `dom_query` → `tauri-utils` → `tauri`, and `selectors` |
| `cssparser-macros` | 0.6.1 | `cssparser` |
| `dtoa-short` | 0.3.5 | `cssparser` |
| `selectors` | 0.36.1 | `dom_query` → `tauri-utils` |
| `option-ext` | 0.2.0 | `dirs-sys` → `dirs` → `tauri` / `tauri-build` |

All five are transitive only. None is a direct dependency of any LINCHPIN
crate, and none was chosen by this project.

`LICENSE_ALLOWLIST.md` states verbatim:

> "MPL-2.0 requires explicit file-level review. GPL, LGPL dynamic-link
> questions, AGPL, SSPL, BSL, FSL and custom/source-available licenses require
> an explicit legal/license ADR before inclusion and are not allowed by default
> in distributable core."

MPL-2.0 is therefore **not** on the default allowlist, and the policy demands a
file-level review plus an ADR. `LICENSE_ALLOWLIST.md` simultaneously lists
**Tauri** as an accepted candidate foundation ("Candidate foundations: Tauri,
DuckDB, Tantivy, Playwright and PQAI are accepted only after exact-version and
transitive dependency audit"), which is exactly the audit that surfaced these
crates.

## Why this is not resolved by an agent

MPL-2.0 is a **file-level weak copyleft**. Its obligations attach to modified
MPL-covered *files*, not to the larger work, so unmodified use of these crates
behind Tauri's own boundary is widely treated as permissible in proprietary
software. Two facts nevertheless matter and neither is an agent's call:

1. Whether the MPL-2.0 Exhibits/notices obligation is satisfied by whatever
   notice file the MSI ships (there is currently **no** notices/SBOM artifact —
   EP-009 M2 is unimplemented).
2. Whether the project's own "explicit file-level review" requirement is met
   for these five crates.

AGENTS.md §5(c) makes this a STOP condition: "a legal, financial, or security
judgment is required and the specs do not answer it." AGENTS.md §3 requires the
conflict be recorded in `DECISIONS.md` rather than silently reconciled.

## Options

**Option A — Accept with recorded review.** Add the five crates to
`deny.toml` `licenses.exceptions` with a per-crate rationale, and ship the
MPL-2.0 notices in the M2 SBOM/notices artifact. Requires a human to confirm
the MPL notice obligation is satisfied and to sign the file-level review.

**Option B — Replace the Tauri path.** Not viable within EP-009: Tauri is the
documented desktop foundation (`ARCHITECTURE.md`, EP-005) and removing it is a
change-control event far outside this node.

**Option C — Leave the gate red.** Honest but blocks DOD-021 and therefore any
GO verdict indefinitely.

## Recommendation

Option A, contingent on human sign-off. The crates are unmodified transitive
dependencies of an already-accepted foundation, and MPL-2.0's copyleft is
file-scoped. This ADR does **not** assert that conclusion; it records the
finding, the policy text, and the decision needed.

## Current gate behaviour (deliberate, honest)

`deny.toml` does **not** list MPL-2.0 in `allow`, and does not exception these
five crates. `cargo deny check licenses` therefore still reports
`licenses FAILED` (exit 4) with exactly these five crates. That is intentional:
adding them without the review this ADR requests would be precisely the silent
policy weakening DOD-021 and §6 forbid. `advisories`, `bans` and `sources` are
otherwise green.

## Impact if unresolved

`scripts/security-check.sh` and `scripts/dependency-audit.sh` remain FAIL.
DOD-021 records FAIL, DOD-003/DOD-042 remain blocked, and the release verdict
cannot be GO. EP-009 M2 (SBOM/notices) cannot close.

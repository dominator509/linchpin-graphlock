# ADR-002: MPL-2.0 transitive dependencies from the Tauri foundation

Status: **ACCEPTED — human decision recorded**
Date: 2026-09-10 (proposed) / decided 2026-09-14
Candidate: `75fc35c` / `x86_64-pc-windows-msvc`

## Decision

The project owner **permitted MPL-2.0** in distributable core for the crates
reviewed below (`MPL-2.0` added to `deny.toml` `licenses.allow`). This closes the
AGENTS.md §5(c) STOP condition that kept the clause open. The decision is
recorded here, in `DECISIONS.md`, and in `LICENSE_ALLOWLIST.md` under change
control per AGENTS.md §3.

Five crates require the permission (`cssparser`, `cssparser-macros`,
`dtoa-short`, `selectors`, `option-ext`); a sixth, `htmlescape`, merely offers
MPL-2.0 among permissive alternatives and never needed it — see the correction
below.

Matching obligations accepted with the decision: the MPL-2.0 notice must ship in
the third-party notices artifact. `scripts/generate-sbom.py` emits CycloneDX 1.5
plus `.agent/evidence/sbom/THIRD_PARTY_NOTICES.md`, which carries every MPL
crate with its resolved license; the notices are covered by the SBOM gate
(`generate-sbom.py --check`) so a dependency change cannot silently drop them.

## Correction made during the review

The proposal listed **five** MPL-2.0 crates and asserted "all five are
transitive only... none was chosen by this project." Verification at the
deciding revision confirmed the five that fail the gate, and additionally found
a **sixth** crate that *offers* MPL-2.0 but does not require it:

| Crate | Version | Declared license | Reached via | Failed the gate? |
| --- | --- | --- | --- | --- |
| `cssparser` | 0.36.0 | `MPL-2.0` | `dom_query` → `tauri-utils` → `tauri`, and `selectors` | yes |
| `cssparser-macros` | 0.6.1 | `MPL-2.0` | `cssparser` | yes |
| `dtoa-short` | 0.3.5 | `MPL-2.0` | `cssparser` | yes |
| `selectors` | 0.36.1 | `MPL-2.0` | `dom_query` → `tauri-utils` | yes |
| `option-ext` | 0.2.0 | `MPL-2.0` | `dirs-sys` → `dirs` → `tauri-build` (build-dependency) | yes |
| `htmlescape` | 0.3.1 | `Apache-2.0 / MIT / MPL-2.0` | `tantivy` → `storage` (this project's own crate) | **no** |

`htmlescape` is a **choice expression** that also offers Apache-2.0 and MIT, so
the gate passed it on an allowlisted alternative and it was never one of the five
errors. That matters for two reasons: it means the MPL-2.0 *obligation* need not
attach to it at all (a permissive alternative may be elected), and it means the
original finding's scope statement was still wrong — the MPL surface was not
confined to the five Tauri crates, because `htmlescape` sits under **tantivy**,
a foundation this project's own `storage` crate depends on directly.

An intermediate revision of this ADR mis-described `htmlescape` as a sixth
gate-failing crate. That was an overstatement, corrected here: it is a sixth
*MPL-2.0-declaring* crate, not a sixth *MPL-2.0-requiring* one. The permission
covers it either way, but the distinction is recorded rather than flattened.

Measured with:

```text
cargo metadata --format-version 1 --locked   # htmlescape license = 'Apache-2.0 / MIT / MPL-2.0'
cargo tree --locked --target x86_64-pc-windows-msvc -i htmlescape
htmlescape v0.3.1
└── tantivy v0.26.2
    └── storage v0.1.0 (crates/storage)
```

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

## Current gate behaviour (was deliberate, now resolved)

At proposal time `deny.toml` did **not** list MPL-2.0 in `allow` and did not
exception these crates, so `cargo deny check licenses` reported
`licenses FAILED` (exit 4) with exactly these crates. That was intentional then:
adding them without the review this ADR requested would have been the silent
policy weakening DOD-021 and AGENTS.md §6 forbid.

Following the decision, `MPL-2.0` is in `licenses.allow` with a comment citing
this ADR, `LICENSE_ALLOWLIST.md` records the change, and
`cargo deny check licenses` passes. `advisories`, `bans` and `sources` were
already green.

## Impact of the decision

`scripts/security-check.sh`, `scripts/dependency-audit.sh` and
`scripts/verify.sh` — all three previously red on this single check — return
exit 0. DOD-021 moves off FAIL. EP-009 M2 (SBOM/notices) is no longer blocked on
the licence question, though signing remains a documented limitation (ADR-005).

## Residual obligation (accepted with the decision)

MPL-2.0 is file-level weak copyleft: the obligation attaches to modified
MPL-covered files. None of the six is modified by this project — they are used
as published — and the notice obligation is discharged through the shipped
third-party notices. If any of these crates is ever vendored or patched, this ADR
must be revisited, because modification changes the obligation.

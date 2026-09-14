# ADR-004: Add Playwright as a devDependency for real DOM E2E

- **Date**: 2026-09-10
- **Status**: ACCEPTED
- **Requirement IDs**: REQ-UI-001, REQ-UI-004, REQ-A11Y-001, REQ-SHIP-001
- **Evidence**: `.agent/evidence/ADR-004-playwright-dependency.md`

## Context

`scripts/test-e2e.sh` invokes `pnpm -r test:e2e`, but **no package declares a
`test:e2e` script**, so the gate failed with
`ERR_PNPM_RECURSIVE_RUN_NO_SCRIPT` (exit 1). Separately, `scripts/lint.sh`
revealed that the three JS packages contained **zero test files** (DOD-006/007
finding). The desktop UI — `apps/desktop/src/App.tsx` and `main.tsx`, the only
user-facing surface in the product — had no automated verification at all.

AGENTS.md §10 requires that a dependency be added only when the capability
cannot be built safely with existing code, with pinned versions, lockfile
update, license/security checks, and recorded rationale. This ADR is that
record.

## Decision

Add `@playwright/test` as a **devDependency** of `apps/desktop` only, pinned
exactly, and use it to drive the built desktop UI in a real browser.

## Why existing code cannot do this

- **Rust tests cannot reach the DOM.** The `linchpin-desktop` crate can assert
  that the process launches and that a window exists (already done in
  `apps/desktop/src-tauri/src/lib.rs` and proven in EP-009/M3), but it cannot
  assert what the UI *renders*.
- **Vitest without a DOM environment cannot either.** Vitest needs `jsdom` or
  `happy-dom` to render React, which is another dependency with the same
  paperwork. Playwright runs a **real browser engine**, which is strictly
  stronger evidence: it exercises the actual rendering path the user sees.
- **A hand-rolled DOM assertion would be a fabricated substitute** — the exact
  DOD-010/AG-005 anti-pattern already found and repaired five times in this
  repository.

## License and supply chain

| Field | Value |
| --- | --- |
| Package | `@playwright/test` |
| SPDX | Apache-2.0 |
| Status | **devDependency — not shipped** |
| Redistributed | No. Dev-only; excluded from the MSI/NSIS bundle. |
| Policy fit | Apache-2.0 is on the `LICENSE_ALLOWLIST.md` default allowlist. |
| Foundation status | `LICENSE_ALLOWLIST.md` names Playwright a "candidate foundation ... accepted only after exact-version and transitive dependency audit" — this ADR is that audit gate. |

Because the package is dev-only and never enters the distributable artifact,
the redistributed-core allowlist question does not arise for it. Version is
pinned exactly; no caret range, per AGENTS.md §10 ("Pin exact versions").

## Browser provisioning

Playwright's bundled browser download is **not** used. The repository pins the
`channel: "msedge"` / `"chrome"` system browsers, both of which are present on
the Windows target (`C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`,
`C:\Program Files\Google\Chrome\Application\chrome.exe`). This avoids a
~500 MB download in CI and avoids re-distributing a browser build.

If neither system browser is present the E2E gate must **fail loudly** rather
than silently skip — a skipped E2E suite is a DOD-006 blind spot.

## Alternatives considered

1. **`jsdom` + `@testing-library/react`.** Two dependencies instead of one,
   and a simulated DOM rather than a real engine — weaker evidence for a
   desktop app whose entire risk surface is what WebView2 renders.
2. **Rust-side WebView2 automation.** No maintained in-tree path; would require
   a larger dependency and platform-specific plumbing.
3. **Leave E2E red.** Honest but leaves the only user-facing surface unverified
   and blocks `test-e2e.sh` indefinitely.
4. **This ADR.** Chosen.

## Consequences

- `scripts/test-e2e.sh` gains a real target instead of failing on a missing
  script.
- The E2E suite becomes the first automated evidence for REQ-UI-001 (navigation)
  and REQ-UI-003 (legal-risk language), both of which currently map to zero
  executed tests.
- CI cost: a browser launch per run. Mitigated by using system browsers.
- `pnpm-lock.yaml` changes; must be committed with this ADR.

## Invalidation

Per DOD-040, adding a devDependency invalidates prior `pnpm`-based results.
Affected: lint, format-check, typecheck, and the JS lane of `test-unit.sh`.
All are rerun in the same session and recorded.

## Security/privacy terms impact

None on production code. The dependency is dev-only, never bundled, and never
executes in the shipped artifact. No provider terms, OAuth scope, or egress
policy is affected. E2E runs against a locally built bundle on loopback only;
no external network, no third-party system, no production data (E2E switch
library governing correction 3).

## Honest limitation

Playwright gives real DOM and accessibility-tree assertions. It does **not**
satisfy DOD-039 (manual assistive-technology validation by named humans), which
remains EXTERNAL_REQUIRED and may not be impersonated by tooling. It also does
not replace clean-room artifact testing (DOD-034).

## Verification

```
pnpm --filter @linchpin/desktop test:e2e     # must exit 0 with >= 1 test
sh scripts/test-e2e.sh                        # must exit 0
sh scripts/lint.sh                            # must stay exit 0
```

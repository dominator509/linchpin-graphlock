# GitHub repository security and dependency automation — verified record

Measured on `dominator509/linchpin-graphlock` (PUBLIC, default branch `main`) at
commit `42c8ff5`, 2026‑09‑21. Every claim below was read back from the API after the
change; no setting is reported as on because a write call returned success. Re-verify
with the commands in each section.

## 1. Account and repository

```
gh auth status
  ✓ Logged in to github.com account dominator509 (keyring)
  - Token scopes: 'gist', 'read:org', 'repo', 'workflow'
gh repo view --json visibility,defaultBranchRef
  {"visibility":"PUBLIC","defaultBranchRef":{"name":"main"}}
```

`workflow` is present, which is what allows pushing `.github/workflows/`.

## 2. Settings: enabled and read back

| Setting | Verification command | Observed after change |
| --- | --- | --- |
| Dependabot alerts | `gh api repos/{o}/{r}/vulnerability-alerts -i` | `HTTP/2.0 204 No Content` (404 before) |
| Dependabot security updates | `gh api repos/{o}/{r} --jq .security_and_analysis.dependabot_security_updates.status` | `enabled` (was `disabled`) |
| Secret scanning | same read-back, `.secret_scanning.status` | `enabled` (was `disabled`) |
| Secret scanning push protection | same read-back, `.secret_scanning_push_protection.status` | `enabled` (was `disabled`) |
| Private vulnerability reporting | `gh api repos/{o}/{r}/private-vulnerability-reporting` | `{"enabled":true}` (was `false`) |
| CodeQL default setup | `gh api repos/{o}/{r}/code-scanning/default-setup` | `{"state":"configured"}` (was `not-configured`) |

Secret scanning is *functioning*, not merely flagged: `gh api
repos/{o}/{r}/secret-scanning/alerts` returns `HTTP/2.0 200 OK` (it 404s when the
feature is off).

## 3. Settings that accepted the call and silently did NOT change

Both were attempted twice, once together and once individually. The PATCH returned
the repository object with HTTP 200 and no error, and the state did not move:

| Setting | Read-back after the write |
| --- | --- |
| Secret scanning non-provider patterns | `secret_scanning_non_provider_patterns: disabled` |
| Secret scanning validity checks | `secret_scanning_validity_checks: disabled` (the response body itself echoed `{"status":"disabled"}`) |

This is the documented trap: **HTTP 200 does not mean enabled.** Neither is claimed
here as on. Enabling them appears to require the repository's Security settings UI,
which an API client cannot click; they are recorded as NOT enabled.

## 4. CodeQL

Default setup is configured and has produced three analyses, all successful:

| Language | Results | Rules |
| --- | --- | --- |
| `actions` | 0 | 17 |
| `javascript-typescript` | 2 | 87 |
| `python` | 2 | 43 |

Run `35566451761` (push to `main`) completed `success`; `gh api
repos/{o}/{r}/code-scanning/analyses` lists the analysis records.

**Rust is not covered, and cannot be.** The repository's detected languages include
`rust`, but configuring default setup for it is rejected outright:

```
gh api -X PATCH repos/{o}/{r}/code-scanning/default-setup -f state=configured -f 'languages[]=rust'
HTTP 422: Invalid property /languages/0: `rust` is not a possible value.
Must be one of the following: actions, c-cpp, csharp, go, java-kotlin,
javascript-typescript, python, ruby, swift.
```

That list is CodeQL's supported set, so advanced setup does not help either: there is
no CodeQL extractor for Rust. The Rust half of this product is analysed by the
repository's own lanes (`cargo clippy` with warnings denied, `cargo deny`, the
weak-crypto scan, code metrics, mutation proofs) and by Dependabot advisories — not by
CodeQL. Stated plainly rather than implied.

### The four findings CodeQL raised, and what happened to them

All four were **fixed in code**, then verified `fixed` by re-reading the alert state
after the next scan (`gh api repos/{o}/{r}/code-scanning/alerts`):

| Alert | Severity | Location | Fix |
| --- | --- | --- | --- |
| `py/clear-text-logging-sensitive-data` | error | `scripts/architecture-drift.py:513` | URL userinfo is redacted before any URL is written, and the CLI prints a classified summary plus the report path instead of raw finding text — a URL can carry a credential into a committed file |
| `py/incomplete-url-substring-sanitization` | warning | `scripts/architecture-drift.py:391` | the self-test now asserts on the fixture path plus an exact-host word-boundary pattern instead of a bare URL substring |
| `js/incomplete-url-substring-sanitization` | warning | `apps/desktop/e2e-artifact.mjs:125` | the embedded-frontend assertion parses the URL and compares protocol and hostname exactly; `startsWith("http://tauri.localhost")` also accepted `http://tauri.localhost.attacker.example` |
| `js/incomplete-sanitization` | warning | `apps/desktop/e2e-local-provider.mjs:529` | markdown table cells now escape backslashes first, then pipes, then line breaks; before, a preceding backslash neutralised the escape and a newline broke the row |

Read-back after the fix: `0` open code scanning alerts, all four in state `fixed`.
The redaction fix was proven with a planted violation: the gate still exits 1 and
prints `architecture-drift: DRIFT -- 1 finding(s) in: local_confidentiality`, stderr
contains neither the planted credential nor the URL, and the committed report names
the offending file with the credential removed.

## 5. Dependabot

`.github/dependabot.yml` declares exactly the three ecosystems this tree has, one
entry each because each has exactly one lockfile at the root:

| Ecosystem | Directory | Lockfile |
| --- | --- | --- |
| `cargo` | `/` | `Cargo.lock` |
| `npm` (pnpm) | `/` | `pnpm-lock.yaml` |
| `github-actions` | `/` | `.github/workflows/` |

No entry exists for pip/uv, go, bundler or docker: this tree has no `pyproject.toml`,
`requirements*.txt`, `go.mod`, `Gemfile` or `Dockerfile`. Groups batch only
`minor` + `patch`; majors stay ungrouped.

Observed activity within minutes of the file landing on `main` (no waiting for the
weekly schedule — these are the security-update and version-update runs):

```
gh run list --workflow 'Dependabot Updates'
  completed success 5 · completed failure 2 · in_progress 2
gh pr list --state open
  #3  actions/checkout 6.1.0 -> 7.0.1          (major, ungrouped)
  #4  the cargo-minor-and-patch group (uuid 1.26.0 -> 1.26.1)   <- group works
  #5  sha2 0.10.9 -> 0.11.0                    (major, ungrouped)
  #6  windows 0.61.3 -> 0.62.2                 (major, ungrouped)
  #7  vitest 3.2.7 -> 4.1.11                   (security update for the open alerts)
  #8  the npm-minor-and-patch group with 6 updates
  #9  typescript 5.9.3 -> 6.0.3                (major, ungrouped)
  #10 vite 6.4.3 -> 8.3.0                      (major, ungrouped)
  #11 vitest 3.2.7 -> 5.0.1                    (major, ungrouped)
  #12 eslint 9.39.5 -> 10.10.0                 (major, ungrouped)
```

The grouping policy is visibly working: the minor/patch group arrived as one PR
(`#4`, `#8`) while every major arrived on its own (`#5`, `#6`, `#9`, `#10`, `#11`,
`#12`). `vitest` legitimately appears twice — `#7` is the security fix to the first
patched version and `#11` is the latest major — which is the expected Dependabot
behaviour, not a duplicate. Both land in the same lockfile, so they must not be
merged blindly in either order.

### Two update runs failed, with the exact reason

`gh run view <id> --log-failed` on runs `35566132168` (`glib`) and `35566127640`
(`@vitest/mocker`):

```
security_update_not_possible {
  "dependency-name": "@vitest/mocker",
  "latest-resolvable-version": "3.2.7",
  "lowest-non-vulnerable-version": "4.1.11",
  "conflicting-dependencies": []
}
security_update_not_possible {
  "dependency-name": "glib",
  "latest-resolvable-version": "0.18.5",
  "lowest-non-vulnerable-version": "0.20.0",
  "conflicting-dependencies": []
}
```

These are *reported* failures, not silent ones: the container exits non-zero and the
reason is recorded. `@vitest/mocker` is pinned exactly by `vitest@3.2.7`, so it cannot
be moved on its own — the fix is the `vitest` bump in `#7`. `glib` is pinned by the
GTK 0.18 Linux stack and moving it means the GTK 0.20 line.

## 6. Open Dependabot alerts and their reachability

`gh api repos/{o}/{r}/dependabot/alerts?state=open` — four alerts, all `medium`:

| # | Package | Manifest | Scope | Vulnerable | First patched | Reachable in what ships? |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `rust glib` | `Cargo.lock` | runtime | `>= 0.15.0, < 0.20.0` | 0.20.0 | **No** — see below |
| 2 | `npm vitest` | `package.json` | development | `>= 2.1.0, < 4.1.11` | 4.1.11 | **No** — devDependency |
| 3 | `npm @vitest/mocker` | `pnpm-lock.yaml` | development | `>= 2.1.0, < 4.1.11` | 4.1.11 | **No** — devDependency |
| 4 | `npm vitest` | `pnpm-lock.yaml` | development | `>= 2.1.0, < 4.1.11` | 4.1.11 | **No** — devDependency |

Reachability evidence, not assertion:

* **glib** — `cargo tree -i glib --target x86_64-pc-windows-msvc` prints
  `warning: nothing to print`, i.e. glib is not in the dependency graph of the target
  this product ships. `cargo tree -i glib --target all` shows the path:
  `glib <- atk <- gtk <- {muda, tao, tauri-runtime, webkit2gtk, wry} <- tauri <-
  linchpin-desktop` — the GTK/WebKit Linux desktop stack. ADR-004 scopes the supported
  clean-room matrix to Windows 10+, so no shipped artifact contains it. It stays in
  `Cargo.lock` (hence the alert) and would matter if this project ever built a Linux
  desktop artifact.
* **vitest / @vitest/mocker** — the root `package.json` has `"dependencies": {}` and
  lists `vitest` under `devDependencies`. No test runner is packaged into the MSI, the
  NSIS setup or the executable, which is also what the repository's own advisory
  register (`.agent/verification/state/ADVISORY_ASSESSMENTS.json`) recorded
  independently for the same GHSA.

Neither claim is stronger than the evidence: glib is *unreachable on the shipping
target*, and the vitest findings are *dev-only*. No alert is dismissed on GitHub; all
four remain open and visible.

## 7. Dependency review

`.github/workflows/dependency-review.yml` runs on `pull_request` targeting `main`
only, with `contents: read` and `pull-requests: read` and no PR comment (that would
need `pull-requests: write`).

* **Actions are pinned to versions verified to exist.** `refs/tags/v5` does **not**
  exist for `actions/dependency-review-action` (`gh api .../git/ref/tags/v5` → 404;
  `matching-refs/tags/v5` lists only `refs/tags/v5.0.0`), so upstream's documented
  `@v5` would fail to resolve at run time. Pinned:
  `actions/dependency-review-action@v5.0.0` (`a1d282b3…`) and
  `actions/checkout@v6.1.0` (`d23441a4…`).
* **Inputs were read from the action's own `action.yml` and README at that tag**, not
  from prose: `fail-on-severity` has no default, so it is set to `high`;
  `fail-on-scopes` defaults to `runtime` and is widened to `runtime, development` to
  match what `scripts/security-check.sh` already enforces via
  `pnpm audit --audit-level high`; `license-check` defaults to `true` and is left on
  with `allow-licenses` transcribed from `LICENSE_ALLOWLIST.md` plus the MPL-2.0
  permission recorded in ADR-002.
* **Trigger proven on real PRs, not synthesised:** `gh run list --workflow
  'Dependency Review'` shows 10 runs, all `completed / success`, one per Dependabot
  PR. The step log echoes the configured inputs and does real work:

```
with:
  fail-on-severity: high
  fail-on-scopes: runtime, development
  license-check: true
  vulnerability-check: true
  allow-licenses: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, MPL-2.0
Vulnerabilities: Dependency review did not detect any vulnerable packages with severity level "high" or higher.
Licenses: (none)
Denied: Dependency review did not detect any denied packages
Dependency Changes
File: Cargo.lock
+ uuid@1.26.1
- uuid@1.26.0
```

* **The prerequisite is satisfied**: the dependency graph is live — `gh api
  repos/{o}/{r}/dependency-graph/sbom` returned `404` before the graph populated and
  returns package data now, which is what the review diffs.
* **NOT exercised:** the failure path. No PR has introduced a high/critical
  vulnerability or a non-allowlisted licence, so the action has never exited non-zero
  here. Claiming "it blocks bad dependencies" would be a claim about a path this
  repository has not run.

## 8. Interdependent upgrades and merge order

Nothing here has been merged. Major bumps interact, and two of them touch the same
lockfile, so the safe order is:

1. **`#7` vitest 3.2.7 → 4.1.11** — the security fix for the two npm alerts. It is
   the smallest change that closes them, and it needs vite ≥ 6, which the tree already
   has (6.4.3).
2. **`#4`, `#8`** — the cargo and npm minor/patch groups, independent of the majors.
3. **`#10` vite 8.3.0 before `#11` vitest 5.0.1** — vitest majors track vite majors;
   taking `#11` first is the peer-incompatibility shape that produced
   `ERR_PACKAGE_PATH_NOT_EXPORTED: './module-runner'` in the past.
4. **`#9` typescript 6.0.3 with `#12` eslint 10.10.0** — the TypeScript major and the
   lint major must move together with `typescript-eslint`; separately either can break
   the other.
5. **`#3` actions/checkout 7.0.1** — updates the pin in
   `.github/workflows/dependency-review.yml`; independent of everything else.
6. **`#5` sha2 0.11.0** and **`#6` windows 0.62.2** — application-code majors: both
   are used by first-party crates (`storage`, `platform_windows`), so they need a
   compile and test pass, not just a review.

**Which majors has CI already failed on? None — because no workflow builds or tests
this repository.** Dependency review checks vulnerabilities and licences; CodeQL
checks code patterns; neither compiles anything. A green check on `#5`, `#6`, `#9`,
`#10`, `#11` or `#12` therefore means "no known-vulnerable or non-allowlisted
dependency was introduced", not "this builds". That gap is real and is not papered
over here.

## 9. Free-on-public: confirmed, denied, and what a private switch would cost

| Capability | State here |
| --- | --- |
| Unlimited Actions minutes on standard runners | available (public) — 10 dependency-review runs and 3 CodeQL analyses executed today |
| CodeQL code scanning | ON — `actions`, `javascript-typescript`, `python`; **Rust impossible** |
| Secret scanning | ON and functional (`/secret-scanning/alerts` → 200) |
| Push protection | ON |
| Non-provider patterns / validity checks | **NOT enabled** — API accepts, state stays `disabled` |
| Dependabot alerts | ON (204) |
| Dependabot security updates | ON — produced `#7` for the vitest advisory |
| Dependabot version updates | ON — 3 ecosystems, 10 PRs within minutes of landing |
| Private vulnerability reporting | ON (`{"enabled":true}`) |
| Dependency review | ON — 10 runs, trigger and diff proven |

**If this repository goes private** (all of the above is free only while public):
Actions minutes become metered against the account's plan; CodeQL code scanning
requires a GitHub Code Security licence; secret scanning and push protection require
GitHub Secret Protection; Dependabot alerts and version updates remain free, and
private vulnerability reporting remains available. The three verified capabilities
that would lapse immediately are therefore unlimited minutes, CodeQL and secret
scanning with push protection. Nothing in this document should be read as a promise
that these keep running after a visibility change.

## 10. How to re-verify

```
gh auth status
gh repo view --json visibility,defaultBranchRef
gh api repos/dominator509/linchpin-graphlock --jq .security_and_analysis
gh api repos/dominator509/linchpin-graphlock/vulnerability-alerts -i
gh api repos/dominator509/linchpin-graphlock/automated-security-fixes
gh api repos/dominator509/linchpin-graphlock/private-vulnerability-reporting
gh api repos/dominator509/linchpin-graphlock/code-scanning/default-setup
gh api "repos/dominator509/linchpin-graphlock/code-scanning/alerts?state=open"
gh api "repos/dominator509/linchpin-graphlock/dependabot/alerts?state=open&per_page=100"
gh run list --workflow 'Dependency Review' --limit 20
gh run list --workflow 'Dependabot Updates' --limit 20
gh run list --workflow 'CodeQL' --limit 10
```

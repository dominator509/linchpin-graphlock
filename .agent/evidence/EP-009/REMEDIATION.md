# EP-009 — Remediation record: the recorded blocker was false

Status: **RE-OPENED for remediation** (previously `CLOSED_BLOCKED`)
Base: `962e365` · Remediation candidate: working tree at the time of writing
Node: EP-009 (Deployment and Release) · Requirements: REQ-REL-001, REQ-REL-005

## The defect being remediated is in the ledger, not in the product

`.agent/state/LEDGER.md` records:

```
| 2026-09-09 | EP-009 | CLOSED_BLOCKED | EP-009-attempt-001 | missing windows environment |
```

That reason is **factually contradicted by EP-009's own committed evidence**, and
it has kept `scripts/graph-next.sh` returning
`RUN_BLOCKED dependencies prevent: EP-010` on every run since, because EP-010
depends on EP-009 and a `CLOSED_BLOCKED` node is never eligible.

The blocker describes the environment the blueprint pack was *authored* in (a
Linux sandbox), not the environment the work runs in. It was inherited into the
ledger as an unverified premise and re-read as fact thereafter. This is the same
stale-disposition failure mode already corrected for DOD-002 (a clean-environment
build was recorded PARTIAL on the false assumption a VM was required), DOD-041
and DOD-042.

## Measurement disproving the blocker

Host, probed directly:

```text
[Environment]::OSVersion.VersionString  ->  Microsoft Windows NT 10.0.19045.0
target triple                           ->  x86_64-pc-windows-msvc
```

Pre-existing in-tree evidence that already recorded Windows execution:

- `.agent/evidence/EP-009/M1/STATUS.md` — "Host: Windows 10 Home 10.0.19045, x64",
  target `x86_64-pc-windows-msvc`, with the RC2175 icon-format defect reproduced
  and repaired on that host.
- `.agent/evidence/EP-009/M3/STATUS.md` — "a real install of the real installer on
  the real host — not a source-tree test and not a mock", with a pinned MSI
  SHA-256.
- `.agent/evidence/EP-009/M2/` — real `msi-install.log` (107,642 B) and
  `msi-uninstall.log` (102,066 B) from actual `msiexec` runs, plus the MSI and
  NSIS bundles and the SUP-004 reproducible-build comparison.

A node cannot have produced `msiexec` install/uninstall logs on Windows while
being blocked for a missing Windows environment.

## Re-verification at the current candidate

Re-run rather than inherited, because DOD-040 requires that a changed candidate
invalidates prior evidence.

### M1 — frozen Windows installer artifacts

```text
sh scripts/build.sh                                        -> exit 0
target/release/bundle/msi/LINCHPIN_0.1.0_x64_en-US.msi     -> 5,558,272 bytes
target/release/bundle/nsis/LINCHPIN_0.1.0_x64-setup.exe    -> 3,121,331 bytes
```

Both supported formats are produced from one candidate by one command.

### M3 — exact-artifact install / launch / uninstall

```text
sh scripts/smoke-test.sh                                   -> exit 0
smoke: sha256 = 7f58678ec487b4b356629645f3f8449577e086f8ad9cc307feae927bb5ba4baf
smoke: install exit 0
smoke: reading back installed state
smoke: installed exe = 12036096 bytes
smoke: ARP name=LINCHPIN version=0.1.0 publisher=linchpin
smoke: window title = 'LINCHPIN Patent Intelligence OS' responding = True
smoke: launched and stopped cleanly
smoke: uninstalling
smoke: verifying removal
smoke: clean removal confirmed
```

This is a real silent install of the real MSI on this host, an independent
read-back of the installed state through the Add/Remove Programs registry (not
through the installer's own exit code), a real process launch whose window title
and responsiveness were observed, and a clean uninstall with removal verified.

Lane 2 of `scripts/test-e2e.sh` additionally drives the **packaged executable's**
WebView2 over CDP and passes 15 assertions against a pinned digest (DOD-004),
including that the loaded origin is the embedded `http://tauri.localhost/` rather
than a development server.

### M2 / M5 — SBOM, notices and candidate freeze

Merged cross-ecosystem SBOM: CycloneDX 1.5, 720 components, `generate-sbom.py
--check` green, notices carrying both license inventories and the artifact and
lockfile digests. The MPL-2.0 licence question that previously gated M2 is
resolved by ADR-002, and `scripts/dependency-audit.sh` now exits 0.

## What is resolved by decision rather than by execution

Two EP-009 milestones depend on signing, and PF-015 is unavailable (probed: no
code-signing certificate in either the CurrentUser or LocalMachine store). The
project owner accepted an unsigned release as a documented limitation (ADR-003):

- **M2** — SBOM/notices are produced; signing and provenance attestation are
  `not applicable by decision`, disclosed in `RELEASE.md` and `DEPLOYMENT.md`.
- **M4** — signed update/rollback is **NOT** executed and is **not** claimed. The
  unsigned limitation covers the absence of a signature; it does **not** convert
  M4 into evidence. M4 stays open and is reported as such.

M4 is deliberately left open rather than folded into the unsigned decision:
"we cannot sign it" is not "rollback was tested".

## External prerequisites still open for this node

- **PF-016** zero-state/virgin target for DOD-034. Scope narrowed to Windows 10 or
  higher by ADR-004 (Windows 11 not claimed), but no virgin host exists, so
  DOD-034 remains `EXTERNAL_REQUIRED`.
- **PF-017 / PF-018** human sign-off gates, confirmed `EXTERNAL_REQUIRED` by
  ADR-005.

Neither is a reason to keep EP-009 eligible-but-blocked on a false premise.

## Ledger action

Append an `EP-009-remediation-001` event recording that the environment blocker
is disproven with the measurements above, superseding `EP-009-attempt-001`, and
place EP-009 back in the runnable graph so EP-010 becomes reachable. Per
AGENTS.md §4, `CLOSED_BLOCKED` is a scheduling state and never success; leaving it
in place while the blocking condition no longer holds corrupts the scheduling
accounting for every dependent node.

# COMMANDS - only legal command source

## Working directory
Run commands from repository root. Export: `CI=true GIT_TERMINAL_PROMPT=0 GIT_PAGER=cat PAGER=cat CARGO_TERM_COLOR=never`.

| Action | Command |
| --- | --- |
| install | `sh scripts/install.sh` |
| preflight | `sh scripts/preflight.sh` |
| lint | `sh scripts/lint.sh` |
| format check | `sh scripts/format-check.sh` |
| typecheck | `sh scripts/typecheck.sh` |
| unit | `sh scripts/test-unit.sh` |
| integration | `sh scripts/test-integration.sh` |
| e2e | `sh scripts/test-e2e.sh` |
| build | `sh scripts/build.sh` |
| security | `sh scripts/security-check.sh` |
| dependency audit | `sh scripts/dependency-audit.sh` |
| smoke | `sh scripts/smoke-test.sh` |
| live fire | `sh scripts/live-fire.sh` |
| verify | `sh scripts/verify.sh` |
| production readiness | `sh scripts/production-readiness-check.sh` |
| validate blueprint | `python3 scripts/validate-generated-pack.py .` |
| anti-gaming scan | `python3 scripts/anti-gaming-scan.py .` |
| graph next | `sh scripts/graph-next.sh` |
| ledger tail | `sh scripts/ledger.sh tail 30` |
| materialize security sources | `sh scripts/materialize-atomic-sources.sh` |
| stage accounting (EP-010 M1) | `python3 scripts/stage-accounting.py --check` |
| architecture drift (EP-010 M3) | `python3 scripts/architecture-drift.py` |
| architecture drift self-test | `python3 scripts/architecture-drift.py --self-test` |
| residual risk / readiness derivation (EP-010 M4/M5) | `python3 scripts/residual-risk-report.py --check` |
| version-matrix source currency (DOD-040) | `python3 scripts/version-matrix.py --check-source` |

### Verification accounting commands

Added by EP-010 so that every gate the closure depends on is published here rather
than living only inside `scripts/verify.sh`. Each is a lane of `verify.sh`:

* `scripts/stage-accounting.py` derives a per-stage record for V-000..V-021 from the
  pack's own `default_stage` ownership and the per-ID accounting. It writes
  `.agent/verification/reports/STAGE_ACCOUNTING.md`, `state/STAGE_ACCOUNTING.json` and
  the harness cursor `state/RUN_STATE.json`; `--check` fails when the record is stale
  against the registry, the applicability matrix or the accounting.
* `scripts/architecture-drift.py` implements the nine-question review of
  `ARCHITECTURE.md` section 145. `--self-test` plants one violation per mechanical
  rule and requires each to be caught; `--check` fails when the record is stale
  against `ARCHITECTURE.md`, the scope module or the crate manifests.
* `scripts/residual-risk-report.py` derives `RESIDUAL_RISK_AND_EXTERNAL_GATES.md`,
  `FINAL_PRODUCTION_READINESS_REPORT.md`, `state/NEXT_ACTION.md`,
  `FINAL_SUBMIT_REPORT.md` and the EP-010 M5 manual release instructions from the
  executed state; `--check` fails when any of them stops matching it.
* `scripts/version-matrix.py --check-source` compares the fingerprint of the staged
  v0.1.0/v0.2.0 packages with the current build inputs and fails naming both revisions.

The **full** cross-version matrix run is deliberately not published as a runnable
command here: it builds both releases and installs/uninstalls them on the host. It is
lane 4 of `sh scripts/test-e2e.sh` and re-provisions itself when the source has moved.

## Known command status

Verified by `scripts/doc-exec.py`, which extracts and executes every command
published here and in the operator documents.

These commands are published but do **not** currently exit 0 when run as
written.

| Command | Exit | Cause | Unblock |
| --- | --- | --- | --- |
| `sh scripts/production-readiness-check.sh` | 1 | verdict is NO_GO (correct behavior, not a defect) | release readiness |

**Resolved since the previous revision of this table** (all now exit 0):
`sh scripts/security-check.sh`, `sh scripts/dependency-audit.sh` and
`sh scripts/verify.sh` were failing on the MPL-2.0 licences check — closed by
ADR-002. `sh scripts/live-fire.sh` was exiting 2 because
`scripts/run-live-fire-real.sh` did not exist; that real production-path runner
now exists and the gate passes. `pnpm audit` used to exit 1 on 2 moderate
advisories; both were **remediated rather than waived** — vitest was upgraded
3.2.7 → 4.1.11 (the first patched version, Dependabot security PR #7) and then
5.0.1 (PR #11), and the command now prints `No known vulnerabilities found` with
exit 0. The register entries in `.agent/verification/state/ADVISORY_ASSESSMENTS.json`
are marked RESOLVED with the remediation recorded, and GitHub marks the three
corresponding Dependabot alerts as `fixed`.

**Ordering caveat for `sh scripts/verify.sh`.** It ends with the change-
invalidation and rerun-obligation checks, so it legitimately FAILS if it is run
after something that changed a tracked input in the same sequence — including
`sh scripts/build.sh`, which `scripts/doc-exec.py` executes earlier in its list.
That is DOD-040 working, not a defect. Run `scripts/change-invalidation.py` and
`scripts/rerun-invalidated.py` to settle first, then `scripts/verify.sh` passes.

The E2E lane binds a fixed port (`4173`) with `--strictPort` and
`reuseExistingServer: false`, so a server squatting that port fails the lane
loudly rather than being silently reused. Observed during this audit: an
unrelated project's `vite preview` held 4173, the lane failed on a 404, and it
passed 14/14 once the port was free. Run the E2E lane on its own; do not run two
documentation or E2E invocations concurrently.

## Mutation proofs (DOD-018)

```sh
python3 scripts/mutation-proof.py            # run every declared mutation
python3 scripts/mutation-proof.py --list     # show what would run
python3 scripts/mutation-proof.py --only ID  # one mutation by id
python3 scripts/mutation-proof.py --check    # verify the recorded report
```

Each mutation is applied as an exact literal edit, the guarding test is run, and
the file is restored in a `finally` block and verified byte-for-byte before the
test is rerun green. Outcomes are classified, and only `CAUGHT` is a proof:

| Verdict | Meaning |
| --- | --- |
| `CAUGHT` | the mutant compiled, the named test failed, no compile error present |
| `SURVIVED` | the mutant compiled and the test PASSED — the test does not discriminate; harness exits 1 |
| `BUILD_ERROR` | the mutant did not compile, so a non-zero exit proves nothing; harness exits 1 |
| `TIMEOUT` | the mutant hung; harness exits 1 |

An anchor that does not match exactly once is a fatal error rather than a no-op
mutation, because a no-op would be reported as `SURVIVED` and misread as a weak
test. The report is written to `.agent/evidence/mutation-proof/REPORT.json`.

## Vault backup and recovery (REQ-REL-005)

Operator path in the product: **Settings → Durable state recovery** — set the
backup file, *Back up vault*, and *Restore vault from backup* (which requires a
consequence-specific confirmation, REQ-UI-004).

Command path: `backup_vault(workspace_id, destination)` and
`restore_vault(workspace_id, source)` over IPC, and in the harness:

```sh
cargo test -p linchpin-desktop --test recovery_drill -- --nocapture
```

The drill runs five fault-injected recovery cycles (total loss, in-place
corruption) and writes measured RPO/RTO/MTTR to
`.agent/evidence/recovery-drill/report.json`. Recovery semantics worth knowing
before using it:

* a backup is a **point-in-time snapshot**; work recorded after it is discarded
  by a restore, and the UI says so;
* a restore whose **source** does not exist is refused **before** the
  destination is touched, so a mistyped backup path cannot damage the vault;
* if the vault **file is gone**, the restore recreates it and reports
  `destination_recreated`;
* if the vault is **unreadable**, it is renamed aside (preserved, never deleted)
  and the path is reported as `destination_quarantined`; it is not repaired;
* reconciliation is by content digest, so a restore that does not reach the
  backup's digest is reported as `reconciled: false` rather than assumed to have
  worked.

## Abbreviated endurance trial (DOD-038)

```sh
LINCHPIN_SOAK_SECONDS=600 cargo test -p linchpin-desktop --test soak_abbreviated -- --nocapture
```

A bounded trial against the real write, read and **recovery** paths: every cycle
writes through `record_conception_keyed` under one client key (so a duplicate is
caught rather than accumulated), every 20th cycle reads the whole ledger back
through the product's own read path and checks the count, and every 50th cycle
backs the vault up, **destroys the file**, restores it and asserts reconciliation —
so recovery runs under load rather than only in a quiet drill. Resource use is
sampled by an external observer (PowerShell working set), not self-reported.

**The run labels itself.** Below 300 seconds it is reported as `SMOKE_SUBSET` and
does **not** write evidence, so the ordinary unit lane (default 30 s) can never
overwrite an abbreviated trial's report. At or above the floor it is labeled
`ABBREVIATED` and writes `.agent/evidence/soak/report.json` with heartbeats,
cycles, errors, vault growth, working-set growth and the epoch it ran against.

**The full-soak scale IS specified, and it is not met here.** The pack's suite
library — `.agent/verification/E2E_SUITE_LIBRARY.md`, which carries the source of
`E2E-SoakResourceLeakTesting.md` and is the registry's own `source_file` for
E2E-018 — requires **24, 48, 72+ hours** of sustained nominal load with continuous
telemetry and the flatline invariant, and explicitly forbids time-bounded runners:
it requires dedicated, persistent infrastructure. This host is a shared
workstation, so every run of this lane is an ABBREVIATED trial labeled separately,
and **DOD-038 is DEFERRED_LONG_RUNNING** rather than PASS. No numeric duration or
workload is specified for the fuzz, stress, performance or recovery terms, in the
repository or in the pack. (An earlier revision of this file claimed no scale was
specified anywhere; that was a measurement failure — it searched the repository's
own specs and not the suite library the registry points at.)

## Coverage gate (DOD-008)

```sh
python3 scripts/coverage-gate.py            # measure, enforce floors, write evidence
python3 scripts/coverage-gate.py --check    # fail if the epoch has moved
```

Instrumented line/function coverage for the eleven **library** crates, measured
with the llvm tools that ship **with the compiler** (`llvm-tools-preview`), so no
repository dependency is added. The desktop application crate is excluded by name
and is covered at the artifact boundary instead; the exclusion is printed in the
report rather than implied.

**Environment prerequisite**: `rustup component add llvm-tools-preview`. Without
it the gate exits 2 naming the component — an environment limitation, never a
product result. The host LLVM installation is a different patch version from the
LLVM rustc links and produced a table of `0.00%` from valid profiles, so the gate
refuses a 0% TOTAL instead of recording it as a measurement of zero.

The report records the **epoch digest** it was measured at, so a source change
invalidates the number; `verify.sh` enforces that with `--check`, and the settle
path (`scripts/rerun-invalidated.py`) re-measures it automatically.

## Live-fire (AGENTS.md section 9)

`sh scripts/live-fire.sh` now runs the real production-path proof suite
(`scripts/run-live-fire-real.sh`), which composes three independently runnable
gates: the packaged-desktop boundary over CDP with a real vault write and a
pinned artifact digest; the real local-provider boundary with both negative
cases; and installer state preservation across update/uninstall/rollback. It
**requires** a served loopback provider (PF-011) and exits 2 naming that
prerequisite when absent — a live-fire gate that quietly skipped its live
dependency would be the fabrication AG-006 recorded.

## Local provider prerequisite (PF-011)

The provider lane and every gate that composes it need a **running** loopback
inference server. The runtime is installed at
`%LOCALAPPDATA%\linchpin-local-runtime\ollama` with models under
`...\linchpin-local-runtime\models`, but it is not a background service: it stops
when the process does, and the gates then exit 2 naming PF-011 rather than
passing or skipping. Start it before running those gates:

```powershell
$dir = "$env:LOCALAPPDATA\linchpin-local-runtime"
$env:OLLAMA_HOST = "127.0.0.1:11434"          # loopback only, by design
$env:OLLAMA_MODELS = "$dir\models"
Start-Process -FilePath "$dir\ollama\ollama.exe" -ArgumentList "serve" -WindowStyle Hidden
```

Verify before relying on it — the first request after start can take several
seconds to warm up, so a short timeout can look like a failure:

```powershell
(Invoke-WebRequest http://127.0.0.1:11434/api/tags -TimeoutSec 20).Content.Contains("smollm2")
```

Observed during this run: the server had exited between sessions, `V-013` failed
with exit 2 and the message `PF-011 UNSATISFIED`, and the lane passed after the
server was restarted. That is the designed behaviour — the prerequisite is
reported, never assumed.

## Local start
After EP-005 creates the application: `pnpm --filter @linchpin/desktop tauri dev > .agent/state/dev-server.log 2>&1 & echo $! > .agent/state/dev-server.pid`; readiness is a bounded 60-second probe defined in EP-005; stop with `kill "$(cat .agent/state/dev-server.pid)"`.

## Database
Database creation/migration commands become executable only after EP-003 transcribes the pinned migration tool command into `scripts/install.sh`/`scripts/verify.sh`; no agent may invent a substitute command. Migrations run only on disposable test vaults in automation.

## Adapter parity
`for f in AGENTS.md CLAUDE.md GEMINI.md GROK.md .github/copilot-instructions.md .hermes/AGENTS.md; do awk '/PRIME-BLOCK-BEGIN/,/PRIME-BLOCK-END/' "$f" | cksum; done`
All checksums must be identical.

## Forbidden
Interactive REPLs/editors/pagers, foreground watch modes in automation, forced pushes, history rewrites, blanket test retries, destructive database commands outside migration/recovery plans, arbitrary shell generated by an LLM, and commands absent from this file.

Coding agents must not invent commands. If a command is missing or stale, update this file first, citing repository evidence, with a Decision Log entry.

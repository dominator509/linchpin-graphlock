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

## Known command status

Verified by `scripts/doc-exec.py`, which extracts and executes every command
published here and in the operator documents.

These commands are published but do **not** currently exit 0 when run as
written.

| Command | Exit | Cause | Unblock |
| --- | --- | --- | --- |
| `sh scripts/production-readiness-check.sh` | 1 | verdict is NO_GO (correct behavior, not a defect) | release readiness |
| `pnpm audit` | 1 | 2 moderate advisories (see below) | dependency upgrade decision |

**Resolved since the previous revision of this table** (all now exit 0):
`sh scripts/security-check.sh`, `sh scripts/dependency-audit.sh` and
`sh scripts/verify.sh` were failing on the MPL-2.0 licences check — closed by
ADR-002. `sh scripts/live-fire.sh` was exiting 2 because
`scripts/run-live-fire-real.sh` did not exist; that real production-path runner
now exists and the gate passes.

**Ordering caveat for `sh scripts/verify.sh`.** It ends with the change-
invalidation and rerun-obligation checks, so it legitimately FAILS if it is run
after something that changed a tracked input in the same sequence — including
`sh scripts/build.sh`, which `doc-exec.py` executes earlier in its list. That is
DOD-040 working, not a defect. Run `scripts/change-invalidation.py` and
`scripts/rerun-invalidated.py` to settle first, then `verify.sh` passes.

`pnpm audit` reports 2 moderate advisories (GHSA-82fw-gwwq-j7x9 in `vitest` and
`@vitest/mocker`, fixed in >= 4.1.11; this repository pins 3.2.7). The enforced
level in `scripts/security-check.sh` is `--audit-level high`, so these do not
fail the lane. The affected package is dev-only and is not embedded in any
shipped artifact. No waiver row exists for this advisory, so it is listed here
as an accepted, documented risk rather than a silent pass.

The E2E lane binds a fixed port (`4173`) with `--strictPort` and
`reuseExistingServer: false`, so a server squatting that port fails the lane
loudly rather than being silently reused. Observed during this audit: an
unrelated project's `vite preview` held 4173, the lane failed on a 404, and it
passed 14/14 once the port was free. Run the E2E lane on its own; do not run two
documentation or E2E invocations concurrently.

## Live-fire (AGENTS.md section 9)

`sh scripts/live-fire.sh` now runs the real production-path proof suite
(`scripts/run-live-fire-real.sh`), which composes three independently runnable
gates: the packaged-desktop boundary over CDP with a real vault write and a
pinned artifact digest; the real local-provider boundary with both negative
cases; and installer state preservation across update/uninstall/rollback. It
**requires** a served loopback provider (PF-011) and exits 2 naming that
prerequisite when absent — a live-fire gate that quietly skipped its live
dependency would be the fabrication AG-006 recorded.

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

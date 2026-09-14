# DOD-004 Artifact-Bound E2E — attempt record (NOT WORKING)

Status: **NOT FUNCTIONAL — do not wire into any gate.**

`scripts/artifact-e2e.sh` and `scripts/artifact-e2e-probe.py` were written this
round to satisfy DOD-004's requirement that E2E run "against the exact
production artifact digest, not merely source or a development server". The
existing Playwright suite runs against `vite preview`, which the clause
excludes, so the gap is real.

The attempt did **not** succeed. It is recorded here rather than left as a
half-wired gate, and it is deliberately **not** referenced by
`harness-validate.sh` or `test-e2e.sh`.

## What was established (useful findings)

1. **The DevTools endpoint requires opting in.** A plain release build never
   opens a debugging port. Tauri's `devtools` cargo feature exists
   (`tauri 2.11.5`), and `tauri.conf.json` supports a per-window
   `additionalBrowserArgs` key. Setting
   `additionalBrowserArgs: "--remote-debugging-port=9222"` in a config overlay
   **does** open the endpoint: verified `HTTP 200` on
   `http://127.0.0.1:9222/json` with one `page` target.
   Setting only `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` in the environment did
   **not** work.

2. **The port is open only briefly.** Measured by polling every 3s while the
   process stayed alive: listening at t=3s, closed by t=6s and every sample
   after. A 1-second poll loop misses it entirely. `artifact-e2e.sh` now polls
   every 250ms, which does observe a page target with a `webSocketDebuggerUrl`.

3. **This must never ship enabled.** A debugging port in a released binary lets
   any local process attach to the webview and invoke backend commands, which
   would break the SPEC-005 confidentiality boundary. Any working solution must
   be feature-gated and build a dedicated test artifact.

## What did not work

A hand-rolled RFC 6455 WebSocket client (standard library only, no new
dependency) completes the handshake, but `Runtime.evaluate` returns empty
results: `document.title` is `''`, `typeof window.__TAURI_INTERNALS__` is
`undefined`, and `document.body` is absent. Enabling the `Runtime` and `Page`
domains did not change this. The connection is therefore reaching a target that
is not the live page context, or the evaluation is not being dispatched
correctly.

A follow-up diagnostic that dumped raw `Runtime.evaluate` responses for `1+1`,
`document.title`, `location.href` and `document.body` **hung** and produced no
output, so the failure mode is not yet characterised.

## Not attempted

- Using Playwright's CDP support (`chromium.connectOverCDP`) instead of a
  hand-rolled client. This is the obvious next step and is not blocked by
  anything: `@playwright/test` is already a devDependency (ADR-004). It removes
  the hand-rolled WebSocket layer entirely, which is where the failure is.
- Installing the artifact and driving the installed copy (the current script
  launches the build output; `scripts/smoke-installed-artifact.sh` covers the
  install path separately).

## Honest status

DOD-004 remains **PARTIAL**. Nothing about this attempt changes its disposition,
and the two scripts are inert. The verified parts — that the endpoint needs an
explicit opt-in, that it is open only briefly, and that enabling it by default
would be a security regression — are recorded here so the next attempt does not
rediscover them.

# DOD-004 Artifact-Bound E2E — CLOSED (three attempts)

Status: **WORKING.** `scripts/artifact-e2e.sh` drives the packaged executable's
real WebView2 over CDP and 15 assertions pass against a pinned digest. Both E2E
lanes now run from `scripts/test-e2e.sh`.

This file is retained because the two failed attempts contain the findings that
made the third one work.

## The blocker was artifact production, not CDP

A raw `cargo build --release` embeds `devUrl` and makes the app load
`http://localhost:5173` — a development server, which DOD-004 excludes. Only an
artifact produced by **`tauri build`** embeds `frontendDist` and serves the app
from `http://tauri.localhost/`.

Measured difference, same source:

| Build | `index-n_ittevb.js` embedded | Loaded origin |
| --- | --- | --- |
| `cargo build --release` | no | `chrome-error://chromewebdata/` (localhost:5173 refused) |
| `tauri build --no-bundle` | yes | `http://tauri.localhost/` |

Two rounds were spent suspecting the CDP client. The CDP client was fine:
`chromium.connectOverCDP` attaches and `window.__TAURI_INTERNALS__` is present.

## Also required

- `devtools-e2e = ["tauri/devtools"]`, a non-default feature: without it no
  debug endpoint exists at all.
- A config carrying `additionalBrowserArgs: "--remote-debugging-port=<port>"`.
  Setting only `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` in the environment did
  **not** work.
- The port opens briefly at startup and closes once the webview finishes
  initialising (measured open at t=3s, closed by t=6s). Poll at 250ms or the
  window is missed.
- Wait for `domcontentloaded` / `readyState === "complete"` before evaluating:
  the webview starts at `about:blank` and navigating destroys the execution
  context mid-flight. That was the cause of the empty-evaluation symptom.

## Security regression found and fixed while wiring the gate

`tauri build` writes to `target/release/linchpin-desktop.exe`, the SAME path the
production artifact uses. An early version of the gate therefore **left the
devtools-enabled binary sitting at the production path** — a released binary
that opens a debug port, letting any local process attach to the webview and
invoke backend commands. That breaks the SPEC-005 confidentiality boundary.

The gate now stashes the production artifact, builds the test one, restores, and
**proves the restored binary keeps port 9222 closed** before running any
assertion. Verified: "production artifact correctly opens no debug port".

## What is verified

15 assertions against the pinned executable digest, including that the loaded
origin is the embedded frontend, that the Tauri IPC bridge exists, that
`get_system_health` and `get_namespace_status` round-trip, and that
`record_conception` — a state-changing command — writes a `sha256:` content
address into the durable vault with `HumanConception` origin preserved. The
digest is re-verified unchanged after the run.

## Remaining limitation

The executable is launched from build output rather than installed from the MSI;
`scripts/smoke-installed-artifact.sh` covers the install path. A virgin
clean-room install (DOD-034) remains EXTERNAL_REQUIRED.


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

## Round 23 — connectOverCDP tried; the WebSocket layer was not the problem

The hand-rolled RFC 6455 client was replaced with Playwright's
`chromium.connectOverCDP` (already a devDependency via ADR-004). This is a
**strict improvement and the WebSocket layer was NOT the failure**:

    targets: page:about:blank
    contexts: 1
    pages: 1
    page url: chrome-error://chromewebdata/
    title: "localhost"
    h1: "Hmmm… can't reach this page"
    hasIpc: true          <-- the REAL Tauri webview context IS reachable

`hasIpc: true` is the significant result. `window.__TAURI_INTERNALS__` is
present, so CDP is attached to the genuine webview context, not a stray target.
The earlier empty-evaluation failure was **a navigation race**: the webview
starts at `about:blank` and navigates, and evaluating before the new document
loads destroys the execution context. Waiting for `domcontentloaded`,
`readyState === "complete"` and an `h1` selector does not fix it here only
because the navigation now fails outright (below).

### The real blocker: the test artifact loads the DEV SERVER, which is not running

The page ends at `chrome-error://chromewebdata/` with "can't reach this page"
against **`localhost:5173`** — Tauri's `devUrl`. So the artifact under test tries
to load a development server, which is precisely the condition DOD-004 excludes.
Measured: nothing listens on 5173.

Attempts that did **not** resolve it:

- Removing `devUrl` from a `TAURI_CONFIG` overlay. The `localhost:5173` string is
  present in the binary from the **committed** `tauri.conf.json` regardless, and
  a normal release build contains it too — so its presence is not by itself
  evidence of which path is used at runtime.
- Forcing the tauri build script to re-run (`rm -rf target/release/build/linchpin-desktop-*`)
  after changing the overlay. No change.
- Confirmed the config overlay IS being read: with the overlay the debug port
  opens; with the committed config (and the same `devtools-e2e` feature) port
  9222 stays closed. So `additionalBrowserArgs` is honoured but the `build`
  section resolves differently than assumed.

Not yet established: whether `cargo build --release` without `tauri build` even
honours `frontendDist`, or whether the dev/prod selection depends on how the
binary was produced. That is the next thing to determine, and it is a question
about Tauri's build pipeline rather than about CDP.

### What is kept

- `devtools-e2e = ["tauri/devtools"]` in the desktop manifest. Verified: this is
  what makes the endpoint available at all, it is not a default feature, and a
  release build without the config overlay does not open the port.
- This record.

### What was reverted

The `tauri.e2e.conf.json` overlay and the CDP probe script, because neither
works yet and an unverified security-relevant build flag is not worth carrying.

## Not attempted

- Installing the artifact and driving the installed copy (`scripts/smoke-installed-artifact.sh`
  covers the install path separately).
- Producing the test artifact with `tauri build` rather than `cargo build`,
  which is the most likely reason `frontendDist` is not being applied.

## Honest status

DOD-004 remains **PARTIAL**. Two rounds have now been spent here without closing
it. The results are negative but not wasted: CDP attachment to the real webview
is proven to work (`hasIpc: true`), so the remaining problem is scoped to how the
test artifact is produced, not to how it is driven.

